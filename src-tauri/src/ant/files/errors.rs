//! Error types for file operations.

use autonomi::client::GetError;
use autonomi::client::merkle_payments::MerklePaymentError;
use autonomi::vault::user_data::UserDataVaultError;
use std::path::PathBuf;
use thiserror::Error as ThisError;

/// Errors that can occur during file upload.
#[derive(ThisError, Debug)]
pub enum UploadError {
    #[error("Could not connect to the network: {0:?}")]
    Connect(#[from] autonomi::client::ConnectError),
    #[error("Could not read file: {0:?}")]
    Read(PathBuf),
    #[error("Failed to encrypt data: {0}")]
    Encryption(String),
    #[error("Failed to retrieve store quotes: {0}")]
    StoreQuote(String),
    #[error("Failed to get or create scratchpad: {0}")]
    Scratchpad(String),
    #[error("Failed to emit payment order: {0}")]
    EmitEvent(String),
    #[error("Failed to serialize data: {0}")]
    Serialization(String),
    #[error("Failed to put data: {0}")]
    Put(String),
    #[error("Merkle payment preparation failed: {0}")]
    MerklePayment(#[from] MerklePaymentError),
}

/// Errors that can occur during vault operations.
#[derive(ThisError, Debug)]
pub enum VaultError {
    #[error("Could not connect to the network: {0:?}")]
    Connect(#[from] autonomi::client::ConnectError),
    #[error("Could not retrieve user data: {0:?}")]
    UserDataGet(#[from] UserDataVaultError),
    #[error("Could not retrieve data: {0:?}")]
    DataGet(#[from] GetError),
    #[error("File not found in vault")]
    FileNotFound,
}

/// Errors that can occur during file download.
#[derive(ThisError, Debug)]
pub enum DownloadError {
    #[error("Could not connect to the network: {0:?}")]
    Connect(#[from] autonomi::client::ConnectError),
    #[error("Could not download file: {0:?}")]
    Download(#[from] autonomi::client::files::DownloadError),
    #[error("Could not get data: {0:?}")]
    Get(#[from] autonomi::client::GetError),
    #[error("Could not analyze address: {0:?}")]
    Analysis(#[from] autonomi::client::analyze::AnalysisError),
}
