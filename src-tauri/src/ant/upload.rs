use crate::ant::stream::MAX_CHUNKS_PER_BATCH;
use autonomi::client::merkle_payments::MerklePaymentReceipt;
use autonomi::client::payment::Receipt;
use autonomi::self_encryption::EncryptionStream;
use autonomi::Client;
use autonomi::XorName;
use std::collections::HashSet;

#[derive(Debug)]
pub(crate) enum UploadError {
    #[allow(dead_code)]
    Put(String),
    #[allow(dead_code)]
    MerklePut(String),
}

pub(crate) async fn batch_upload_encryption_stream(
    client: &Client,
    receipt: &Receipt,
    encryption_stream: &mut EncryptionStream,
) -> Result<(), UploadError> {
    while let Some(next_batch) = encryption_stream.next_batch(MAX_CHUNKS_PER_BATCH) {
        client
            .chunk_batch_upload(next_batch.iter().collect(), receipt)
            .await
            .map_err(|err| UploadError::Put(err.to_string()))?;
    }

    Ok(())
}

/// Upload a single encryption stream using merkle payment proofs.
/// Note: This function takes ownership of the stream.
pub(crate) async fn batch_upload_encryption_stream_merkle(
    client: &Client,
    receipt: &MerklePaymentReceipt,
    encryption_stream: EncryptionStream,
) -> Result<(), UploadError> {
    // Track chunks we've already uploaded to avoid duplicates
    let mut dont_reupload: HashSet<XorName> = HashSet::new();

    // Use upload_batch_with_merkle for the stream
    let streams = vec![encryption_stream];
    let result = client
        .upload_batch_with_merkle(streams, receipt, &mut dont_reupload, usize::MAX)
        .await
        .map_err(|err| UploadError::MerklePut(err.to_string()))?;

    // Check for failed chunks
    if !result.failed_chunks.is_empty() {
        return Err(UploadError::MerklePut(format!(
            "{} chunks failed to upload",
            result.failed_chunks.len()
        )));
    }

    Ok(())
}

/// Upload multiple encryption streams using merkle payment proofs.
pub(crate) async fn batch_upload_encryption_streams_merkle(
    client: &Client,
    receipt: &MerklePaymentReceipt,
    streams: Vec<EncryptionStream>,
) -> Result<(), UploadError> {
    // Track chunks we've already uploaded to avoid duplicates
    let mut dont_reupload: HashSet<XorName> = HashSet::new();

    // Use upload_batch_with_merkle for all streams
    let result = client
        .upload_batch_with_merkle(streams, receipt, &mut dont_reupload, usize::MAX)
        .await
        .map_err(|err| UploadError::MerklePut(err.to_string()))?;

    // Check for failed chunks
    if !result.failed_chunks.is_empty() {
        return Err(UploadError::MerklePut(format!(
            "{} chunks failed to upload",
            result.failed_chunks.len()
        )));
    }

    Ok(())
}
