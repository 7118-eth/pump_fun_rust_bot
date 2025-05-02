// src/core/wallet.rs

use solana_sdk::signer::keypair::{keypair_from_seed, Keypair};
use solana_sdk::signer::Signer;
use bs58;
use anyhow::{Result, anyhow};

#[derive(Debug)]
pub struct Wallet {
    pub keypair: Keypair,
}

impl Wallet {
    pub fn from_private_key(private_key_bs58: &str) -> Result<Self> {
        let decoded_key = bs58::decode(private_key_bs58)
            .into_vec()
            .map_err(|e| anyhow!("Failed to decode private key from base58: {}", e))?;

        // Assuming the private key is a 64-byte array for ed25519 keypair
        // or a 32-byte seed. The Python bot likely uses a full keypair string or a seed.
        // If it's a full keypair (seed + public key), we only need the seed part (first 32 bytes).
        // If it's just a seed, it should be 32 bytes.
        // Solana's Keypair::from_bytes expects a 64-byte array [seed, public_key]
        // Keypair::from_seed expects a 32-byte seed.

        if decoded_key.len() == 64 { // Full keypair bytes
            let keypair = Keypair::from_bytes(&decoded_key)
                .map_err(|e| anyhow!("Failed to create keypair from 64 bytes: {}", e))?;
            Ok(Self { keypair })
        } else if decoded_key.len() == 32 { // Seed bytes
            let keypair = keypair_from_seed(&decoded_key)
                .map_err(|e| anyhow!("Failed to create keypair from 32-byte seed: {}", e))?;
            Ok(Self { keypair })
        } else {
            Err(anyhow!(
                "Private key (decoded) has invalid length: {}. Expected 32 or 64 bytes.",
                decoded_key.len()
            ))
        }
    }

    pub fn public_key(&self) -> solana_sdk::pubkey::Pubkey {
        self.keypair.pubkey()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_from_private_key() {
        // Generate a new keypair for testing
        let test_keypair = Keypair::new();
        let private_key_bytes = test_keypair.to_bytes();
        let private_key_bs58 = bs58::encode(&private_key_bytes[0..32]).into_string(); // Use seed part for this test

        let wallet = Wallet::from_private_key(&private_key_bs58).unwrap();
        assert_eq!(wallet.public_key(), test_keypair.pubkey());

        // Test with a full 64-byte keypair representation (less common for direct input but good to check)
        let full_keypair_bs58 = bs58::encode(&private_key_bytes).into_string();
        let wallet_from_full = Wallet::from_private_key(&full_keypair_bs58);
        // This might fail if bs58 decoding of 64 bytes isn't what from_bytes expects directly
        // The common way is to provide the 32-byte seed as bs58 string.
        // For now, we'll focus on the seed-based creation as it's more typical for .env files.
        assert!(wallet_from_full.is_ok());
        assert_eq!(wallet_from_full.unwrap().public_key(), test_keypair.pubkey());


        let invalid_key = "invalid_bs58_key_obviously_too_short";
        let result = Wallet::from_private_key(invalid_key);
        assert!(result.is_err());
    }
}

