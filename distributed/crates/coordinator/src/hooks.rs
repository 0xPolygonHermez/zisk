use anyhow::Result;
use zisk_cluster_common::{
    dto::{WebhookErrorDto, WebhookPayloadDto},
    JobId,
};
use zisk_common::RandomSeed;

/// Sends a webhook notification upon job completion
///
/// # Arguments
///
/// * `webhook_url` - The URL to send the webhook to. It can contain a placeholder `{$job_id}`
///   which will be replaced with the actual job ID. If the placeholder is not present, the job ID
///   will be appended to the URL as a path segment.
/// * `job_id` - The ID of the job that has completed or failed.
/// * `duration_ms` - Duration of the job in milliseconds.
/// * `proof_data` - Optional proof to include in the payload, as `Proof::to_bytes_without_seed_trailer`.
/// * `seed` - The job's host randomness seed, if it ran one.
pub async fn send_completion_webhook(
    webhook_url: String,
    job_id: JobId,
    duration_ms: u64,
    executed_steps: Option<u64>,
    proof_data: Option<Vec<u8>>,
    seed: Option<RandomSeed>,
) -> Result<()> {
    send_webhook(webhook_url, job_id, duration_ms, executed_steps, proof_data, seed, None).await
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
/// * `seed` - The job's host randomness seed, if it ran one, to replay the failure.
pub async fn send_failure_webhook(
    webhook_url: String,
    job_id: JobId,
    duration_ms: u64,
    error_code: String,
    error_message: String,
    seed: Option<RandomSeed>,
) -> Result<()> {
    let error = WebhookErrorDto { code: error_code, message: error_message };
    send_webhook(webhook_url, job_id, duration_ms, Some(0), None, seed, Some(error)).await
}

/// Internal function to send webhook notifications with optional error details.
async fn send_webhook(
    webhook_url: String,
    job_id: JobId,
    duration_ms: u64,
    executed_steps: Option<u64>,
    proof_data: Option<Vec<u8>>,
    seed: Option<RandomSeed>,
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

    let mut payload = if let Some(error) = error {
        WebhookPayloadDto::failure(job_id.as_string(), duration_ms, error)
    } else {
        WebhookPayloadDto::success(job_id.as_string(), duration_ms, executed_steps, proof_data)
    };
    payload.random_seed = seed.map(|s| s.to_string());

    let response = client
        .post(&webhook_url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?;

    // This handles HTTP response status codes.
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Webhook returned non-success status {} for {}: {}",
            response.status(),
            job_id,
            response.text().await.unwrap_or_default()
        ));
    }

    Ok(())
}
