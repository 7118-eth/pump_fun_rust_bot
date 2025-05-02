// src/utils/config_loader.rs

use serde::{Deserialize, Serialize};
use serde_yaml;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeConfig {
    pub buy_amount: f64,
    pub buy_slippage: f64,
    pub sell_slippage: f64,
    #[serde(default)]
    pub extreme_fast_mode: bool,
    #[serde(default = "default_extreme_fast_token_amount")]
    pub extreme_fast_token_amount: u32,
}

fn default_extreme_fast_token_amount() -> u32 {
    30
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FiltersConfig {
    pub listener_type: String, // e.g., "logs", "geyser", "block"
    pub match_string: Option<String>,
    pub bro_address: Option<String>,
    #[serde(default)]
    pub marry_mode: bool,
    #[serde(default)]
    pub yolo_mode: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeyserConfig {
    pub endpoint: Option<String>,
    pub api_token: Option<String>,
    #[serde(default = "default_geyser_auth_type")]
    pub auth_type: String,
}

fn default_geyser_auth_type() -> String {
    "x-token".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriorityFeesConfig {
    #[serde(default)]
    pub enable_dynamic: bool,
    #[serde(default = "default_true")]
    pub enable_fixed: bool,
    #[serde(default = "default_fixed_priority_fee")]
    pub fixed_amount: u64, // lamports
    #[serde(default)]
    pub extra_percentage: f64,
    #[serde(default = "default_hard_cap_priority_fee")]
    pub hard_cap: u64, // lamports
}

fn default_true() -> bool {
    true
}

fn default_fixed_priority_fee() -> u64 {
    500_000
}

fn default_hard_cap_priority_fee() -> u64 {
    500_000
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetriesConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_wait_time")]
    pub wait_after_creation: u64, // seconds
    #[serde(default = "default_wait_time")]
    pub wait_after_buy: u64, // seconds
    #[serde(default = "default_wait_time")]
    pub wait_before_new_token: u64, // seconds
}

fn default_max_attempts() -> u32 {
    10
}

fn default_wait_time() -> u64 {
    15
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimingConfig {
    #[serde(default = "default_max_token_age")]
    pub max_token_age: f64, // seconds
    #[serde(default = "default_token_wait_timeout")]
    pub token_wait_timeout: u64, // seconds
}

fn default_max_token_age() -> f64 {
    0.001
}

fn default_token_wait_timeout() -> u64 {
    30
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CleanupConfig {
    #[serde(default = "default_cleanup_mode")]
    pub mode: String, // "disabled", "auto", "manual"
    #[serde(default)]
    pub force_close_with_burn: bool,
    #[serde(default)]
    pub with_priority_fee: bool,
}

fn default_cleanup_mode() -> String {
    "disabled".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BotConfig {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub separate_process: bool, // Note: process separation needs careful handling in Rust (e.g. using threads or async tasks)
    pub rpc_endpoint: String,
    pub wss_endpoint: String,
    pub private_key: String,
    pub trade: TradeConfig,
    pub filters: FiltersConfig,
    #[serde(default)]
    pub geyser: Option<GeyserConfig>,
    #[serde(default)]
    pub priority_fees: PriorityFeesConfig,
    #[serde(default)]
    pub retries: RetriesConfig,
    #[serde(default)]
    pub timing: TimingConfig,
    #[serde(default)]
    pub cleanup: CleanupConfig,
}

impl Default for PriorityFeesConfig {
    fn default() -> Self {
        Self {
            enable_dynamic: false,
            enable_fixed: true,
            fixed_amount: default_fixed_priority_fee(),
            extra_percentage: 0.0,
            hard_cap: default_hard_cap_priority_fee(),
        }
    }
}

impl Default for RetriesConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            wait_after_creation: default_wait_time(),
            wait_after_buy: default_wait_time(),
            wait_before_new_token: default_wait_time(),
        }
    }
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            max_token_age: default_max_token_age(),
            token_wait_timeout: default_token_wait_timeout(),
        }
    }
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            mode: default_cleanup_mode(),
            force_close_with_burn: false,
            with_priority_fee: false,
        }
    }
}

pub fn load_bot_config(config_path: &str) -> Result<BotConfig, anyhow::Error> {
    let path = Path::new(config_path);
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: BotConfig = serde_yaml::from_str(&contents)?;
    Ok(config)
}

pub fn print_config_summary(config: &BotConfig) {
    log::info!("Loaded Bot Configuration: {}", config.name);
    log::info!("  Enabled: {}", config.enabled);
    log::info!("  RPC Endpoint: {}", config.rpc_endpoint);
    log::info!("  WSS Endpoint: {}", config.wss_endpoint);
    log::info!("  Trade Config: {:?}", config.trade);
    log::info!("  Filters Config: {:?}", config.filters);
    if let Some(geyser_conf) = &config.geyser {
        log::info!("  Geyser Config: {:?}", geyser_conf);
    }
    log::info!("  Priority Fees Config: {:?}", config.priority_fees);
    log::info!("  Retries Config: {:?}", config.retries);
    log::info!("  Timing Config: {:?}", config.timing);
    log::info!("  Cleanup Config: {:?}", config.cleanup);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_config() {
        // Create a dummy config file
        let dummy_config_content = r#"
name: "TestBot"
enabled: true
rpc_endpoint: "http://localhost:8899"
wss_endpoint: "ws://localhost:8900"
private_key: "your_private_key_here"
trade:
  buy_amount: 0.01
  buy_slippage: 0.25
  sell_slippage: 0.25
filters:
  listener_type: "logs"
"#;
        let config_path = "/tmp/test_bot_config.yaml";
        let mut file = File::create(config_path).unwrap();
        file.write_all(dummy_config_content.as_bytes()).unwrap();

        let config = load_bot_config(config_path).unwrap();
        assert_eq!(config.name, "TestBot");
        assert_eq!(config.trade.buy_amount, 0.01);
        assert_eq!(config.filters.listener_type, "logs");
        assert!(config.priority_fees.enable_fixed); // Check default

        // Clean up dummy file
        std::fs::remove_file(config_path).unwrap();
    }
}

