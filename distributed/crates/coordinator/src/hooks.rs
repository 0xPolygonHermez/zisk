use anyhow::Result;
use distributed_common::JobId;
use tracing::{error, info, warn};

/// Sends a webhook notification upon job completion or failure.
///
/// # Arguments
///
/// * `webhook_url` - The URL to send the webhook to. It can contain a placeholder `{$job_id}`
///   which will be replaced with the actual job ID.
/// * `job_id` - The ID of the job that has completed or failed.
/// * `proof_data` - Optional proof data to include in the webhook payload.
/// * `success` - A boolean indicating whether the job completed successfully or failed.
pub async fn send_completion_webhook(
    webhook_url: String,
    job_id: JobId,
    proof_data: Option<Vec<u64>>,
    success: bool,
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

    let payload = serde_json::json!({
        "job_id": job_id.as_string(),
        "status": if success { "completed" } else { "failed" },
        "proof": proof_data,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

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
