use std::path::Path;

use autonomi::client::payment::Receipt;
use autonomi::client::quote::StoreQuote;
use tracing::{error, info};

use crate::ant::files::{get_payment_cache, File};
use crate::ant::receipt_utils::merge_receipts;

/// Handles receipt creation, merging, and caching operations.
pub struct ReceiptProcessor;

impl ReceiptProcessor {
    /// Creates a final receipt by merging a new receipt (from store quote) with an optional cached receipt.
    pub fn create_final_receipt(
        store_quote: StoreQuote,
        cached_receipt: Option<Receipt>,
    ) -> Receipt {
        let new_receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);

        if let Some(cached) = cached_receipt {
            info!("Merging new receipt with cached receipt");
            // Safe: vec always has 2 elements
            merge_receipts(vec![cached, new_receipt])
                .expect("merge_receipts with non-empty vec cannot fail")
        } else {
            new_receipt
        }
    }

    /// Caches a payment receipt for a single file.
    pub fn cache_file_receipt(file_path: &Path, receipt: &Receipt) {
        if let Ok(cache) = get_payment_cache() {
            if let Err(e) = cache.save_payment(file_path, receipt) {
                error!("Failed to cache payment receipt: {}", e);
            } else {
                info!("Successfully cached payment receipt for file: {:?}", file_path);
            }
        }
    }

    /// Caches a payment receipt for an archive.
    pub fn cache_archive_receipt(files: &[File], archive_name: &str, receipt: &Receipt) {
        if let Ok(cache) = get_payment_cache() {
            if let Err(e) = cache.save_archive_payment(files, archive_name, receipt) {
                error!("Failed to cache archive payment receipt: {}", e);
            } else {
                info!(
                    "Successfully cached archive payment receipt for: {}",
                    archive_name
                );
            }
        }
    }
}
