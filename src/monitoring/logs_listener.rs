// src/monitoring/logs_listener.rs

use crate::monitoring::base_listener::{TokenListener, TokenInfo};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use solana_client::pubsub_client::{PubsubClient, PubsubClientSubscription};
use solana_client::rpc_config::RpcTransactionLogsConfig;
use solana_client::rpc_config::RpcTransactionLogsFilter; // Corrected import path
use solana_client::rpc_response::{Response as RpcResponse, RpcLogsResponse};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::Arc;
use crossbeam_channel::Receiver; 
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct LogsListener {
    wss_url: String,
    program_id: Pubkey,
    subscription_handler: Arc<Mutex<Option<PubsubClientSubscription<RpcResponse<RpcLogsResponse>>>>>, 
    listener_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl LogsListener {
    pub fn new(wss_url: &str, program_id: Pubkey) -> Result<Self> {
        Ok(Self {
            wss_url: wss_url.to_string(),
            program_id,
            subscription_handler: Arc::new(Mutex::new(None)),
            listener_handle: Arc::new(Mutex::new(None)),
        })
    }

    fn parse_logs_for_token_info(logs_response_value: &RpcLogsResponse, program_id: &Pubkey) -> Option<TokenInfo> {
        let token_mint: Option<Pubkey> = None;
        let symbol: Option<String> = None;
        let name: Option<String> = None;
        let description: Option<String> = None;
        let creator: Option<Pubkey> = None;
        let bonding_curve: Option<Pubkey> = None;
        let created_timestamp = chrono::Utc::now().timestamp();

        for log_msg in &logs_response_value.logs {
            if log_msg.starts_with(&format!("Program {}", program_id)) {
                if log_msg.contains("Instruction: Create") {
                    log::debug!("Found Create instruction log: {}", log_msg);
                }
                if log_msg.contains("mint: ") { /* TODO: extract mint */ }
                if log_msg.contains("symbol: ") { /* TODO: extract symbol */ }
            }
        }
        
        if let (Some(mint_val), Some(symbol_val), Some(name_val), Some(desc_val), Some(creator_val), Some(bc_val)) = 
            (token_mint, symbol, name, description, creator, bonding_curve) {
            return Some(TokenInfo {
                mint: mint_val,
                symbol: symbol_val,
                name: name_val,
                description: desc_val,
                creator: creator_val,
                bonding_curve: bc_val,
                created_timestamp,
            });
        }
        None
    }
}

#[async_trait]
impl TokenListener for LogsListener {
    async fn listen_for_tokens(
        &mut self, 
        callback: Box<dyn Fn(TokenInfo) + Send + Sync + 'static>,
        match_string: Option<String>, 
        bro_address: Option<String>
    ) -> Result<()>
    {
        log::info!("Starting logsSubscribe listener for program: {}", self.program_id);
        let program_id_clone = self.program_id;
        let wss_url_clone = self.wss_url.clone();
        let sub_handler_arc = Arc::clone(&self.subscription_handler);
        let listener_handle_arc = Arc::clone(&self.listener_handle);

        let handle = tokio::spawn(async move {
            let rpc_logs_filter = RpcTransactionLogsFilter::Mentions(vec![program_id_clone.to_string()]);
            let rpc_transaction_logs_config = RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::confirmed()),
            };

            match PubsubClient::logs_subscribe(
                &wss_url_clone,
                rpc_logs_filter,
                rpc_transaction_logs_config,
            ) {
                Ok((subscription, mut message_receiver)) => { 
                    log::info!("Successfully subscribed to logs for program {}", program_id_clone);
                    
                    let mut sub_handler_guard = sub_handler_arc.lock().await;
                    *sub_handler_guard = Some(subscription);
                    drop(sub_handler_guard);

                    loop {
                        match message_receiver.recv() { 
                            Ok(response_message) => {
                                log::debug!("Received logs: slot={}, signature={}", response_message.context.slot, response_message.value.signature);
                                if let Some(token_info) = Self::parse_logs_for_token_info(&response_message.value, &program_id_clone) {
                                    let mut passes_filters = true;
                                    if let Some(ms) = &match_string {
                                        if !token_info.name.contains(ms) && !token_info.symbol.contains(ms) {
                                            passes_filters = false;
                                        }
                                    }
                                    if let Some(ba) = &bro_address {
                                        if token_info.creator.to_string() != *ba {
                                            passes_filters = false;
                                        }
                                    }

                                    if passes_filters {
                                        callback(token_info.clone());
                                    }
                                }
                            }
                            Err(_e) => { 
                                log::error!("Logs subscription channel error or disconnected: {}. Listener stopping.", _e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to subscribe to logs: {}", e);
                }
            }
            log::info!("Logs listener task finished.");
        });
        
        let mut handle_guard = listener_handle_arc.lock().await;
        *handle_guard = Some(handle);

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping LogsListener...");
        let mut sub_handler_guard = self.subscription_handler.lock().await;
        if let Some(_sub_handler) = sub_handler_guard.take() { 
            log::info!("Log subscription handler removed. Unsubscribe will be called on drop of PubsubClientSubscription.");
        }
        drop(sub_handler_guard);

        let mut handle_guard = self.listener_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            log::info!("Waiting for listener task to complete...");
            handle.await.map_err(|e| anyhow!("Listener task panicked: {:?}", e))?;
            log::info!("Listener task completed.");
        }
        Ok(())
    }
}

impl Drop for LogsListener {
    fn drop(&mut self) {
        log::warn!("LogsListener dropped. Ensure stop() was called for graceful async cleanup.");
    }
}

