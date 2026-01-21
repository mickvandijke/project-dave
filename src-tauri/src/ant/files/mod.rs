//! File operations module for uploading, downloading, and managing files on the Autonomi network.

mod errors;
mod types;

// Re-export types and errors for backward compatibility
pub use errors::{DownloadError, UploadError, VaultError};
pub use types::{
    ArchiveInfo, FailedArchive, File, FileAccess, FileFromVault, FileMetadata, FileType,
    LoadingArchive, UploadProgress, VaultStructure, VaultUpdate, VaultUpdateType,
};

use crate::ant::app_data;
use crate::ant::cached_payments::PaymentCache;
use crate::ant::client::SharedClient;
use crate::ant::encryption::encrypt_file_or_folder;
use crate::ant::quote::combine_quotes;
use crate::ant::receipt_utils::validate_receipt_coverage_with_content_addresses;
use crate::ant::stream::content_addresses_from_encryption_stream;
use crate::ant::upload::batch_upload_encryption_stream;
use crate::ant::{local_storage, vault};
use autonomi::chunk::DataMapChunk;
use autonomi::client::payment::{PaymentOption, Receipt};
use autonomi::client::quote::DataTypes;
use autonomi::client::vault::key::vault_key_from_signature_hex;
use autonomi::client::vault::{UserData, VaultSecretKey};
use autonomi::data::DataAddress;
use autonomi::files::{Metadata, PrivateArchive, PublicArchive};
use autonomi::vault::user_data::UserDataVaultError;
use autonomi::{Amount, Bytes, XorName};
use serde_json;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, State};
use tokio::fs;
use tracing::{debug, error, info, warn};

static PAYMENT_CACHE: OnceLock<Result<PaymentCache, String>> = OnceLock::new();

pub fn get_payment_cache() -> Result<&'static PaymentCache, &'static str> {
    PAYMENT_CACHE
        .get_or_init(|| {
            app_data::data_dir()
                .ok_or_else(|| "Could not get app data directory".to_string())
                .and_then(|dir| {
                    PaymentCache::new(&dir)
                        .map_err(|e| format!("Failed to create payment cache: {}", e))
                })
        })
        .as_ref()
        .map_err(|_| "Payment cache not available")
}

// ============================================================================
// Helper Functions - Phase 1
// ============================================================================

use autonomi::client::quote::StoreQuote;
use autonomi::Client;
use super::payments::Payment;

/// Emits an upload-quote event with the given parameters.
/// Consolidates the 8 identical quote emission patterns across upload functions.
fn emit_upload_quote(
    app: &AppHandle,
    upload_id: &str,
    total_files: usize,
    total_size: u64,
    total_cost: Amount,
    payments: &[serde_json::Value],
    raw_payments: &[Payment],
) -> Result<(), UploadError> {
    let has_payments = total_cost > Amount::ZERO;
    app.emit(
        "upload-quote",
        serde_json::json!({
            "upload_id": upload_id,
            "total_files": total_files,
            "total_size": total_size,
            "total_cost_nano": total_cost.to_string(),
            "total_cost_formatted": format!("{} ATTO", total_cost),
            "payment_required": has_payments,
            "payments": payments,
            "raw_payments": raw_payments
        }),
    )
    .map_err(|err| {
        error!("Failed to emit upload-quote event: {}", err);
        UploadError::EmitEvent(err.to_string())
    })
}

/// Emits an upload-quote event with zero cost (for cached receipts).
fn emit_zero_cost_quote(
    app: &AppHandle,
    upload_id: &str,
    total_files: usize,
    total_size: u64,
) -> Result<(), UploadError> {
    app.emit(
        "upload-quote",
        serde_json::json!({
            "upload_id": upload_id,
            "total_files": total_files,
            "total_size": total_size,
            "total_cost_nano": "0",
            "total_cost_formatted": "0 ATTO",
            "payment_required": false,
            "payments": Vec::<serde_json::Value>::new(),
            "raw_payments": Vec::<serde_json::Value>::new()
        }),
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))
}

/// Builds payment JSON array from store quote for emission.
fn build_payments_json(store_quote: &StoreQuote) -> Vec<serde_json::Value> {
    let total_cost: Amount = store_quote.payments().iter().map(|(_, _, amount)| *amount).sum();
    if total_cost > Amount::ZERO {
        store_quote
            .payments()
            .iter()
            .map(|(addr, _, amount)| {
                serde_json::json!({
                    "address": hex::encode(addr),
                    "amount": amount.to_string(),
                    "amount_formatted": format!("{} ATTO", amount)
                })
            })
            .collect()
    } else {
        vec![]
    }
}

/// Gets raw payments from store quote filtered by non-zero amounts.
fn get_raw_payments(store_quote: &StoreQuote) -> Vec<Payment> {
    store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect()
}

/// Retrieves user data from vault, creating new if vault doesn't exist yet.
/// Consolidates the error handling pattern used in add_local_*_to_vault functions.
async fn get_or_create_user_data(
    client: &Client,
    secret_key: &VaultSecretKey,
) -> Result<UserData, VaultError> {
    match client.vault_get_user_data(secret_key).await {
        Ok(data) => {
            debug!("Successfully retrieved user data from vault");
            Ok(data)
        }
        Err(e) => {
            debug!("Failed to get user data from vault: {:?}", e);
            match &e {
                UserDataVaultError::GetError(_) | UserDataVaultError::Vault(_) => {
                    debug!("Vault might not exist yet, creating new user data");
                    Ok(UserData::new())
                }
                _ => {
                    error!("Other vault error, returning error");
                    Err(VaultError::UserDataGet(e))
                }
            }
        }
    }
}

// ============================================================================
// Helper Functions - Phase 2
// ============================================================================

/// Result of checking cached payment for a file or archive.
struct CachedPaymentResult {
    /// The cached receipt if found (partial or complete).
    cached_receipt: Option<Receipt>,
    /// True if the cached receipt is partial and needs additional payment.
    need_additional_payment: bool,
    /// XorName addresses of chunks missing from the cached receipt.
    missing_chunks: Vec<Vec<u8>>,
    /// True if the cached receipt fully covers all chunks.
    is_fully_cached: bool,
}

/// Checks for cached payment for a file and validates its coverage.
fn check_cached_payment_for_file(
    cache: &PaymentCache,
    file_path: &std::path::Path,
    content_addresses: &[XorName],
) -> CachedPaymentResult {
    if let Ok(Some(cached_receipt)) = cache.load_payment_for_file(file_path) {
        info!("Found cached payment, validating coverage...");
        let validation =
            validate_receipt_coverage_with_content_addresses(&cached_receipt, content_addresses);

        if validation.is_complete {
            CachedPaymentResult {
                cached_receipt: Some(cached_receipt),
                need_additional_payment: false,
                missing_chunks: vec![],
                is_fully_cached: true,
            }
        } else {
            info!(
                "Cached receipt is partial, missing {} chunks",
                validation.missing_chunks.len()
            );
            CachedPaymentResult {
                cached_receipt: Some(cached_receipt),
                need_additional_payment: true,
                missing_chunks: validation.missing_chunks,
                is_fully_cached: false,
            }
        }
    } else {
        CachedPaymentResult {
            cached_receipt: None,
            need_additional_payment: false,
            missing_chunks: vec![],
            is_fully_cached: false,
        }
    }
}

/// Checks for cached payment for an archive and validates its coverage.
fn check_cached_payment_for_archive(
    cache: &PaymentCache,
    files: &[File],
    archive_name: &str,
    content_addresses: &[XorName],
) -> CachedPaymentResult {
    if let Ok(Some(cached_receipt)) = cache.load_archive_payment(files, archive_name) {
        info!("Found cached payment, validating coverage...");
        let validation =
            validate_receipt_coverage_with_content_addresses(&cached_receipt, content_addresses);

        if validation.is_complete {
            CachedPaymentResult {
                cached_receipt: Some(cached_receipt),
                need_additional_payment: false,
                missing_chunks: vec![],
                is_fully_cached: true,
            }
        } else {
            info!(
                "Cached receipt is partial, missing {} chunks",
                validation.missing_chunks.len()
            );
            CachedPaymentResult {
                cached_receipt: Some(cached_receipt),
                need_additional_payment: true,
                missing_chunks: validation.missing_chunks,
                is_fully_cached: false,
            }
        }
    } else {
        CachedPaymentResult {
            cached_receipt: None,
            need_additional_payment: false,
            missing_chunks: vec![],
            is_fully_cached: false,
        }
    }
}

/// Fetches store quotes for chunks, either partial (missing only) or full.
async fn fetch_store_quotes(
    client: &Client,
    all_content_addresses: Vec<(XorName, usize)>,
    missing_chunks: Option<&[Vec<u8>]>,
) -> Result<StoreQuote, UploadError> {
    match missing_chunks {
        Some(missing) if !missing.is_empty() => {
            info!("Getting store quotes for {} missing chunks...", missing.len());
            let missing_chunks_iter = all_content_addresses
                .iter()
                .filter(|(name, _)| missing.contains(&name.to_vec()))
                .cloned();

            client
                .get_store_quotes(DataTypes::Chunk, missing_chunks_iter)
                .await
                .map_err(|err| {
                    error!("Failed to get store quotes: {}", err);
                    UploadError::StoreQuote(err.to_string())
                })
        }
        _ => {
            info!(
                "Getting store quotes for {} chunks...",
                all_content_addresses.len()
            );
            client
                .get_store_quotes(DataTypes::Chunk, all_content_addresses.into_iter())
                .await
                .map_err(|err| {
                    error!("Failed to get store quotes: {}", err);
                    UploadError::StoreQuote(err.to_string())
                })
        }
    }
}

/// Result of preparing a vault update.
struct VaultUpdatePrep {
    /// The vault quote merged with the provided store quote.
    merged_quote: StoreQuote,
    /// The vault update metadata for later use.
    vault_update: vault::VaultUpdate,
}

/// Enum to specify what item is being added to the vault.
enum VaultItem {
    PrivateFile {
        data_map_chunk: DataMapChunk,
        name: String,
    },
    PublicFile {
        data_address: DataAddress,
        name: String,
    },
    PrivateArchive {
        data_map_chunk: DataMapChunk,
        name: String,
    },
    PublicArchive {
        data_address: DataAddress,
        name: String,
    },
}

/// Prepares a vault update by getting vault quote and merging with store quote.
async fn prepare_vault_update(
    client: &Client,
    secret_key: &VaultSecretKey,
    store_quote: StoreQuote,
    item: VaultItem,
) -> Result<VaultUpdatePrep, UploadError> {
    debug!("Getting vault quote for add_to_vault...");

    let mut user_data = client
        .vault_get_user_data(secret_key)
        .await
        .unwrap_or(UserData::new());

    // Add the item to user data based on type
    match &item {
        VaultItem::PrivateFile { data_map_chunk, name } => {
            user_data.private_files.insert(data_map_chunk.clone(), name.clone());
        }
        VaultItem::PublicFile { data_address, name } => {
            user_data.public_files.insert(*data_address, name.clone());
        }
        VaultItem::PrivateArchive { data_map_chunk, name } => {
            user_data.private_file_archives.insert(data_map_chunk.clone(), name.clone());
        }
        VaultItem::PublicArchive { data_address, name } => {
            user_data.file_archives.insert(*data_address, name.clone());
        }
    }

    let vault_data = user_data
        .to_bytes()
        .map_err(|e| UploadError::Serialization(e.to_string()))?;

    let vault_quote_result = vault::vault_quote(client, vault_data, secret_key)
        .await
        .map_err(|e| UploadError::StoreQuote(e.to_string()))?;

    info!("Got vault quote, merging with store quote");

    let merged_quote = combine_quotes(vec![store_quote, vault_quote_result.quote]);

    Ok(VaultUpdatePrep {
        merged_quote,
        vault_update: vault::VaultUpdate {
            new_graph_entries: vault_quote_result.new_graph_entries,
            new_scratchpad_derivations: vault_quote_result.new_scratchpad_derivations,
        },
    })
}

// ============================================================================
// Helper Functions - Phase 3
// ============================================================================

/// Emits an upload progress event.
fn emit_upload_progress(app: &AppHandle, progress: UploadProgress) -> Result<(), UploadError> {
    app.emit("upload-progress", progress)
        .map_err(|err| UploadError::EmitEvent(err.to_string()))
}

#[allow(dead_code)]
pub async fn read_file_to_bytes(file_path: PathBuf) -> Result<Bytes, UploadError> {
    tokio::fs::read(file_path.clone())
        .await
        .map(Bytes::from)
        .map_err(|_| UploadError::Read(file_path))
}

pub async fn calculate_total_size(files: &[File]) -> Result<u64, UploadError> {
    let mut total_size = 0u64;

    for file in files {
        let metadata = fs::metadata(&file.path)
            .await
            .map_err(|_| UploadError::Read(file.path.clone()))?;

        if metadata.is_dir() {
            // Calculate directory size
            let collected_files = collect_files_from_directory(file.path.clone()).await?;
            for (_, absolute_path) in collected_files {
                let file_metadata = fs::metadata(&absolute_path)
                    .await
                    .map_err(|_| UploadError::Read(absolute_path))?;
                total_size += file_metadata.len();
            }
        } else {
            total_size += metadata.len();
        }
    }

    Ok(total_size)
}

/// Collects files from a directory and its subdirectories, preserving relative paths.
/// Returns a vector of tuples containing (relative_path, absolute_path).
pub async fn collect_files_from_directory(
    dir_path: PathBuf,
) -> Result<Vec<(PathBuf, PathBuf)>, UploadError> {
    let mut files = Vec::new();
    let mut queue = VecDeque::new();

    // Get the parent directory to calculate relative paths correctly
    let base_dir = dir_path.parent().unwrap_or(&dir_path);
    let _dir_name = dir_path.file_name().unwrap_or_default();

    // Start with the root directory
    queue.push_back(dir_path.clone());

    while let Some(current_dir) = queue.pop_front() {
        let mut entries = fs::read_dir(&current_dir)
            .await
            .map_err(|_| UploadError::Read(current_dir.clone()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|_| UploadError::Read(current_dir.clone()))?
        {
            let path = entry.path();

            if path.is_dir() {
                // Add directory to the queue for processing
                queue.push_back(path);
            } else {
                // Calculate relative path including the root folder name
                if let Ok(rel_path) = path.strip_prefix(base_dir) {
                    files.push((rel_path.to_path_buf(), path));
                } else {
                    // Fallback if strip_prefix fails
                    files.push((path.clone(), path));
                }
            }
        }
    }

    Ok(files)
}

pub async fn start_private_single_file_upload(
    app: AppHandle,
    file: File,
    vault_secret_key: Option<&VaultSecretKey>,
    upload_id: String,
    add_to_vault: bool,
    use_cached_receipts: bool,
    shared_client: State<'_, SharedClient>,
    pending_uploads: Option<&tokio::sync::Mutex<crate::PendingUploads>>,
) -> Result<(), UploadError> {
    debug!(
        upload_id = %upload_id,
        use_cached_receipts = %use_cached_receipts,
        "start_single_file_upload called"
    );
    let client = shared_client.get_client().await.map_err(|e| {
        error!("Failed to get client: {:?}", e);
        e
    })?;

    // Calculate file size
    let file_size = fs::metadata(&file.path)
        .await
        .map_err(|_| UploadError::Read(file.path.clone()))?
        .len();

    // Create encryption stream for the file
    info!("Creating encryption stream for file: {:?}", file.path);
    let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), false)
        .await
        .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

    let stream = encryption_streams
        .first_mut()
        .ok_or(UploadError::Encryption("Expected one stream".to_string()))?;

    let all_content_addresses = content_addresses_from_encryption_stream(stream).await;

    let data_map_chunk = stream.data_map_chunk().ok_or(UploadError::Encryption(
        "Missing data map chunk".to_string(),
    ))?;

    info!("Got encryption stream with {} chunks", all_content_addresses.len());

    // Check for cached payment first (only if user wants to use cached receipts)
    let mut cached_result = CachedPaymentResult {
        cached_receipt: None,
        need_additional_payment: false,
        missing_chunks: vec![],
        is_fully_cached: false,
    };

    if use_cached_receipts {
        if let Ok(cache) = get_payment_cache() {
            info!("Checking for cached payment for file: {:?}", file.path);
            let content_addresses: Vec<XorName> = all_content_addresses
                .iter()
                .map(|(address, _)| *address)
                .collect();

            cached_result = check_cached_payment_for_file(cache, &file.path, &content_addresses);

            if cached_result.is_fully_cached {
                info!("Cached receipt covers all chunks, reusing it for upload");
                emit_zero_cost_quote(&app, &upload_id, 1, file_size)?;

                return execute_private_single_file_upload(
                    app,
                    file,
                    data_map_chunk,
                    cached_result.cached_receipt.unwrap(),
                    Default::default(),
                    vault_secret_key,
                    upload_id,
                    add_to_vault,
                    shared_client,
                )
                .await;
            }
        }
    } else {
        info!("User chose not to use cached receipts, will request full payment");
    }

    // Get store quotes (for missing chunks if partial, or all chunks if none cached)
    let missing_refs = if cached_result.need_additional_payment {
        Some(cached_result.missing_chunks.as_slice())
    } else {
        None
    };
    let store_quote = fetch_store_quotes(&client, all_content_addresses, missing_refs).await?;
    info!("Got store quote successfully");

    // If add_to_vault is true, get vault quote and merge with store quote
    let (total_store_quote, vault_update) = match (add_to_vault, vault_secret_key.as_ref()) {
        (true, Some(secret_key)) => {
            let prep = prepare_vault_update(
                &client,
                secret_key,
                store_quote,
                VaultItem::PrivateFile {
                    data_map_chunk: data_map_chunk.clone(),
                    name: file.name.clone(),
                },
            )
            .await?;
            (prep.merged_quote, prep.vault_update)
        }
        _ => (store_quote, Default::default()),
    };

    // Calculate total cost and emit quote event
    let total_cost: Amount = total_store_quote.payments().iter().map(|(_, _, amount)| *amount).sum();
    let payments = build_payments_json(&total_store_quote);
    let raw_payments = get_raw_payments(&total_store_quote);

    info!("Emitting upload-quote event for upload_id: {}", upload_id);
    emit_upload_quote(&app, &upload_id, 1, file_size, total_cost, &payments, &raw_payments)?;
    info!("Successfully emitted upload-quote event");

    // If cost is 0, this is a duplicate file - skip upload and mark as completed
    if total_cost == Amount::ZERO {
        info!("Duplicate file detected (cost=0), marking as completed immediately for upload_id: {}", upload_id);
        emit_upload_progress(
            &app,
            UploadProgress::Completed {
                upload_id: upload_id.clone(),
                total_files: 1,
                total_bytes: file_size,
                add_to_vault,
                file_access: Some(FileAccess::Private(data_map_chunk)),
            },
        )?;
        info!("Emitted completion event for duplicate file upload_id: {}", upload_id);
    } else if let Some(pending_uploads) = pending_uploads {
        // Store upload data for later execution after payment
        let mut pending = pending_uploads.lock().await;
        pending.store_single_file(
            upload_id.clone(),
            file,
            data_map_chunk,
            total_store_quote,
            vault_update,
            vault_secret_key.cloned(),
            add_to_vault,
            cached_result.cached_receipt,
        );
    }

    Ok(())
}

pub async fn execute_private_single_file_upload(
    app: AppHandle,
    file: File,
    datamap: DataMapChunk,
    receipt: Receipt,
    vault_update: vault::VaultUpdate,
    vault_secret_key: Option<&VaultSecretKey>,
    upload_id: String,
    add_to_vault: bool,
    shared_client: State<'_, SharedClient>,
) -> Result<(), UploadError> {
    let client = shared_client.get_client().await?;
    let file_size = fs::metadata(&file.path)
        .await
        .map_err(|_| UploadError::Read(file.path.clone()))?
        .len();

    // Emit started event
    app.emit(
        "upload-progress",
        UploadProgress::Started {
            upload_id: upload_id.clone(),
            total_files: 1,
            total_size: file_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Emit uploading progress
    app.emit(
        "upload-progress",
        UploadProgress::Uploading {
            upload_id: upload_id.clone(),
            chunks_uploaded: 0,
            total_chunks: 0, // todo: use actual chunk count
            bytes_uploaded: 0,
            total_bytes: file_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Clone vault_secret_key for async closure
    let vault_secret_key = vault_secret_key.cloned();

    tokio::spawn(async move {
        let result = async {
            // Create encryption stream for upload
            let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), false)
                .await
                .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

            let stream = encryption_streams
                .first_mut()
                .ok_or(UploadError::Encryption("Expected one stream".to_string()))?;

            batch_upload_encryption_stream(&client, &receipt, stream)
                .await
                .map_err(|err| UploadError::Put(format!("{:?}", err)))?;

            // Store file in vault if requested
            if add_to_vault {
                if let Some(secret_key) = vault_secret_key.as_ref() {
                    let mut user_data = client
                        .vault_get_user_data(&secret_key)
                        .await
                        .unwrap_or(UserData::new());

                    // Add the single file to user data (not as archive)
                    user_data
                        .private_files
                        .insert(DataMapChunk::from(datamap.clone()), file.name.clone());

                    // Serialize user data for vault update
                    let vault_data = user_data
                        .to_bytes()
                        .map_err(|e| UploadError::Serialization(e.to_string()))?;

                    // Use vault_update with the receipt (which already includes vault payment)
                    vault::vault_update(
                        &client,
                        vault_data,
                        &secret_key,
                        receipt,
                        vault_update.new_graph_entries,
                        vault_update.new_scratchpad_derivations,
                    )
                    .await
                    .map_err(|err| UploadError::StoreQuote(err.to_string()))?;
                }
            }

            // Store file locally
            local_storage::write_local_private_file(
                datamap.to_hex(),
                datamap.address(),
                &file.name,
            )
            .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            Ok::<(), UploadError>(())
        }
        .await;

        match result {
            Ok(()) => {
                // Emit completion
                if let Err(_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Completed {
                        upload_id: upload_id.clone(),
                        total_files: 1,
                        total_bytes: file_size,
                        add_to_vault,
                        file_access: Some(FileAccess::Private(datamap)),
                    },
                ) {
                    // Failed to emit completion
                }
            }
            Err(_err) => {
                // Emit failure - simplified
                let _ = app.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        upload_id: upload_id.clone(),
                        error: format!("Upload {}", "issue"),
                    },
                );
            }
        }
    });

    // Return immediately - the upload continues in background
    Ok(())
}

pub async fn start_public_single_file_upload(
    app: AppHandle,
    file: File,
    upload_id: String,
    add_to_vault: bool,
    use_cached_receipts: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
    pending_uploads: Option<&tokio::sync::Mutex<crate::PendingUploads>>,
) -> Result<(), UploadError> {
    info!(
        upload_id = %upload_id,
        add_to_vault = %add_to_vault,
        use_cached_receipts = %use_cached_receipts,
        "start_single_file_upload_public called"
    );
    let client = shared_client.get_client().await.map_err(|e| {
        error!("Failed to get client: {:?}", e);
        e
    })?;

    // Calculate file size
    let file_size = fs::metadata(&file.path)
        .await
        .map_err(|_| UploadError::Read(file.path.clone()))?
        .len();

    let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), true)
        .await
        .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

    let stream = encryption_streams
        .first_mut()
        .ok_or(UploadError::Encryption("Expected one stream".to_string()))?;

    let all_content_addresses = content_addresses_from_encryption_stream(stream).await;

    let data_map_chunk = stream.data_map_chunk().ok_or(UploadError::Encryption(
        "Missing data map chunk".to_string(),
    ))?;

    // Check for cached payment first (only if user wants to use cached receipts)
    let mut cached_result = CachedPaymentResult {
        cached_receipt: None,
        need_additional_payment: false,
        missing_chunks: vec![],
        is_fully_cached: false,
    };

    if use_cached_receipts {
        if let Ok(cache) = get_payment_cache() {
            info!("Checking for cached payment for public file: {:?}", file.path);
            let content_addresses: Vec<XorName> = all_content_addresses
                .iter()
                .map(|(address, _)| *address)
                .collect();

            cached_result = check_cached_payment_for_file(cache, &file.path, &content_addresses);

            if cached_result.is_fully_cached {
                info!("Cached receipt covers all chunks, reusing it for public upload");
                emit_zero_cost_quote(&app, &upload_id, 1, file_size)?;

                return execute_public_single_file_upload(
                    app,
                    file,
                    data_map_chunk,
                    cached_result.cached_receipt.unwrap(),
                    Default::default(),
                    upload_id,
                    add_to_vault,
                    vault_secret_key,
                    shared_client,
                )
                .await;
            }
        }
    } else {
        info!("User chose not to use cached receipts, will request full payment");
    }

    // Get store quotes (for missing chunks if partial, or all chunks if none cached)
    let missing_refs = if cached_result.need_additional_payment {
        Some(cached_result.missing_chunks.as_slice())
    } else {
        None
    };
    let store_quote = fetch_store_quotes(&client, all_content_addresses, missing_refs).await?;
    info!("Got store quote successfully");

    // If add_to_vault is true, get vault quote and merge with store quote
    let (total_store_quote, vault_update) = match (add_to_vault, vault_secret_key.as_ref()) {
        (true, Some(secret_key)) => {
            let data_address = DataAddress::new(*data_map_chunk.0.address().xorname());
            let prep = prepare_vault_update(
                &client,
                secret_key,
                store_quote,
                VaultItem::PublicFile {
                    data_address,
                    name: file.name.clone(),
                },
            )
            .await?;
            (prep.merged_quote, prep.vault_update)
        }
        _ => (store_quote, Default::default()),
    };

    // Calculate total cost and emit quote event
    let total_cost: Amount = total_store_quote.payments().iter().map(|(_, _, amount)| *amount).sum();
    let payments = build_payments_json(&total_store_quote);
    let raw_payments = get_raw_payments(&total_store_quote);

    info!("Emitting upload-quote event for public upload_id: {}", upload_id);
    emit_upload_quote(&app, &upload_id, 1, file_size, total_cost, &payments, &raw_payments)?;
    debug!("Successfully emitted upload-quote event");

    // If cost is 0, this is a duplicate file - skip upload and mark as completed
    if total_cost == Amount::ZERO {
        info!("Duplicate public file detected (cost=0), marking as completed immediately for upload_id: {}", upload_id);
        emit_upload_progress(
            &app,
            UploadProgress::Completed {
                upload_id: upload_id.clone(),
                total_files: 1,
                total_bytes: file_size,
                add_to_vault,
                file_access: Some(FileAccess::Public(DataAddress::new(*data_map_chunk.0.name()))),
            },
        )?;
        info!("Emitted completion event for duplicate public file upload_id: {}", upload_id);
    } else if let Some(pending_uploads) = pending_uploads {
        // Store upload data for later execution after payment
        let mut pending = pending_uploads.lock().await;
        pending.store_single_file_public(
            upload_id.clone(),
            file,
            data_map_chunk,
            total_store_quote,
            vault_update,
            add_to_vault,
            vault_secret_key.cloned(),
            cached_result.cached_receipt,
        );
    }

    Ok(())
}

pub async fn execute_public_single_file_upload(
    app: AppHandle,
    file: File,
    datamap: DataMapChunk,
    receipt: Receipt,
    vault_update: vault::VaultUpdate,
    upload_id: String,
    add_to_vault: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
) -> Result<(), UploadError> {
    info!(
        ">>> execute_single_file_upload_public called for upload_id: {}, add_to_vault: {}",
        upload_id, add_to_vault
    );

    let client = shared_client.get_client().await?;
    let file_size = fs::metadata(&file.path)
        .await
        .map_err(|_| UploadError::Read(file.path.clone()))?
        .len();

    // Emit upload started progress
    app.emit(
        "upload-progress",
        UploadProgress::Started {
            upload_id: upload_id.clone(),
            total_files: 1,
            total_size: file_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Emit uploading progress
    app.emit(
        "upload-progress",
        UploadProgress::Uploading {
            upload_id: upload_id.clone(),
            chunks_uploaded: 0,
            total_chunks: 0, // todo: use actual chunk count
            bytes_uploaded: 0,
            total_bytes: file_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Clone vault_secret_key for async closure
    let vault_secret_key = vault_secret_key.cloned();

    // Clone data for completion event (public files use DataAddress)
    let public_data_address = DataAddress::new(*datamap.0.name());

    // Spawn the actual upload work in background
    tokio::spawn(async move {
        let result = async {
            let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), true)
                .await
                .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

            let total_chunks = encryption_streams.len();

            let stream = encryption_streams
                .first_mut()
                .ok_or(UploadError::Encryption("Expected one stream".to_string()))?;

            batch_upload_encryption_stream(&client, &receipt, stream)
                .await
                .map_err(|err| UploadError::Put(format!("{:?}", err)))?;

            info!("Uploaded {} file chunks + datamap", total_chunks,);

            // The public file's address is the datamap's address (not the file chunks)
            let public_data_address = DataAddress::new(*datamap.0.name());

            info!(
                ">>> Public file uploaded successfully with address: {:?}",
                public_data_address
            );

            // Emit final uploading progress
            app.emit(
                "upload-progress",
                UploadProgress::Uploading {
                    upload_id: upload_id.clone(),
                    chunks_uploaded: total_chunks,
                    total_chunks,
                    bytes_uploaded: file_size,
                    total_bytes: file_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

            // Add to vault if requested
            if add_to_vault {
                if let Some(secret_key) = vault_secret_key.as_ref() {
                    info!("Adding public file to vault...");

                    let file_name = file.name.clone();

                    let mut user_data = client
                        .vault_get_user_data(&secret_key)
                        .await
                        .unwrap_or(UserData::new());

                    // Add the public file to the vault using the datamap address
                    user_data
                        .public_files
                        .insert(public_data_address, file_name.clone());

                    // Serialize user data for vault update
                    let vault_data = user_data
                        .to_bytes()
                        .map_err(|e| UploadError::Serialization(e.to_string()))?;

                    // Use vault_update with the receipt (which already includes vault payment)
                    vault::vault_update(
                        &client,
                        vault_data,
                        &secret_key,
                        receipt,
                        vault_update.new_graph_entries,
                        vault_update.new_scratchpad_derivations,
                    )
                    .await
                    .map_err(|err| {
                        info!("Failed to update vault: {:?}", err);
                        UploadError::Scratchpad(err.to_string())
                    })?;

                    info!("Successfully added public file to vault");
                } else {
                    warn!(">>> Warning: add_to_vault=true but no vault_secret_key provided");
                }
            }

            // Store the file locally for future reference
            local_storage::write_local_public_file(
                hex::encode(public_data_address.xorname().0),
                &file.name,
            )
            .map_err(|err| {
                warn!(">>> Warning: Failed to store local reference: {:?}", err);
                // Don't fail the upload for local storage issues
                err
            })
            .ok();

            Ok::<(), UploadError>(())
        }
        .await;

        match result {
            Ok(()) => {
                // Emit completion
                if let Err(_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Completed {
                        upload_id: upload_id.clone(),
                        total_files: 1,
                        total_bytes: file_size,
                        add_to_vault,
                        file_access: Some(FileAccess::Public(public_data_address)),
                    },
                ) {
                    // Failed to emit completion
                }
            }
            Err(_err) => {
                // Emit failure - simplified
                let _ = app.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        upload_id: upload_id.clone(),
                        error: format!("Upload {}", "issue"),
                    },
                );
            }
        }
    });

    // Return immediately - the upload continues in background
    Ok(())
}

pub async fn start_private_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    upload_id: String,
    add_to_vault: bool,
    use_cached_receipts: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
    pending_uploads: Option<&tokio::sync::Mutex<crate::PendingUploads>>,
) -> Result<(), UploadError> {
    info!(
        upload_id = %upload_id,
        add_to_vault = %add_to_vault,
        use_cached_receipts = %use_cached_receipts,
        "start_private_archive_upload called"
    );

    let client = shared_client.get_client().await?;

    // Calculate total size and collect files
    let total_size = calculate_total_size(&files).await?;
    let total_files = files.len();

    // Use encryption streaming for private archives
    let mut private_archive = PrivateArchive::new();
    let mut all_content_addresses = Vec::new();

    for file in &files {
        let path_metadata = fs::metadata(&file.path)
            .await
            .map_err(|_| UploadError::Read(file.path.clone()))?;

        if path_metadata.is_dir() {
            info!("Creating encryption streams for directory: {:?}", file.path);
            let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), false)
                .await
                .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

            for stream in &mut encryption_streams {
                let file_path = PathBuf::from(&stream.file_path);
                let file_metadata = stream.metadata.clone();
                let content_addresses = content_addresses_from_encryption_stream(stream).await;
                all_content_addresses.extend(content_addresses.clone());

                let base_dir = file.path.parent().unwrap_or(&file.path);
                let relative_path = file_path
                    .strip_prefix(base_dir)
                    .unwrap_or(&file_path)
                    .to_path_buf();

                let datamap = stream
                    .data_map_chunk()
                    .ok_or(UploadError::Encryption("Failed to get datamap".to_string()))?;

                private_archive.add_file(relative_path, datamap, file_metadata);
            }
        } else {
            info!("Creating encryption stream for file: {:?}", file.path);
            let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), false)
                .await
                .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

            let stream = encryption_streams
                .first_mut()
                .ok_or(UploadError::Encryption("Expected one stream".to_string()))?;

            let file_metadata = stream.metadata.clone();
            let content_addresses = content_addresses_from_encryption_stream(stream).await;
            all_content_addresses.extend(content_addresses);

            let datamap = stream
                .data_map_chunk()
                .ok_or(UploadError::Encryption("Failed to get datamap".to_string()))?;

            private_archive.add_file(file.path.clone(), datamap, file_metadata);
        }
    }

    // Serialize and encrypt the archive itself
    let archive_bytes = private_archive
        .to_bytes()
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    let (archive_datamap, archive_chunks) = autonomi::self_encryption::encrypt(archive_bytes)
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    // Add archive chunk content addresses
    let archive_content_addresses: Vec<(XorName, usize)> = archive_chunks
        .iter()
        .map(|chunk| (*chunk.name(), chunk.size()))
        .collect();
    all_content_addresses.extend(archive_content_addresses);

    let archive_datamap_chunk = DataMapChunk::from(archive_datamap.clone());

    // Check for cached payment first (only if user wants to use cached receipts)
    let mut cached_result = CachedPaymentResult {
        cached_receipt: None,
        need_additional_payment: false,
        missing_chunks: vec![],
        is_fully_cached: false,
    };

    if use_cached_receipts {
        if let Ok(cache) = get_payment_cache() {
            info!("Checking for cached payment for private archive: {}", archive_name);
            let content_addresses: Vec<XorName> = all_content_addresses
                .iter()
                .map(|(address, _)| *address)
                .collect();

            cached_result = check_cached_payment_for_archive(cache, &files, &archive_name, &content_addresses);

            if cached_result.is_fully_cached {
                info!("Cached receipt covers all chunks, reusing it for private archive upload");
                emit_zero_cost_quote(&app, &upload_id, total_files, total_size)?;

                return execute_private_archive_upload(
                    app,
                    files,
                    archive_name,
                    archive_datamap_chunk,
                    private_archive,
                    cached_result.cached_receipt.unwrap(),
                    Default::default(),
                    upload_id,
                    add_to_vault,
                    vault_secret_key,
                    shared_client,
                )
                .await;
            }
        }
    } else {
        info!("User chose not to use cached receipts, will request full payment");
    }

    // Get store quotes (for missing chunks if partial, or all chunks if none cached)
    let missing_refs = if cached_result.need_additional_payment {
        Some(cached_result.missing_chunks.as_slice())
    } else {
        None
    };
    let store_quote = fetch_store_quotes(&client, all_content_addresses, missing_refs).await?;
    debug!("Got store quote successfully");

    // If add_to_vault is true, get vault quote and merge with store quote
    let (total_store_quote, vault_update) = match (add_to_vault, vault_secret_key.as_ref()) {
        (true, Some(secret_key)) => {
            let prep = prepare_vault_update(
                &client,
                secret_key,
                store_quote,
                VaultItem::PrivateArchive {
                    data_map_chunk: archive_datamap_chunk.clone(),
                    name: archive_name.clone(),
                },
            )
            .await?;
            (prep.merged_quote, prep.vault_update)
        }
        _ => (store_quote, Default::default()),
    };

    // Calculate total cost and emit quote event
    let total_cost: Amount = total_store_quote.payments().iter().map(|(_, _, amount)| *amount).sum();
    let payments = build_payments_json(&total_store_quote);
    let raw_payments = get_raw_payments(&total_store_quote);

    debug!(upload_id = %upload_id, "Emitting upload-quote event for private archive");
    emit_upload_quote(&app, &upload_id, total_files, total_size, total_cost, &payments, &raw_payments)?;
    debug!("Successfully emitted upload-quote event");

    // If cost is 0, this is a duplicate archive - mark as completed
    if total_cost == Amount::ZERO {
        debug!(upload_id = %upload_id, "Duplicate private archive detected (cost=0), marking as completed immediately");
        emit_upload_progress(
            &app,
            UploadProgress::Completed {
                upload_id: upload_id.clone(),
                total_files,
                total_bytes: total_size,
                add_to_vault,
                file_access: Some(FileAccess::Private(archive_datamap_chunk)),
            },
        )?;
        debug!(upload_id = %upload_id, "Emitted completion event for duplicate private archive");
    } else if let Some(pending_uploads) = pending_uploads {
        // Store upload data for later execution after payment
        let mut pending = pending_uploads.lock().await;
        pending.store_private_archive(
            upload_id.clone(),
            files,
            archive_name,
            archive_datamap_chunk,
            private_archive,
            total_store_quote,
            vault_update,
            add_to_vault,
            vault_secret_key.cloned(),
            cached_result.cached_receipt,
        );
    }

    Ok(())
}

pub async fn execute_private_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    archive_datamap: DataMapChunk,
    archive: PrivateArchive,
    receipt: Receipt,
    vault_update: vault::VaultUpdate,
    upload_id: String,
    add_to_vault: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
) -> Result<(), UploadError> {
    info!(
        ">>> execute_private_archive_upload called for upload_id: {}, add_to_vault: {}",
        upload_id, add_to_vault
    );

    let client = shared_client.get_client().await?;
    let total_size = calculate_total_size(&files).await?;

    // Emit upload started progress
    app.emit(
        "upload-progress",
        UploadProgress::Started {
            upload_id: upload_id.clone(),
            total_files: files.len(),
            total_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Emit uploading progress
    app.emit(
        "upload-progress",
        UploadProgress::Uploading {
            upload_id: upload_id.clone(),
            chunks_uploaded: 0,
            total_chunks: 0, // Will be determined from streams
            bytes_uploaded: 0,
            total_bytes: total_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Clone vault_secret_key for async closure
    let vault_secret_key = vault_secret_key.cloned();

    // Clone datamap for completion event
    let completion_datamap = archive_datamap.clone();

    // Serialize and encrypt the archive metadata
    let archive_bytes = archive
        .to_bytes()
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    let (_, archive_chunks) = autonomi::self_encryption::encrypt(archive_bytes)
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    // Spawn the actual upload work in background
    tokio::spawn(async move {
        let result = async {
            // Use encryption streaming for all files
            let mut total_chunks = 0;

            for file in &files {
                let path_metadata = fs::metadata(&file.path)
                    .await
                    .map_err(|_| UploadError::Read(file.path.clone()))?;

                if path_metadata.is_dir() {
                    // Handle directory - encrypt_file_or_folder will handle all files in the directory
                    info!(
                        ">>> Creating encryption streams for directory: {:?}",
                        file.path
                    );

                    let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), false)
                        .await
                        .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

                    // Each stream corresponds to a file in the directory
                    for stream in &mut encryption_streams {
                        total_chunks += stream.total_chunks();

                        batch_upload_encryption_stream(&client, &receipt, stream)
                            .await
                            .map_err(|err| UploadError::Put(format!("{:?}", err)))?;
                    }
                } else {
                    // Handle single file
                    info!("Creating encryption stream for file: {:?}", file.path);
                    let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), false)
                        .await
                        .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

                    let stream = encryption_streams
                        .first_mut()
                        .ok_or(UploadError::Encryption("Expected one stream".to_string()))?;

                    total_chunks += stream.total_chunks();

                    batch_upload_encryption_stream(&client, &receipt, stream)
                        .await
                        .map_err(|err| UploadError::Put(format!("{:?}", err)))?;
                }
            }

            // Upload archive metadata chunks
            total_chunks += archive_chunks.len();

            client
                .chunk_batch_upload(archive_chunks.iter().collect(), &receipt)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            info!(
                ">>> Private archive uploaded successfully with local datamap: {:?}",
                archive_datamap.to_hex()
            );

            // Emit final uploading progress
            app.emit(
                "upload-progress",
                UploadProgress::Uploading {
                    upload_id: upload_id.clone(),
                    chunks_uploaded: total_chunks,
                    total_chunks,
                    bytes_uploaded: total_size,
                    total_bytes: total_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

            // Add to vault if requested
            if add_to_vault {
                if let Some(secret_key) = vault_secret_key.as_ref() {
                    info!("Adding private archive to vault...");

                    let mut user_data = client
                        .vault_get_user_data(&secret_key)
                        .await
                        .unwrap_or(UserData::new());

                    // Add the private archive to the vault using the archive datamap
                    user_data
                        .private_file_archives
                        .insert(archive_datamap.clone(), archive_name.clone());

                    // Serialize user data for vault update
                    let vault_data = user_data
                        .to_bytes()
                        .map_err(|e| UploadError::Serialization(e.to_string()))?;

                    vault::vault_update(
                        &client,
                        vault_data,
                        secret_key,
                        receipt,
                        vault_update.new_graph_entries,
                        vault_update.new_scratchpad_derivations,
                    )
                    .await
                    .map_err(|err| {
                        error!(">>> Failed to update vault: {:?}", err);
                        UploadError::Scratchpad(err.to_string())
                    })?;

                    info!("Successfully added private archive to vault");
                } else {
                    warn!(">>> Warning: add_to_vault=true but no vault_secret_key provided");
                }
            }

            // Store the archive locally for future reference
            local_storage::write_local_private_file_archive(
                archive_datamap.to_hex(),
                archive_datamap.address(),
                &archive_name,
            )
            .map_err(|err| {
                warn!(">>> Warning: Failed to store local reference: {:?}", err);
                // Don't fail the upload for local storage issues
                err
            })
            .ok();

            Ok::<(), UploadError>(())
        }
        .await;

        match result {
            Ok(()) => {
                // Emit completion
                if let Err(_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Completed {
                        upload_id: upload_id.clone(),
                        total_files: files.len(),
                        total_bytes: total_size,
                        add_to_vault,
                        file_access: Some(FileAccess::Private(completion_datamap)),
                    },
                ) {
                    // Failed to emit completion
                }
            }
            Err(err) => {
                // Emit failure
                if let Err(_emit_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        upload_id: upload_id.clone(),
                        error: err.to_string(),
                    },
                ) {
                    error!("Failed to emit failure event: {}", err);
                }
            }
        }
    });

    // Return immediately - the upload continues in background
    Ok(())
}

pub async fn start_public_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    upload_id: String,
    add_to_vault: bool,
    use_cached_receipts: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
    pending_uploads: Option<&tokio::sync::Mutex<crate::PendingUploads>>,
) -> Result<(), UploadError> {
    info!(
        upload_id = %upload_id,
        add_to_vault = %add_to_vault,
        use_cached_receipts = %use_cached_receipts,
        "start_public_archive_upload called"
    );

    let client = shared_client.get_client().await?;

    // Calculate total size and collect files
    let total_size = calculate_total_size(&files).await?;
    let total_files = files.len();

    // Use encryption streaming for all files
    let mut public_archive = PublicArchive::new();
    let mut all_content_addresses = vec![];

    for file in &files {
        let path_metadata = fs::metadata(&file.path)
            .await
            .map_err(|_| UploadError::Read(file.path.clone()))?;

        if path_metadata.is_dir() {
            info!("Creating encryption streams for directory: {:?}", file.path);
            let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), true)
                .await
                .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

            for stream in &mut encryption_streams {
                let file_path = PathBuf::from(stream.file_path.clone());
                let file_size = fs::metadata(&file_path)
                    .await
                    .map_err(|_| UploadError::Read(file_path.clone()))?
                    .len();

                let data_map_chunk =
                    stream
                        .data_map_chunk()
                        .ok_or(UploadError::Encryption(format!(
                            "Missing data map chunk for file: {:?}",
                            file_path
                        )))?;

                let base_dir = file.path.parent().unwrap_or(&file.path);
                let relative_path = file_path
                    .strip_prefix(base_dir)
                    .unwrap_or(&file_path)
                    .to_path_buf();

                let metadata = Metadata::new_with_size(file_size);
                public_archive.add_file(
                    relative_path,
                    DataAddress::new(*data_map_chunk.0.name()),
                    metadata,
                );

                let content_addresses = content_addresses_from_encryption_stream(stream).await;
                all_content_addresses.extend(content_addresses);
            }
        } else {
            info!("Creating encryption stream for file: {:?}", file.path);
            let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), true)
                .await
                .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

            let stream = encryption_streams
                .first_mut()
                .ok_or(UploadError::Encryption("Expected one stream".to_string()))?;

            let file_size = fs::metadata(&file.path)
                .await
                .map_err(|_| UploadError::Read(file.path.clone()))?
                .len();

            let data_map_chunk = stream.data_map_chunk().ok_or(UploadError::Encryption(
                "Missing data map chunk".to_string(),
            ))?;

            let metadata = Metadata::new_with_size(file_size);
            public_archive.add_file(
                file.path.clone(),
                DataAddress::new(*data_map_chunk.0.name()),
                metadata,
            );

            let content_addresses = content_addresses_from_encryption_stream(stream).await;
            all_content_addresses.extend(content_addresses);
        }
    }

    // Serialize and encrypt the archive metadata
    let archive_bytes = public_archive
        .to_bytes()
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    info!("Encrypting archive");
    let (archive_datamap, archive_chunks) = autonomi::self_encryption::encrypt(archive_bytes)
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    let archive_datamap_chunk = DataMapChunk::from(archive_datamap.clone());
    let public_data_address = DataAddress::new(*archive_datamap_chunk.0.name());
    let mut total_archive_chunks = archive_chunks;
    total_archive_chunks.push(archive_datamap);

    let archive_content_addresses = total_archive_chunks
        .iter()
        .map(|chunk| (*chunk.address.xorname(), chunk.value.len()));
    all_content_addresses.extend(archive_content_addresses);

    // Check for cached payment first (only if user wants to use cached receipts)
    let mut cached_result = CachedPaymentResult {
        cached_receipt: None,
        need_additional_payment: false,
        missing_chunks: vec![],
        is_fully_cached: false,
    };

    if use_cached_receipts {
        if let Ok(cache) = get_payment_cache() {
            debug!(archive_name = %archive_name, "Checking for cached payment for public archive");
            let content_addresses: Vec<XorName> = all_content_addresses
                .iter()
                .map(|(address, _)| *address)
                .collect();

            cached_result = check_cached_payment_for_archive(cache, &files, &archive_name, &content_addresses);

            if cached_result.is_fully_cached {
                debug!("Cached receipt covers all chunks, reusing it for public archive upload");
                emit_zero_cost_quote(&app, &upload_id, total_files, total_size)?;

                return execute_public_archive_upload(
                    app,
                    files,
                    archive_name,
                    public_archive,
                    cached_result.cached_receipt.unwrap(),
                    Default::default(),
                    upload_id,
                    add_to_vault,
                    vault_secret_key,
                    shared_client,
                )
                .await;
            }
        }
    } else {
        debug!("User chose not to use cached receipts, will request full payment");
    }

    // Get store quotes (for missing chunks if partial, or all chunks if none cached)
    let missing_refs = if cached_result.need_additional_payment {
        Some(cached_result.missing_chunks.as_slice())
    } else {
        None
    };
    let store_quote = fetch_store_quotes(&client, all_content_addresses, missing_refs).await?;

    // If add_to_vault is true, get vault quote and merge with store quote
    let (total_store_quote, vault_update) = match (add_to_vault, vault_secret_key.as_ref()) {
        (true, Some(secret_key)) => {
            let prep = prepare_vault_update(
                &client,
                secret_key,
                store_quote,
                VaultItem::PublicArchive {
                    data_address: public_data_address,
                    name: archive_name.clone(),
                },
            )
            .await?;
            (prep.merged_quote, prep.vault_update)
        }
        _ => (store_quote, Default::default()),
    };

    info!("Got store quote successfully");

    // Calculate total cost and emit quote event
    let total_cost: Amount = total_store_quote.payments().iter().map(|(_, _, amount)| *amount).sum();
    let payments = build_payments_json(&total_store_quote);
    let raw_payments = get_raw_payments(&total_store_quote);

    debug!(upload_id = %upload_id, "Emitting upload-quote event for public archive");
    emit_upload_quote(&app, &upload_id, total_files, total_size, total_cost, &payments, &raw_payments)?;
    debug!("Successfully emitted upload-quote event");

    // If cost is 0, this is a duplicate archive - mark as completed
    if total_cost == Amount::ZERO {
        debug!(upload_id = %upload_id, "Duplicate public archive detected (cost=0), marking as completed immediately");
        emit_upload_progress(
            &app,
            UploadProgress::Completed {
                upload_id: upload_id.clone(),
                total_files,
                total_bytes: total_size,
                add_to_vault,
                file_access: Some(FileAccess::Public(public_data_address)),
            },
        )?;
        debug!(upload_id = %upload_id, "Emitted completion event for duplicate public archive");
    } else if let Some(pending_uploads) = pending_uploads {
        // Store upload data for later execution after payment
        let mut pending = pending_uploads.lock().await;
        pending.store_public_archive(
            upload_id.clone(),
            files,
            archive_name,
            public_archive,
            total_store_quote,
            vault_update,
            add_to_vault,
            vault_secret_key.cloned(),
            cached_result.cached_receipt,
        );
    }

    Ok(())
}

pub async fn execute_public_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    archive: PublicArchive,
    receipt: Receipt,
    vault_update: vault::VaultUpdate,
    upload_id: String,
    add_to_vault: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
) -> Result<(), UploadError> {
    debug!(
        upload_id = %upload_id,
        add_to_vault = %add_to_vault,
        "execute_public_archive_upload called"
    );

    let client = shared_client.get_client().await?;
    let total_size = calculate_total_size(&files).await?;

    // Emit upload started progress
    app.emit(
        "upload-progress",
        UploadProgress::Started {
            upload_id: upload_id.clone(),
            total_files: files.len(),
            total_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Emit uploading progress
    app.emit(
        "upload-progress",
        UploadProgress::Uploading {
            upload_id: upload_id.clone(),
            chunks_uploaded: 0,
            total_chunks: 0, // Will be determined from streams
            bytes_uploaded: 0,
            total_bytes: total_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Clone vault_secret_key for async closure
    let vault_secret_key = vault_secret_key.cloned();

    // Serialize and encrypt the archive metadata
    let archive_bytes = archive
        .to_bytes()
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    let (archive_datamap, archive_chunks) = autonomi::self_encryption::encrypt(archive_bytes)
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    let archive_datamap_chunk = DataMapChunk::from(archive_datamap.clone());

    // The public archive's address is the archive datamap's address
    let public_archive_address = DataAddress::new(archive_datamap_chunk.0.name().to_owned());

    // Spawn the actual upload work in background
    tokio::spawn(async move {
        let result = async {
            // Use encryption streaming for all files
            let mut total_chunks = 0;

            for file in &files {
                let path_metadata = fs::metadata(&file.path)
                    .await
                    .map_err(|_| UploadError::Read(file.path.clone()))?;

                if path_metadata.is_dir() {
                    // Handle directory - encrypt_file_or_folder will handle all files in the directory
                    info!(
                        ">>> Creating encryption streams for directory: {:?}",
                        file.path
                    );

                    let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), true)
                        .await
                        .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

                    // Each stream corresponds to a file in the directory
                    for stream in &mut encryption_streams {
                        total_chunks += stream.total_chunks();

                        batch_upload_encryption_stream(&client, &receipt, stream)
                            .await
                            .map_err(|err| UploadError::Put(format!("{:?}", err)))?;
                    }
                } else {
                    // Handle single file
                    info!("Creating encryption stream for file: {:?}", file.path);
                    let mut encryption_streams = encrypt_file_or_folder(file.path.clone(), true)
                        .await
                        .map_err(|err| UploadError::Encryption(format!("{:?}", err)))?;

                    let stream = encryption_streams
                        .first_mut()
                        .ok_or(UploadError::Encryption("Expected one stream".to_string()))?;

                    total_chunks += stream.total_chunks();

                    batch_upload_encryption_stream(&client, &receipt, stream)
                        .await
                        .map_err(|err| UploadError::Put(format!("{:?}", err)))?;
                }
            }

            let mut total_archive_chunks = archive_chunks;
            total_archive_chunks.push(archive_datamap);

            total_chunks += total_archive_chunks.len();

            client
                .chunk_batch_upload(total_archive_chunks.iter().collect(), &receipt)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            debug!(
                address = ?public_archive_address,
                "Public archive uploaded successfully"
            );

            // Emit final uploading progress
            app.emit(
                "upload-progress",
                UploadProgress::Uploading {
                    upload_id: upload_id.clone(),
                    chunks_uploaded: total_chunks,
                    total_chunks,
                    bytes_uploaded: total_size,
                    total_bytes: total_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

            // Add to vault if requested
            if add_to_vault {
                if let Some(secret_key) = vault_secret_key.as_ref() {
                    debug!("Adding public archive to vault...");

                    let mut user_data = client
                        .vault_get_user_data(&secret_key)
                        .await
                        .unwrap_or(UserData::new());

                    // Add the public archive to the vault using the archive datamap address
                    user_data
                        .file_archives
                        .insert(public_archive_address, archive_name.clone());

                    // Serialize user data for vault update
                    let vault_data = user_data
                        .to_bytes()
                        .map_err(|e| UploadError::Serialization(e.to_string()))?;

                    vault::vault_update(
                        &client,
                        vault_data,
                        &secret_key,
                        receipt,
                        vault_update.new_graph_entries,
                        vault_update.new_scratchpad_derivations,
                    )
                    .await
                    .map_err(|err| {
                        error!(">>> Failed to update vault: {:?}", err);
                        UploadError::Scratchpad(err.to_string())
                    })?;

                    info!("Successfully added public archive to vault");
                } else {
                    error!(">>> Warning: add_to_vault=true but no vault_secret_key provided");
                }
            }

            // Store the archive locally for future reference
            local_storage::write_local_public_file_archive(
                hex::encode(public_archive_address.xorname().0),
                &archive_name,
            )
            .map_err(|err| {
                warn!(">>> Warning: Failed to store local reference: {:?}", err);
                // Don't fail the upload for local storage issues
                err
            })
            .ok();

            Ok::<(), UploadError>(())
        }
        .await;

        match result {
            Ok(()) => {
                // Emit completion
                if let Err(_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Completed {
                        upload_id: upload_id.clone(),
                        total_files: files.len(),
                        total_bytes: total_size,
                        add_to_vault,
                        file_access: Some(FileAccess::Public(public_archive_address)),
                    },
                ) {
                    // Failed to emit completion
                }
            }
            Err(err) => {
                // Emit failure
                if let Err(_emit_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        upload_id: upload_id.clone(),
                        error: err.to_string(),
                    },
                ) {
                    error!("Failed to emit failure event: {}", err);
                }
            }
        }
    });

    // Return immediately - the upload continues in background
    Ok(())
}

pub async fn get_vault_structure(
    secret_key: &VaultSecretKey,
    shared_client: State<'_, SharedClient>,
) -> Result<VaultStructure, VaultError> {
    let client = shared_client.get_client().await?;

    // Fetch user data
    let user_data = client.vault_get_user_data(secret_key).await?;

    let mut archives: Vec<ArchiveInfo> = vec![];
    let mut failed_archives: Vec<FailedArchive> = vec![];

    // Process private archives
    for (data_map, name) in user_data.private_file_archives {
        let archive_name = name.clone();

        if let Ok(archive) = client.archive_get(&data_map).await {
            let mut files: Vec<FileMetadata> = vec![];

            for (filepath, (data_map, metadata)) in archive.map() {
                let file = FileMetadata {
                    path: filepath.display().to_string(),
                    metadata: metadata.clone(),
                    file_type: FileType::Private,
                    is_loaded: false,
                    archive_name: archive_name.clone(),
                    access_data: Some(FileAccess::Private(data_map.clone())),
                };
                files.push(file);
            }

            archives.push(ArchiveInfo {
                name: archive_name.clone(),
                address: data_map.to_hex(),
                is_private: true,
                files,
            });
        } else {
            failed_archives.push(FailedArchive {
                name: archive_name.clone(),
                address: data_map.to_hex(),
                is_private: true,
            });
        }
    }

    // Process public archives
    for (archive_addr, name) in user_data.file_archives {
        let archive_name = name.clone();

        if let Ok(archive) = client.archive_get_public(&archive_addr).await {
            let mut files: Vec<FileMetadata> = vec![];

            for (filepath, (data_addr, metadata)) in archive.map() {
                let file = FileMetadata {
                    path: filepath.display().to_string(),
                    metadata: metadata.clone(),
                    file_type: FileType::Public,
                    is_loaded: false,
                    archive_name: archive_name.clone(),
                    access_data: Some(FileAccess::Public(*data_addr)),
                };
                files.push(file);
            }

            archives.push(ArchiveInfo {
                name: archive_name.clone(),
                address: archive_addr.to_hex(),
                is_private: false,
                files,
            });
        } else {
            failed_archives.push(FailedArchive {
                name: archive_name.clone(),
                address: archive_addr.to_hex(),
                is_private: false,
            });
        }
    }

    // Process individual files
    let mut individual_files: Vec<FileMetadata> = vec![];

    // Process individual private files
    for (data_map, name) in &user_data.private_files {
        let file = FileMetadata {
            path: name.clone(),
            metadata: autonomi::files::Metadata::new_with_size(0),
            file_type: FileType::Private,
            is_loaded: true,
            archive_name: String::new(),
            access_data: Some(FileAccess::Private(data_map.clone())),
        };
        individual_files.push(file);
    }

    // Process individual public files
    for (data_addr, name) in &user_data.public_files {
        let file = FileMetadata {
            path: name.clone(),
            metadata: autonomi::files::Metadata::new_with_size(0),
            file_type: FileType::Public,
            is_loaded: true,
            archive_name: String::new(),
            access_data: Some(FileAccess::Public(*data_addr)),
        };
        individual_files.push(file);
    }

    Ok(VaultStructure {
        archives,
        failed_archives,
        files: individual_files,
    })
}

pub async fn get_vault_structure_streaming(
    app: tauri::AppHandle,
    secret_key: &VaultSecretKey,
    temp_code: String,
    shared_client: State<'_, SharedClient>,
) -> Result<(), VaultError> {
    let client = shared_client.get_client().await?;

    // Fetch user data
    let user_data = client.vault_get_user_data(secret_key).await?;

    // First, emit individual files immediately (these are fast)
    let mut individual_files: Vec<FileMetadata> = vec![];

    // Process individual private files
    for (data_map, name) in &user_data.private_files {
        let file = FileMetadata {
            path: name.clone(),
            metadata: autonomi::files::Metadata::new_with_size(0),
            file_type: FileType::Private,
            is_loaded: true,
            archive_name: String::new(),
            access_data: Some(FileAccess::Private(data_map.clone())),
        };
        individual_files.push(file);
    }

    // Process individual public files
    for (data_addr, name) in &user_data.public_files {
        let file = FileMetadata {
            path: name.clone(),
            metadata: autonomi::files::Metadata::new_with_size(0),
            file_type: FileType::Public,
            is_loaded: true,
            archive_name: String::new(),
            access_data: Some(FileAccess::Public(*data_addr)),
        };
        individual_files.push(file);
    }

    // Emit individual files first if we have any
    if !individual_files.is_empty() {
        let update = VaultUpdate {
            update_type: VaultUpdateType::IndividualFiles,
            archive: None,
            failed_archive: None,
            loading_archive: None,
            files: individual_files,
            is_complete: false,
            temp_code: temp_code.clone(),
        };
        app.emit("vault-update", update)
            .map_err(|_| VaultError::FileNotFound)?;
    }

    // Process archives concurrently
    let mut archive_tasks = vec![];

    // Create tasks for private archives
    for (data_map, name) in &user_data.private_file_archives {
        let client = client.clone();
        let app = app.clone();
        let archive_name = name.clone();
        let data_map = data_map.clone();

        // Emit loading status immediately
        let loading_update = VaultUpdate {
            update_type: VaultUpdateType::ArchiveLoading,
            archive: None,
            failed_archive: None,
            loading_archive: Some(LoadingArchive {
                name: archive_name.clone(),
                address: data_map.to_hex(),
                is_private: true,
            }),
            files: vec![],
            is_complete: false,
            temp_code: temp_code.clone(),
        };
        let _ = app.emit("vault-update", loading_update);

        let temp_code = temp_code.clone();
        let task = tokio::spawn(async move {
            match client.archive_get(&data_map).await {
                Ok(archive) => {
                    let mut files: Vec<FileMetadata> = vec![];

                    for (filepath, (data_map, metadata)) in archive.map() {
                        files.push(FileMetadata {
                            path: filepath.display().to_string(),
                            metadata: metadata.clone(),
                            file_type: FileType::Private,
                            is_loaded: true,
                            archive_name: archive_name.clone(),
                            access_data: Some(FileAccess::Private(data_map.clone())),
                        });
                    }

                    let archive_loaded = ArchiveInfo {
                        name: archive_name.clone(),
                        address: data_map.to_hex(),
                        is_private: true,
                        files,
                    };

                    let update = VaultUpdate {
                        update_type: VaultUpdateType::ArchiveLoaded,
                        archive: Some(archive_loaded),
                        failed_archive: None,
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("vault-update", update);
                }
                Err(_) => {
                    let failed_archive = FailedArchive {
                        name: archive_name.clone(),
                        address: data_map.to_hex(),
                        is_private: true,
                    };

                    let update = VaultUpdate {
                        update_type: VaultUpdateType::ArchiveFailed,
                        archive: None,
                        failed_archive: Some(failed_archive),
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("vault-update", update);
                }
            }
        });

        archive_tasks.push(task);
    }

    // Create tasks for public archives
    for (archive_addr, name) in &user_data.file_archives {
        let client = client.clone();
        let app = app.clone();
        let archive_name = name.clone();
        let archive_addr = *archive_addr;

        // Emit loading status immediately
        let loading_update = VaultUpdate {
            update_type: VaultUpdateType::ArchiveLoading,
            archive: None,
            failed_archive: None,
            loading_archive: Some(LoadingArchive {
                name: archive_name.clone(),
                address: archive_addr.to_hex(),
                is_private: false,
            }),
            files: vec![],
            is_complete: false,
            temp_code: temp_code.clone(),
        };
        let _ = app.emit("vault-update", loading_update);

        let temp_code = temp_code.clone();
        let task = tokio::spawn(async move {
            match client.archive_get_public(&archive_addr).await {
                Ok(archive) => {
                    let mut files: Vec<FileMetadata> = vec![];

                    for (filepath, (data_addr, metadata)) in archive.map() {
                        files.push(FileMetadata {
                            path: filepath.display().to_string(),
                            metadata: metadata.clone(),
                            file_type: FileType::Public,
                            is_loaded: true,
                            archive_name: archive_name.clone(),
                            access_data: Some(FileAccess::Public(data_addr.clone())),
                        });
                    }

                    let archive_loaded = ArchiveInfo {
                        name: archive_name.clone(),
                        address: archive_addr.to_hex(),
                        is_private: false,
                        files,
                    };

                    let update = VaultUpdate {
                        update_type: VaultUpdateType::ArchiveLoaded,
                        archive: Some(archive_loaded),
                        failed_archive: None,
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("vault-update", update);
                }
                Err(_) => {
                    let failed_archive = FailedArchive {
                        name: archive_name.clone(),
                        address: archive_addr.to_hex(),
                        is_private: false,
                    };

                    let update = VaultUpdate {
                        update_type: VaultUpdateType::ArchiveFailed,
                        archive: None,
                        failed_archive: Some(failed_archive),
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("vault-update", update);
                }
            }
        });

        archive_tasks.push(task);
    }

    // Wait for all archive tasks to complete
    for task in archive_tasks {
        let _ = task.await;
    }

    // Finally, emit completion
    let completion_update = VaultUpdate {
        update_type: VaultUpdateType::Complete,
        archive: None,
        failed_archive: None,
        loading_archive: None,
        files: vec![],
        is_complete: true,
        temp_code: temp_code.clone(),
    };
    app.emit("vault-update", completion_update)
        .map_err(|_| VaultError::FileNotFound)?;

    Ok(())
}

pub async fn get_files_from_vault(
    secret_key: &VaultSecretKey,
    shared_client: State<'_, SharedClient>,
) -> Result<Vec<FileFromVault>, VaultError> {
    let client = shared_client.get_client().await?;

    // Fetch user data
    let user_data = client.vault_get_user_data(secret_key).await?;

    let mut files: Vec<FileFromVault> = vec![];

    // Add individual private files
    for (data_map, name) in &user_data.private_files {
        let file = FileFromVault::new(
            name.clone(),
            autonomi::files::Metadata::new_with_size(0),
            FileAccess::Private(data_map.clone()),
        );
        files.push(file);
    }

    // Add individual public files
    for (data_addr, name) in &user_data.public_files {
        let file = FileFromVault::new(
            name.clone(),
            autonomi::files::Metadata::new_with_size(0),
            FileAccess::Public(*data_addr),
        );
        files.push(file);
    }

    Ok(files)
}

// Utility: Ensure parent directories exist
fn ensure_parent_dir(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}

async fn download_private_file(
    data_map: &DataMapChunk,
    dest: PathBuf,
    client: &autonomi::Client,
) -> Result<(), DownloadError> {
    ensure_parent_dir(&dest);
    client.file_download(data_map, dest).await?;
    Ok(())
}

async fn download_private_archive(
    data_map: &DataMapChunk,
    dest: PathBuf,
    client: &autonomi::Client,
) -> Result<(), DownloadError> {
    let archive = client.archive_get(data_map).await?;
    let _ = std::fs::create_dir_all(&dest);

    for (file_path, (file_data_map, _)) in archive.map() {
        let full_path = dest.join(file_path);
        ensure_parent_dir(&full_path);
        client.file_download(file_data_map, full_path).await?;
    }
    Ok(())
}

pub async fn download_private(
    data_map: &DataMapChunk,
    dest: PathBuf,
    shared_client: State<'_, SharedClient>,
) -> Result<(), DownloadError> {
    let client = shared_client.get_client().await?;
    let hex_addr = data_map.to_hex();

    match client.analyze_address(&hex_addr, true).await {
        Ok(autonomi::client::analyze::Analysis::RawDataMap { .. })
        | Ok(autonomi::client::analyze::Analysis::DataMap { .. }) => {
            download_private_file(data_map, dest, &client).await
        }
        Ok(autonomi::client::analyze::Analysis::PrivateArchive { .. }) => {
            download_private_archive(data_map, dest, &client).await
        }
        Ok(_) => Err(DownloadError::Download(
            autonomi::client::files::DownloadError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported private data type",
            )),
        )),
        Err(e) => Err(DownloadError::Analysis(e)),
    }
}

async fn download_public_file(
    addr: &DataAddress,
    dest: PathBuf,
    client: &autonomi::Client,
) -> Result<(), DownloadError> {
    ensure_parent_dir(&dest);

    // Try streaming first, fallback to direct data if needed
    match client.file_download_public(addr, dest.clone()).await {
        Ok(()) => Ok(()),
        Err(_) => {
            let data = client.data_get_public(addr).await?;
            std::fs::write(&dest, data).map_err(|e| {
                DownloadError::Download(autonomi::client::files::DownloadError::IoError(e))
            })
        }
    }
}

async fn download_public_archive(
    addr: &DataAddress,
    dest: PathBuf,
    client: &autonomi::Client,
) -> Result<(), DownloadError> {
    use autonomi::files::PublicArchive;

    let data = client.data_get_public(addr).await?;
    let archive = PublicArchive::from_bytes(data).map_err(|e| {
        DownloadError::Download(autonomi::client::files::DownloadError::IoError(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Archive parse error: {e}"),
            ),
        ))
    })?;

    let _ = std::fs::create_dir_all(&dest);

    for (file_path, (file_addr, _)) in archive.map() {
        let full_path = dest.join(file_path);
        ensure_parent_dir(&full_path);
        client.file_download_public(file_addr, full_path).await?;
    }

    Ok(())
}

pub async fn download_public(
    addr: &DataAddress,
    dest: PathBuf,
    shared_client: State<'_, SharedClient>,
) -> Result<(), DownloadError> {
    let client = shared_client.get_client().await?;
    let hex_addr = addr.to_hex();

    match client.analyze_address(&hex_addr, false).await {
        Ok(autonomi::client::analyze::Analysis::RawDataMap { .. })
        | Ok(autonomi::client::analyze::Analysis::DataMap { .. }) => {
            download_public_file(addr, dest, &client).await
        }
        Ok(autonomi::client::analyze::Analysis::PublicArchive { .. }) => {
            download_public_archive(addr, dest, &client).await
        }
        Ok(_) => Err(DownloadError::Download(
            autonomi::client::files::DownloadError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported public data type",
            )),
        )),
        Err(e) => Err(DownloadError::Analysis(e)),
    }
}

pub async fn get_single_file_data(
    vault_key_signature: &str,
    file_path: &str,
    shared_client: State<'_, SharedClient>,
) -> Result<FileFromVault, VaultError> {
    let prefix = &format!("{}{}", "0", "x");
    let secret_key = vault_key_from_signature_hex(vault_key_signature.trim_start_matches(prefix))
        .expect(&format!("Invalid vault key {}", "signature"));
    let client = shared_client.get_client().await?;
    let user_data = client.vault_get_user_data(&secret_key).await?;

    // Try to find the file in individual private files
    for (data_map, name) in &user_data.private_files {
        if name == file_path {
            return Ok(FileFromVault::new(
                file_path.to_string(),
                autonomi::files::Metadata::new_with_size(0),
                FileAccess::Private(data_map.clone()),
            ));
        }
    }

    // Try to find the file in individual public files
    for (data_addr, name) in &user_data.public_files {
        if name == file_path {
            return Ok(FileFromVault::new(
                file_path.to_string(),
                autonomi::files::Metadata::new_with_size(0),
                FileAccess::Public(*data_addr),
            ));
        }
    }

    Err(VaultError::FileNotFound)
}

pub async fn remove_from_vault(
    secret_key: &VaultSecretKey,
    file_path: &str,
    archive_address: Option<String>,
    shared_client: State<'_, SharedClient>,
) -> Result<(), VaultError> {
    let client = shared_client.get_client().await?;

    // Get current user data from vault
    let mut user_data = client.vault_get_user_data(secret_key).await?;

    // If archive_address is provided, remove the entire archive
    if let Some(ref address) = archive_address {
        // Try to remove from private archives first
        let mut found = false;

        // Remove from private_file_archives by address
        user_data.private_file_archives.retain(|data_map, _name| {
            if data_map.to_hex() == *address {
                found = true;
                false // Remove this archive
            } else {
                true // Keep this archive
            }
        });

        // If not found in private archives, try public archives
        if !found {
            user_data.file_archives.retain(|data_addr, _name| {
                if data_addr.to_hex() == *address {
                    found = true;
                    false // Remove this archive
                } else {
                    true // Keep this archive
                }
            });
        }

        if !found {
            return Err(VaultError::FileNotFound);
        }
    } else {
        // Remove individual file by path
        let mut found = false;

        // Try to remove from private files first
        user_data.private_files.retain(|_data_map, name| {
            if name == file_path {
                found = true;
                false // Remove this file
            } else {
                true // Keep this file
            }
        });

        // If not found in private files, try public files
        if !found {
            user_data.public_files.retain(|_data_addr, name| {
                if name == file_path {
                    found = true;
                    false // Remove this file
                } else {
                    true // Keep this file
                }
            });
        }

        if !found {
            return Err(VaultError::FileNotFound);
        }
    }

    // Save the updated user data back to vault (vault updates are free)
    // Create an empty receipt for free vault operations
    let empty_store_quote = client
        .get_store_quotes(DataTypes::Chunk, std::iter::empty())
        .await
        .map_err(|_err| VaultError::FileNotFound)?;
    let receipt = autonomi::client::payment::receipt_from_store_quotes(empty_store_quote);

    client
        .vault_put_user_data(secret_key, receipt.into(), user_data)
        .await
        .map_err(|_err| VaultError::FileNotFound)?;
    Ok(())
}

/// Adds an item (file or archive) to the vault.
/// This is the unified implementation for both files and archives.
async fn add_item_to_vault_impl(
    client: &Client,
    secret_key: &VaultSecretKey,
    file_access: FileAccess,
    name: &str,
    is_archive: bool,
) -> Result<(), VaultError> {
    let item_type = if is_archive { "archive" } else { "file" };
    debug!(
        name = %name,
        access = ?file_access,
        item_type = %item_type,
        "add_item_to_vault"
    );

    // Get current user data from vault (or create new if vault doesn't exist)
    let mut user_data = get_or_create_user_data(client, secret_key).await?;

    // Handle based on access type and whether it's a file or archive
    match (&file_access, is_archive) {
        (FileAccess::Private(data_map), true) => {
            user_data
                .private_file_archives
                .insert(data_map.clone(), name.to_string());
            debug!("Added private archive to vault: {}", name);
        }
        (FileAccess::Public(data_addr), true) => {
            user_data.file_archives.insert(*data_addr, name.to_string());
            debug!("Added public archive to vault: {}", name);
        }
        (FileAccess::Private(data_map), false) => {
            user_data
                .private_files
                .insert(data_map.clone(), name.to_string());
            debug!("Added private file to vault: {}", name);
        }
        (FileAccess::Public(data_addr), false) => {
            user_data.public_files.insert(*data_addr, name.to_string());
            debug!("Added public file to vault: {}", name);
        }
    }

    // Save the updated user data back to vault
    client
        .vault_put_user_data(
            secret_key,
            PaymentOption::Receipt(Default::default()),
            user_data,
        )
        .await
        .map_err(|e| {
            error!("Failed to put user data to vault: {:?}", e);
            VaultError::FileNotFound
        })?;

    debug!("Successfully updated vault with new {}", item_type);
    Ok(())
}

pub async fn add_local_archive_to_vault(
    secret_key: &VaultSecretKey,
    archive_access: FileAccess,
    archive_name: &str,
    shared_client: State<'_, SharedClient>,
) -> Result<(), VaultError> {
    let client = shared_client.get_client().await?;
    add_item_to_vault_impl(&client, secret_key, archive_access, archive_name, true).await
}

pub async fn add_local_file_to_vault(
    secret_key: &VaultSecretKey,
    file_access: FileAccess,
    file_name: &str,
    shared_client: State<'_, SharedClient>,
) -> Result<(), VaultError> {
    let client = shared_client.get_client().await?;
    add_item_to_vault_impl(&client, secret_key, file_access, file_name, false).await
}
