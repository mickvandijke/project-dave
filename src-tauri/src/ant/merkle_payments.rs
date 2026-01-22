//! Merkle payment types and utilities for gas-efficient bulk uploads.
//!
//! Merkle payments use a single merkle tree transaction instead of individual
//! payments, significantly reducing gas costs for uploads of 64+ chunks.

use serde::{Deserialize, Serialize};

/// Minimum number of chunks required to use merkle payments.
/// Below this threshold, standard payments are more cost-effective.
/// NOTE: Set to 3 for testing. Production value should be 64.
pub const MERKLE_PAYMENT_THRESHOLD: usize = 3;

/// Result from the frontend after executing merkle payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerklePaymentResult {
    /// The winning pool hash returned by the contract
    pub winner_pool_hash: String,
    /// The actual amount paid
    pub amount_paid: String,
}

/// Determines whether to use merkle payments based on settings and chunk count.
pub fn should_use_merkle_payments(
    use_merkle_payments: Option<bool>,
    use_paymaster: Option<bool>,
    chunk_count: usize,
) -> bool {
    let merkle_enabled = use_merkle_payments.unwrap_or(false);
    let paymaster_enabled = use_paymaster.unwrap_or(false);
    let enough_chunks = chunk_count >= MERKLE_PAYMENT_THRESHOLD;

    merkle_enabled && !paymaster_enabled && enough_chunks
}
