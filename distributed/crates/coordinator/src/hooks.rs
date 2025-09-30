use anyhow::Result;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info, warn};
use zisk_distributed_common::{
    dto::{WebhookErrorDto, WebhookPayloadDto},
    JobId,
};
use zstd::Encoder;

use crate::coordinator_errors::{CoordinatorError, CoordinatorResult};

/// Sends a webhook notification upon job completion or failure.
///
/// # Arguments
///
/// * `webhook_url` - The URL to send the webhook to. It can contain a placeholder `{$job_id}`
///   which will be replaced with the actual job ID.
/// * `job_id` - The ID of the job that has completed or failed.
/// * `duration_ms` - Duration of the job in milliseconds.
/// * `proof_data` - Optional proof data to include in the webhook payload.
/// * `success` - A boolean indicating whether the job completed successfully or failed.
pub async fn send_completion_webhook(
    webhook_url: String,
    job_id: JobId,
    duration_ms: u64,
    proof_data: Option<Vec<u64>>,
    success: bool,
) -> Result<()> {
    send_webhook_with_error(webhook_url, job_id, duration_ms, proof_data, success, None).await
}

/// Sends a webhook notification upon job failure with error details.
///
/// # Arguments
///
/// * `webhook_url` - The URL to send the webhook to.
/// * `job_id` - The ID of the job that has failed.
/// * `duration_ms` - Duration of the job in milliseconds.
/// * `error_code` - Error code representing the type of failure.
/// * `error_message` - Human-readable error message.
pub async fn _send_failure_webhook(
    webhook_url: String,
    job_id: JobId,
    duration_ms: u64,
    error_code: String,
    error_message: String,
) -> Result<()> {
    let error = WebhookErrorDto { code: error_code, message: error_message };
    send_webhook_with_error(webhook_url, job_id, duration_ms, None, false, Some(error)).await
}

/// Internal function to send webhook notifications with optional error details.
async fn send_webhook_with_error(
    webhook_url: String,
    job_id: JobId,
    duration_ms: u64,
    proof_data: Option<Vec<u64>>,
    _success: bool, // Determined by presence of error
    error: Option<WebhookErrorDto>,
) -> Result<()> {
    let client = reqwest::Client::new();

    // Formats the webhook URL based on the presence of a job ID placeholder:
    // - If the URL contains `{$job_id}`, the placeholder is replaced with the actual job ID.
    // - If no placeholder is found, the job ID is appended to the URL as a path segment.

    let webhook_url = if webhook_url.contains("{$job_id}") {
        webhook_url.replace("{$job_id}", job_id.as_str())
    } else {
        format!("{}/{}", webhook_url, job_id.as_str())
    };

    let payload = if let Some(error) = error {
        WebhookPayloadDto::failure(job_id.as_string(), duration_ms, error)
    } else {
        WebhookPayloadDto::success(job_id.as_string(), duration_ms, proof_data)
    };

    let response = match client
        .post(&webhook_url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            // This handles connection errors, timeouts, DNS resolution failures, etc.
            error!("Failed to send webhook request to {}: {}", webhook_url, e);
            return Err(e.into());
        }
    };

    if response.status().is_success() {
        info!("Successfully sent webhook notification for {} to {}", job_id, webhook_url);
    } else {
        warn!(
            "Webhook returned non-success status {} for {}: {}",
            response.status(),
            job_id,
            response.text().await.unwrap_or_default()
        );
    }

    Ok(())
}

/// Saves the final proof data to disk with unique filename generation.
///
/// Creates a unique filename to avoid overwriting existing proof files by appending
/// a counter suffix (_2, _3, etc.) if the initial filename already exists.
///
/// # Arguments
///
/// * `job_id` - The ID of the job whose proof is being saved
/// * `proof_data` - The proof data as a vector of u64 values
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `CoordinatorError` on failure
pub async fn save_proof(
    id: &str,
    proof_folder: PathBuf,
    proof_data: &[u64],
) -> CoordinatorResult<()> {
    // Ensure the proofs directory exists
    fs::create_dir_all(&proof_folder).await.map_err(|e| {
        error!("Failed to create proofs directory: {}", e);
        CoordinatorError::Internal(e.to_string())
    })?;

    // Generate unique filename to avoid overwriting existing files
    let mut raw_path = proof_folder.join(format!("proof_{}.fri", id));
    let mut zip_path = raw_path.with_extension("fri.compressed");
    let mut counter = 2;

    while fs::try_exists(&raw_path).await.map_err(|e| {
        error!("Failed to check proof file existence: {}", e);
        CoordinatorError::Internal(e.to_string())
    })? || fs::try_exists(&zip_path).await.map_err(|e| {
        error!("Failed to check compressed file existence: {}", e);
        CoordinatorError::Internal(e.to_string())
    })? {
        raw_path = proof_folder.join(format!("proof_{}_{}.fri", id, counter));
        zip_path = raw_path.with_extension("fri.compressed");
        counter += 1;
    }

    // Convert Vec<u64> to bytes safely
    let proof_bytes = bytemuck::cast_slice::<u64, u8>(proof_data);

    // Write raw proof file
    fs::write(&raw_path, proof_bytes).await.map_err(|e| {
        error!("Failed to write proof file: {}", e);
        CoordinatorError::Internal(e.to_string())
    })?;

    // Compress proof data and write to file
    let zip_size = save_zip_proof(proof_bytes, &zip_path, 1).await?;

    // Calculate compression statistics
    let raw_size = proof_bytes.len();
    let ratio = zip_size as f64 / raw_size as f64;

    info!("Final proof compression completed:");
    info!("  Raw: {} ({} bytes)", raw_path.display(), raw_size);
    info!("  Compressed: {} ({} bytes, ratio: {:.2}x)", zip_path.display(), zip_size, ratio);

    Ok(())
}

/// Compresses data using zstd and writes it to a file.
///
/// # Arguments
///
/// * `data` - The raw data to compress
/// * `output_path` - Path where the compressed file will be written
/// * `compression_level` - Compression level (1 = fastest, 22 = best compression)
///
/// # Returns
///
/// Returns the compressed size in bytes, or a `CoordinatorError` on failure
async fn save_zip_proof(
    data: &[u8],
    zip_path: &std::path::Path,
    compression_level: i32,
) -> CoordinatorResult<usize> {
    // Compress data in memory using zstd
    let mut compressed_buffer = Cursor::new(Vec::new());
    {
        let mut encoder = Encoder::new(&mut compressed_buffer, compression_level).map_err(|e| {
            error!("Failed to create zstd encoder: {}", e);
            CoordinatorError::Internal(e.to_string())
        })?;

        encoder.write_all(data).map_err(|e| {
            error!("Failed to write data to compressor: {}", e);
            CoordinatorError::Internal(e.to_string())
        })?;

        encoder.finish().map_err(|e| {
            error!("Failed to finish compression: {}", e);
            CoordinatorError::Internal(e.to_string())
        })?;
    }

    // Extract compressed data and get size
    let compressed_data = compressed_buffer.into_inner();
    let compressed_size = compressed_data.len();

    // Write compressed data to file
    fs::write(zip_path, &compressed_data).await.map_err(|e| {
        error!("Failed to write compressed file: {}", e);
        CoordinatorError::Internal(e.to_string())
    })?;

    Ok(compressed_size)
}
