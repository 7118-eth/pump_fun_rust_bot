// src/core/mod.rs

pub mod client;
pub mod wallet;
pub mod curve; // Placeholder for BondingCurveManager
pub mod priority_fee; // Placeholder for PriorityFeeManager
pub mod pubkeys; // Placeholder for PumpAddresses and other pubkeys

// Re-export key structs if needed
pub use client::SolanaClient;
pub use wallet::Wallet;

