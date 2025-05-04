// src/monitoring/base_listener.rs

use anyhow::Result;
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub mint: Pubkey,
    pub symbol: String,
    pub name: String,
    pub description: String,
    pub creator: Pubkey,
    pub bonding_curve: Pubkey,
    pub created_timestamp: i64, // Unix timestamp
}

// To make TokenListener dyn-safe, the generic type F in listen_for_tokens
// needs to be replaced with a trait object like Box<dyn Fn(...)>.
#[async_trait]
pub trait TokenListener: Send + Sync { // Added Sync as listeners are often shared across threads
    async fn listen_for_tokens(
        &mut self, 
        callback: Box<dyn Fn(TokenInfo) + Send + Sync + 'static>,
        match_string: Option<String>, 
        bro_address: Option<String>
    ) -> Result<()>;

    async fn stop(&mut self) -> Result<()>;
}

