// src/trading/trader.rs

use crate::core::client::SolanaClient;
use crate::core::wallet::Wallet;
use crate::core::curve::BondingCurveManager;
use crate::core::priority_fee::PriorityFeeManager;
use crate::core::pubkeys::PumpAddresses;
use crate::trading::buyer::TokenBuyer;
use crate::trading::seller::TokenSeller;
use crate::utils::config_loader::BotConfig; // Assuming BotConfig will be passed or accessible
use crate::monitoring::base_listener::TokenInfo; // Assuming TokenInfo struct from listener
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use tokio::sync::mpsc; // For token queue if using async processing
use tokio::time::{sleep, Duration, Instant};

// Placeholder for PumpTrader
pub struct PumpTrader {
    // Core components
    solana_client: SolanaClient,
    wallet: Wallet,
    curve_manager: BondingCurveManager,
    priority_fee_manager: PriorityFeeManager,
    // Trading components
    buyer: TokenBuyer,
    seller: TokenSeller,
    // Configuration (simplified, might be part of a larger AppState or passed directly)
    config: BotConfig,
    // State
    traded_mints: HashSet<Pubkey>,
    token_queue: mpsc::Receiver<TokenInfo>, // Assuming tokens are pushed by a listener
    // token_timestamps: HashMap<Pubkey, Instant>, // To track token freshness
}

impl PumpTrader {
    pub async fn new(
        config: BotConfig,
        token_queue: mpsc::Receiver<TokenInfo>,
    ) -> Result<Self> {
        let solana_client = SolanaClient::new_with_timeout(&config.rpc_endpoint, Duration::from_secs(30));
        let wallet = Wallet::from_private_key(&config.private_key)?;
        
        // Clone client for managers if they need ownership, or pass references.
        // For simplicity, let's assume they can take ownership or are created with new clients if needed.
        // This part needs careful design based on how SolanaClient is structured (e.g. if it wraps an Arc<RpcClient>).
        // For now, let's assume we can create new instances or clone if cheap.
        let curve_manager = BondingCurveManager::new(SolanaClient::new(&config.rpc_endpoint));
        let priority_fee_manager = PriorityFeeManager::new(
            SolanaClient::new(&config.rpc_endpoint),
            config.priority_fees.enable_dynamic,
            config.priority_fees.enable_fixed,
            config.priority_fees.fixed_amount,
            config.priority_fees.extra_percentage,
            config.priority_fees.hard_cap,
        );

        let buyer = TokenBuyer::new(
            SolanaClient::new(&config.rpc_endpoint),
            Wallet::from_private_key(&config.private_key)?, // Buyer might need its own wallet instance or ref
            BondingCurveManager::new(SolanaClient::new(&config.rpc_endpoint)),
            PriorityFeeManager::new(
                SolanaClient::new(&config.rpc_endpoint),
                config.priority_fees.enable_dynamic, 
                config.priority_fees.enable_fixed, 
                config.priority_fees.fixed_amount, 
                config.priority_fees.extra_percentage, 
                config.priority_fees.hard_cap
            ),
            config.trade.buy_amount,
            (config.trade.buy_slippage * 100.0) as u16, // Assuming slippage is decimal, convert to bps
            config.retries.max_attempts,
        );

        let seller = TokenSeller::new(
            SolanaClient::new(&config.rpc_endpoint),
            Wallet::from_private_key(&config.private_key)?, // Seller might need its own wallet instance or ref
            BondingCurveManager::new(SolanaClient::new(&config.rpc_endpoint)),
            PriorityFeeManager::new(
                SolanaClient::new(&config.rpc_endpoint),
                config.priority_fees.enable_dynamic, 
                config.priority_fees.enable_fixed, 
                config.priority_fees.fixed_amount, 
                config.priority_fees.extra_percentage, 
                config.priority_fees.hard_cap
            ),
            (config.trade.sell_slippage * 100.0) as u16, // Assuming slippage is decimal, convert to bps
            config.retries.max_attempts,
        );

        Ok(Self {
            solana_client,
            wallet,
            curve_manager,
            priority_fee_manager,
            buyer,
            seller,
            config,
            traded_mints: HashSet::new(),
            token_queue,
            // token_timestamps: HashMap::new(),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        log::info!("PumpTrader starting...");
        log::info!("Match filter: {:?}", self.config.filters.match_string);
        log::info!("Creator filter: {:?}", self.config.filters.bro_address);
        log::info!("Marry mode: {}", self.config.filters.marry_mode);
        log::info!("YOLO mode: {}", self.config.filters.yolo_mode);
        log::info!("Max token age: {} seconds", self.config.timing.max_token_age);

        // Warm up RPC (optional, but good practice)
        match self.solana_client.get_health().await {
            Ok(health_msg) => log::info!("RPC Health: {}", health_msg),
            Err(e) => log::warn!("RPC Health Check failed: {}", e),
        }

        if !self.config.filters.yolo_mode {
            log::info!("Running in single token mode.");
            // Wait for one token from the queue with timeout
            match tokio::time::timeout(Duration::from_secs(self.config.timing.token_wait_timeout), self.token_queue.recv()).await {
                Ok(Some(token_info)) => {
                    log::info!("Received token for single mode: {:?}", token_info);
                    self.handle_token(token_info).await?;
                }
                Ok(None) => {
                    log::info!("Token queue closed while waiting for single token.");
                }
                Err(_) => {
                    log::info!("Timed out waiting for a token in single mode.");
                }
            }
        } else {
            log::info!("Running in YOLO (continuous) mode.");
            while let Some(token_info) = self.token_queue.recv().await {
                // TODO: Add freshness check (max_token_age)
                // let token_key = token_info.mint;
                // let current_time = Instant::now();
                // if let Some(timestamp) = self.token_timestamps.get(&token_key) {
                //     if current_time.duration_since(*timestamp).as_secs_f64() > self.config.timing.max_token_age {
                //         log::info!("Skipping token {} - too old.", token_key);
                //         continue;
                //     }
                // }

                if self.traded_mints.contains(&token_info.mint) {
                    log::debug!("Token {} already processed. Skipping...", token_info.mint);
                    continue;
                }
                self.handle_token(token_info).await?;
            }
            log::info!("Token queue closed. Exiting YOLO mode.");
        }
        
        self.cleanup_resources().await?;
        log::info!("PumpTrader shut down.");
        Ok(())
    }

    async fn handle_token(&mut self, token_info: TokenInfo) -> Result<()> {
        log::info!("Handling token: {} ({}) by creator {}", token_info.symbol, token_info.mint, token_info.creator);
        
        if self.traded_mints.contains(&token_info.mint) {
            log::info!("Token {} already handled. Skipping.", token_info.mint);
            return Ok(());
        }

        // Wait for bonding curve to stabilize (unless in extreme fast mode)
        if !self.config.trade.extreme_fast_mode {
            log::info!("Waiting {} seconds for bonding curve stabilization...", self.config.retries.wait_after_creation);
            sleep(Duration::from_secs(self.config.retries.wait_after_creation)).await;
        }

        // Buy token
        match self.buyer.buy_token(&token_info.mint, &token_info.creator).await {
            Ok(_) => {
                log::info!("Successfully bought token: {}", token_info.mint);
                self.traded_mints.insert(token_info.mint);

                if self.config.filters.marry_mode {
                    log::info!("Marry mode enabled. Not selling token {}.
", token_info.mint);
                    return Ok(());
                }

                // Wait after buy
                log::info!("Waiting {} seconds after buy...", self.config.retries.wait_after_buy);
                sleep(Duration::from_secs(self.config.retries.wait_after_buy)).await;

                // Sell token (placeholder: need token account and amount)
                // This requires fetching the token account for the mint owned by the wallet
                // and the balance of that account.
                let token_account_to_sell = Pubkey::new_unique(); // Placeholder
                let amount_to_sell = 0; // Placeholder
                
                log::warn!("Sell logic needs to fetch actual token account and balance.");
                match self.seller.sell_token(&token_info.mint, &token_account_to_sell, amount_to_sell).await {
                    Ok(_) => log::info!("Successfully sold token: {}", token_info.mint),
                    Err(e) => log::error!("Failed to sell token {}: {}", token_info.mint, e),
                }
            }
            Err(e) => {
                log::error!("Failed to buy token {}: {}", token_info.mint, e);
                // Handle cleanup after failure if configured
            }
        }
        
        // Wait before processing next token if in YOLO mode
        if self.config.filters.yolo_mode {
            log::info!("Waiting {} seconds before next token...", self.config.retries.wait_before_new_token);
            sleep(Duration::from_secs(self.config.retries.wait_before_new_token)).await;
        }

        Ok(())
    }

    async fn cleanup_resources(&mut self) -> Result<()> {
        log::info!("Cleaning up resources...");
        // Implement cleanup logic similar to Python version (e.g., closing accounts)
        // This would use the cleanup module and configuration.
        if !self.traded_mints.is_empty() && self.config.cleanup.mode != "disabled" {
            log::info!("Performing post-session cleanup for {} traded mints.", self.traded_mints.len());
            // Call cleanup manager here
        }
        // self.solana_client.close().await?; // RpcClient doesn't need explicit close for HTTP
        Ok(())
    }
}

