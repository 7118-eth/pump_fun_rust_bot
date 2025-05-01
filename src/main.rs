// src/main.rs

use anyhow::Result;
use pump_fun_rust_bot::utils::config_loader::{load_bot_config, print_config_summary, BotConfig};
use pump_fun_rust_bot::utils::logger::{setup_logger, setup_file_logging};
use pump_fun_rust_bot::monitoring::TokenListener; // Trait
use pump_fun_rust_bot::monitoring::{LogsListener, GeyserListener, BlockListener, TokenInfo};
use pump_fun_rust_bot::trading::PumpTrader;
use pump_fun_rust_bot::core::pubkeys::pump_fun_program_id;
use std::path::Path;
use tokio::sync::mpsc;
use tokio::signal;
use log::LevelFilter;

async fn run_single_bot(config_path: String) -> Result<()> {
    let config = load_bot_config(&config_path)?;

    // Setup logging
    // Determine log file path similar to Python version
    let log_dir = Path::new("logs");
    if !log_dir.exists() {
        std::fs::create_dir_all(log_dir)?;
    }
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let log_filename = log_dir.join(format!("{}_{}.log", config.name, timestamp));
    
    // Using a simple console logger for now, file logging can be more complex.
    // The setup_file_logging in the lib is a placeholder.
    // For actual file logging, a crate like `fern` or `log4rs` would be better.
    // setup_file_logging(log_filename.to_str().unwrap_or("bot.log"), LevelFilter::Info)?;
    setup_logger(LevelFilter::Info)?; // Default to console Info level logger
    log::info!("Logging initialized. Log file (conceptual): {}", log_filename.display());

    print_config_summary(&config);

    if !config.enabled {
        log::info!("Bot 	{} is disabled in config. Exiting.", config.name);
        return Ok(());
    }

    let (token_sender, token_receiver) = mpsc::channel::<TokenInfo>(100); // Buffer of 100 tokens

    let mut trader = PumpTrader::new(config.clone(), token_receiver).await?;

    // Choose and start listener based on config
    let mut listener: Box<dyn TokenListener + Send> = match config.filters.listener_type.to_lowercase().as_str() {
        "logs" => Box::new(LogsListener::new(&config.wss_endpoint, pump_fun_program_id())?),
        "geyser" => {
            if let Some(geyser_config) = &config.geyser {
                Box::new(GeyserListener::new(
                    geyser_config.endpoint.as_deref().unwrap_or_default(),
                    geyser_config.api_token.clone(),
                    &geyser_config.auth_type,
                    pump_fun_program_id(),
                )?)
            } else {
                return Err(anyhow::anyhow!("Geyser listener type selected, but no Geyser configuration provided."));
            }
        }
        "block" | "blocks" => Box::new(BlockListener::new(&config.wss_endpoint, pump_fun_program_id())?),
        _ => return Err(anyhow::anyhow!("Unsupported listener type: {}", config.filters.listener_type)),
    };

    log::info!("Starting listener type: {}", config.filters.listener_type);
    let listener_match_string = config.filters.match_string.clone();
    let listener_bro_address = config.filters.bro_address.clone();

    // Spawn listener in a separate task
    let listener_task = tokio::spawn(async move {
        // The callback needs to be Send + Sync + 'static
        let callback = move |token_info: TokenInfo| {
            log::debug!("Listener callback received token: {:?}", token_info.symbol);
            if let Err(e) = token_sender.try_send(token_info) { // Use try_send for non-blocking or handle blocking case
                log::error!("Failed to send token to trader: {}", e);
            }
        };
        if let Err(e) = listener.listen_for_tokens(Box::new(callback), listener_match_string, listener_bro_address).await {
            log::error!("Token listener error: {}", e);
        }
        // listener.stop().await.unwrap_or_else(|e| log::error!("Error stopping listener: {}", e));
        log::info!("Listener task finished.");
    });

    // Start trader
    let trader_task = tokio::spawn(async move {
        if let Err(e) = trader.start().await {
            log::error!("Trader error: {}", e);
        }
        log::info!("Trader task finished.");
    });

    // Wait for tasks or shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            log::info!("Ctrl+C received. Shutting down...");
        }
        _ = listener_task => {
            log::info!("Listener task completed unexpectedly.");
        }
        _ = trader_task => {
            log::info!("Trader task completed unexpectedly.");
        }
    }
    
    // TODO: Implement graceful shutdown for listener and trader if they have stop methods that need awaiting.
    // For now, tasks will be aborted when main exits or select! branch completes.
    log::info!("Bot shutdown complete.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // TODO: Implement proper CLI argument parsing for config path
    // For now, hardcode a default path or expect it to be in a specific location.
    // The Python bot looks for YAML files in a "bots" directory.
    // Let's assume a single config file path for this example.
    let default_config_path = "bots/bot-sniper-1-geyser.yaml".to_string(); // Example path
    
    // Check if the default config file exists
    if !Path::new(&default_config_path).exists() {
        log::error!("Default config file not found at: {}. Please create it or specify a path.", default_config_path);
        // Create a dummy bots directory and an example config file if it doesn't exist
        // to prevent immediate failure on first run for the user.
        std::fs::create_dir_all("bots")?;
        let example_config_content = r#"
name: "MyRustBot"
enabled: true
separate_process: false # Not used in this simplified Rust runner
rpc_endpoint: "https://api.mainnet-beta.solana.com"
wss_endpoint: "wss://api.mainnet-beta.solana.com"
private_key: "YOUR_PRIVATE_KEY_BS58_HERE_DO_NOT_COMMIT_REAL_KEYS"

trade:
  buy_amount: 0.001 # SOL
  buy_slippage: 0.25 # 25%
  sell_slippage: 0.25 # 25%
  extreme_fast_mode: false
  extreme_fast_token_amount: 30

filters:
  listener_type: "logs" # or "geyser", "block"
  match_string: null # Optional: e.g., "PEPE"
  bro_address: null # Optional: Creator address to filter by
  marry_mode: false
  yolo_mode: true # Continuous trading

# geyser: # Uncomment and configure if using geyser listener_type
#   endpoint: "your_geyser_endpoint_here"
#   api_token: "your_geyser_api_token_here"
#   auth_type: "x-token" # or "basic"

priority_fees:
  enable_dynamic: false
  enable_fixed: true
  fixed_amount: 100000 # lamports, e.g., 0.0001 SOL
  extra_percentage: 0.0
  hard_cap: 500000

retries:
  max_attempts: 5
  wait_after_creation: 10 # seconds
  wait_after_buy: 10 # seconds
  wait_before_new_token: 5 # seconds

timing:
  max_token_age: 0.5 # seconds, how old a token can be when picked up by trader
  token_wait_timeout: 60 # seconds, for single token mode

cleanup:
  mode: "disabled" # "auto" or "manual"
  force_close_with_burn: false
  with_priority_fee: false
"#;
        std::fs::write(&default_config_path, example_config_content)?;
        log::info!("Created an example config file at: {}. Please edit it with your details, especially the private_key.", default_config_path);
        return Err(anyhow::anyhow!("Default config file was missing. An example has been created. Please configure and restart."));
    }

    // In a real app, use clap or similar for args.
    let args: Vec<String> = std::env::args().collect();
    let config_path = args.get(1).cloned().unwrap_or(default_config_path);

    log::info!("Using config file: {}", config_path);
    run_single_bot(config_path).await
}

