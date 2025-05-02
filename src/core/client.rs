// src/core/client.rs

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_sdk::transaction::Transaction;
use std::time::Duration;
use anyhow::Result;
use solana_transaction_status::TransactionConfirmationStatus; // Import for clarity

// Removed #[derive(Debug)] because RpcClient doesn't implement Debug
pub struct SolanaClient {
    pub rpc_client: RpcClient,
}

impl SolanaClient {
    pub fn new(rpc_endpoint: &str) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(
                rpc_endpoint.to_string(),
                CommitmentConfig::confirmed(),
            ),
        }
    }

    pub fn new_with_timeout(rpc_endpoint: &str, timeout: Duration) -> Self {
        Self {
            rpc_client: RpcClient::new_with_timeout_and_commitment(
                rpc_endpoint.to_string(),
                timeout,
                CommitmentConfig::confirmed(),
            ),
        }
    }

    pub async fn get_health(&self) -> Result<String> {
        match self.rpc_client.get_slot() {
            Ok(slot) => Ok(format!("RPC Healthy. Current slot: {}", slot)),
            Err(e) => Err(anyhow::anyhow!("RPC Health Check Failed: {}", e)),
        }
    }

    pub async fn send_and_confirm_transaction(
        &self,
        transaction: &Transaction,
        _signers: &[&Keypair], // signers are used by the caller to sign before passing to this function
    ) -> Result<Signature> {
        let signature = self.rpc_client.send_transaction_with_config(
            transaction,
            RpcSendTransactionConfig { 
                skip_preflight: false,
                preflight_commitment: Some(CommitmentConfig::confirmed().commitment),
                ..Default::default()
            }
        )?;

        let mut confirmed = false;
        let mut attempts = 0;
        let max_attempts = 20; // Increased attempts for more robust confirmation (e.g. ~60 seconds)
        let mut last_status_log_time = std::time::Instant::now();

        while !confirmed && attempts < max_attempts {
            match self.rpc_client.get_signature_statuses(&[signature])? {
                response if !response.value.is_empty() => {
                    if let Some(Some(status)) = response.value.get(0) { // Check if status exists
                        let is_confirmed_or_finalized = matches!(
                            status.confirmation_status,
                            Some(TransactionConfirmationStatus::Confirmed) | 
                            Some(TransactionConfirmationStatus::Finalized)
                        );

                        if is_confirmed_or_finalized {
                            confirmed = true;
                            log::info!("Transaction {} confirmed with status: {:?}.", signature, status.confirmation_status.clone().unwrap());
                        } else {
                            if last_status_log_time.elapsed() >= Duration::from_secs(5) {
                                log::debug!(
                                    "Transaction {} not yet confirmed. Status: {:?}, Slot: {}. Attempt {}/{}", 
                                    signature, status.confirmation_status, status.slot, attempts + 1, max_attempts
                                );
                                last_status_log_time = std::time::Instant::now();
                            }
                        }
                    } else {
                        if last_status_log_time.elapsed() >= Duration::from_secs(5) {
                            log::debug!(
                                "Transaction {} status not yet available (empty status). Attempt {}/{}", 
                                signature, attempts + 1, max_attempts
                            );
                            last_status_log_time = std::time::Instant::now();
                        }
                    }
                }
                _ => { // Handles empty response.value or other unexpected cases
                    if last_status_log_time.elapsed() >= Duration::from_secs(5) {
                        log::debug!(
                            "Transaction {} status not yet available (no value in response). Attempt {}/{}", 
                            signature, attempts + 1, max_attempts
                        );
                        last_status_log_time = std::time::Instant::now();
                    }
                }
            }
            
            if !confirmed {
                tokio::time::sleep(Duration::from_secs(3)).await;
                attempts += 1;
            }
        }

        if confirmed {
            Ok(signature)
        } else {
            Err(anyhow::anyhow!("Transaction {} failed to confirm after {} attempts", signature, max_attempts))
        }
    }
}

