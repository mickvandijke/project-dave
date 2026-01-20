//! Type definitions for file operations.

use autonomi::chunk::DataMapChunk;
use autonomi::data::DataAddress;
use autonomi::files::Metadata;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a file to be uploaded.
#[derive(Deserialize, Clone)]
pub struct File {
    pub name: String,
    pub path: PathBuf,
}

/// Progress updates for file uploads.
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum UploadProgress {
    Started {
        upload_id: String,
        total_files: usize,
        total_size: u64,
    },
    #[allow(dead_code)]
    Processing {
        upload_id: String,
        current_file: String,
        files_processed: usize,
        total_files: usize,
        bytes_processed: u64,
        total_bytes: u64,
    },
    #[allow(dead_code)]
    Encrypting {
        upload_id: String,
        current_file: String,
        files_processed: usize,
        total_files: usize,
    },
    #[allow(dead_code)]
    RequestingPayment {
        upload_id: String,
        files_processed: usize,
        total_files: usize,
    },
    Uploading {
        upload_id: String,
        chunks_uploaded: usize,
        total_chunks: usize,
        bytes_uploaded: u64,
        total_bytes: u64,
    },
    Completed {
        upload_id: String,
        total_files: usize,
        total_bytes: u64,
        add_to_vault: bool,
        file_access: Option<FileAccess>,
    },
    Failed {
        upload_id: String,
        error: String,
    },
    #[allow(dead_code)]
    Cancelled {
        upload_id: String,
    },
}

/// A file retrieved from the vault.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileFromVault {
    path: String,
    metadata: Metadata,
    file_access: FileAccess,
}

impl FileFromVault {
    pub fn new(path: String, metadata: Metadata, file_access: FileAccess) -> Self {
        Self {
            path,
            metadata,
            file_access,
        }
    }
}

/// Information about a failed archive load.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FailedArchive {
    pub name: String,
    pub address: String,
    pub is_private: bool,
}

/// Structure representing the user's vault contents.
#[derive(Debug, Serialize, Deserialize)]
pub struct VaultStructure {
    pub archives: Vec<ArchiveInfo>,
    pub failed_archives: Vec<FailedArchive>,
    pub files: Vec<FileMetadata>,
}

/// Update information for vault streaming operations.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultUpdate {
    pub update_type: VaultUpdateType,
    pub archive: Option<ArchiveInfo>,
    pub failed_archive: Option<FailedArchive>,
    pub loading_archive: Option<LoadingArchive>,
    pub files: Vec<FileMetadata>,
    pub is_complete: bool,
    pub temp_code: String,
}

/// Information about an archive currently being loaded.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoadingArchive {
    pub name: String,
    pub address: String,
    pub is_private: bool,
}

/// Types of vault update events.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VaultUpdateType {
    IndividualFiles,
    ArchiveLoading,
    ArchiveLoaded,
    ArchiveFailed,
    Complete,
}

/// Information about an archive in the vault.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchiveInfo {
    pub name: String,
    pub address: String,
    pub is_private: bool,
    pub files: Vec<FileMetadata>,
}

/// Metadata for a file in the vault.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub metadata: Metadata,
    pub file_type: FileType,
    pub is_loaded: bool,
    pub archive_name: String,
    pub access_data: Option<FileAccess>,
}

/// Type of file access (public or private).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FileType {
    Public,
    Private,
}

/// Access method for a file (public address or private data map).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FileAccess {
    Public(DataAddress),
    Private(DataMapChunk),
}
