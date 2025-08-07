use crate::{Error, Result};
use consensus_api::{
    coordinator_message, prover_message, CoordinatorMessage, ProverCapabilities, ProverMessage,
};

use chrono::{DateTime, Utc};
use std::{collections::HashMap, fmt::Display, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Job ID wrapper for type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobId(pub String);

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl JobId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for JobId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<JobId> for String {
    fn from(job_id: JobId) -> Self {
        job_id.0
    }
}

impl Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JobId({})", self.0)
    }
}

/// Configuration for the coordinator functionality
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    pub max_provers_per_job: u32,
    pub max_total_provers: u32,
    pub max_concurrent_connections: u32,
    pub message_buffer_size: u32,
    pub phase1_timeout_seconds: u64,
    pub phase2_timeout_seconds: u64,
}

impl CoordinatorConfig {
    /// Create from consensus_config::CoordinatorConfig
    pub fn from_config(config: &consensus_config::ProverManagerConfig) -> Self {
        Self {
            max_provers_per_job: config.max_provers_per_job,
            max_total_provers: config.max_total_provers,
            max_concurrent_connections: config.max_concurrent_connections,
            message_buffer_size: config.message_buffer_size,
            phase1_timeout_seconds: config.phase1_timeout_seconds,
            phase2_timeout_seconds: config.phase2_timeout_seconds,
        }
    }
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            max_provers_per_job: 10,
            max_total_provers: 1000, // Default limit of 1000 total provers
            max_concurrent_connections: 500, // Default limit of 500 concurrent connections
            message_buffer_size: 1000, // Default bounded channel size
            phase1_timeout_seconds: 300, // 5 minutes
            phase2_timeout_seconds: 600, // 10 minutes
        }
    }
}

/// Result types for coordinator operations
#[derive(Debug, Clone)]
pub struct ProverRegistrationResult {
    pub accepted: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct JobStartResult {
    pub job_id: String,
}

#[derive(Debug, Clone)]
pub struct Phase1Result {
    pub job_id: String,
    pub prover_id: String,
    pub rank_id: u32,
    pub result_data: Vec<u64>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FinalProofResult {
    pub job_id: String,
    pub prover_id: String,
    pub rank_id: u32,
    pub proof_data: Vec<u64>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Status of a connected prover
#[derive(Debug, Clone)]
pub enum ProverStatus {
    Idle,
    Working { job_id: JobId, rank_id: u32 },
    Error { message: String },
}

/// Information about a connected prover - business logic only, no transport layer
#[derive(Debug)]
pub struct ProverConnection {
    pub id: String,
    pub capabilities: ProverCapabilities,
    pub status: ProverStatus,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub message_sender: mpsc::Sender<CoordinatorMessage>,
}

/// Job assignment for coordination
#[derive(Debug, Clone)]
pub struct Job {
    pub job_id: JobId,
    pub block_id: u64,
    pub required_provers: usize,
    pub assigned_provers: Vec<String>,
    pub status: JobStatus,
    pub phase1_results: HashMap<String, ProvePhase1Result>, // prover_id -> result
}

/// Phase1 result data for internal tracking
#[derive(Debug, Clone)]
pub struct ProvePhase1Result {
    pub prover_id: String,
    pub rank_id: u32,
    pub result_data: Vec<u64>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum JobStatus {
    Pending,
    InProgress,
    Phase1Complete,
    Phase2InProgress,
    Completed,
    Failed { reason: String },
}

/// Centralized prover manager for handling multiple provers
#[derive(Debug)]
pub struct ProverManager {
    provers: Arc<RwLock<HashMap<String, ProverConnection>>>,
    jobs: Arc<RwLock<HashMap<JobId, Job>>>,
    config: CoordinatorConfig,
}

impl ProverManager {
    pub fn new(config: CoordinatorConfig) -> Self {
        Self {
            provers: Arc::new(RwLock::new(HashMap::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register a new prover connection - business logic only
    pub async fn register_prover(
        &self,
        prover_id: String,
        capabilities: ProverCapabilities,
        message_sender: mpsc::Sender<CoordinatorMessage>,
    ) -> Result<ProverRegistrationResult> {
        // Check if we've reached the maximum number of total provers
        let provers_guard = self.provers.read().await;
        let current_count = provers_guard.len();
        drop(provers_guard);

        if current_count >= self.config.max_total_provers as usize {
            return Ok(ProverRegistrationResult {
                accepted: false,
                message: format!(
                    "Maximum number of provers reached: {}/{}",
                    current_count, self.config.max_total_provers
                ),
            });
        }

        let now = Utc::now();
        let connection = ProverConnection {
            id: prover_id.clone(),
            capabilities,
            status: ProverStatus::Idle,
            connected_at: now,
            last_heartbeat: now,
            message_sender,
        };

        let mut provers = self.provers.write().await;
        provers.insert(prover_id.clone(), connection);
        info!("Registered prover: {} (total: {})", prover_id, provers.len());

        Ok(ProverRegistrationResult {
            accepted: true,
            message: "Successfully registered".to_string(),
        })
    }

    /// Remove a prover connection
    pub async fn disconnect_prover(&self, prover_id: &str) -> Result<()> {
        let mut provers = self.provers.write().await;
        if let Some(prover) = provers.remove(prover_id) {
            info!(
                "Removed prover: {} (remaining: {}, was in state: {:?})",
                prover_id,
                provers.len(),
                prover.status
            );
            // TODO: Handle job reassignment if prover was working
        } else {
            warn!("Attempted to remove prover {} but it was not found", prover_id);
        }
        Ok(())
    }

    /// Get list of available provers
    pub async fn get_available_provers(&self) -> Vec<String> {
        let provers = self.provers.read().await;
        provers
            .values()
            .filter(|p| matches!(p.status, ProverStatus::Idle))
            .map(|p| p.id.clone())
            .collect()
    }

    /// Start a proof job with the specified request
    pub async fn start_proof(
        &self,
        block_id: u64,
        num_provers: u32,
        input_path: String,
    ) -> Result<JobId> {
        info!(
            "Starting proof for block {} with {} provers, input: {}",
            block_id, num_provers, input_path
        );

        // Validate prover count using configuration
        let max_provers_per_job = self.config().max_provers_per_job;
        if num_provers > max_provers_per_job {
            return Err(Error::InvalidRequest(format!(
                "Requested {num_provers} provers exceeds maximum of {max_provers_per_job}"
            )));
        }

        // Create job with a generated ID
        let job_id = JobId::new();
        let job = Job {
            job_id: job_id.clone(),
            block_id,
            required_provers: num_provers as usize,
            assigned_provers: Vec::new(),
            status: JobStatus::Pending,
            phase1_results: HashMap::new(),
        };

        let available_provers = self.get_available_provers().await;

        if available_provers.len() < job.required_provers {
            return Err(Error::InvalidRequest(format!(
                "Not enough provers available: need {}, have {}",
                job.required_provers,
                available_provers.len()
            )));
        }

        // Select only the required number of provers (not all of them)
        let selected_provers: Vec<String> =
            available_provers.into_iter().take(job.required_provers).collect();

        // Update job with assigned provers
        let mut jobs = self.jobs.write().await;
        let mut updated_job = job;
        updated_job.assigned_provers = selected_provers.clone();
        updated_job.status = JobStatus::InProgress;
        jobs.insert(updated_job.job_id.clone(), updated_job.clone());

        // Send messages to selected provers
        let mut provers = self.provers.write().await;
        for (rank, prover_id) in selected_provers.iter().enumerate() {
            if let Some(prover) = provers.get_mut(prover_id) {
                prover.status = ProverStatus::Working {
                    job_id: updated_job.job_id.clone(),
                    rank_id: rank as u32,
                };

                let message = CoordinatorMessage {
                    payload: Some(coordinator_message::Payload::ProvePhase1(
                        consensus_api::ProvePhase1 {
                            job_id: updated_job.job_id.as_string(),
                            block_id: updated_job.block_id,
                            rank_id: rank as u32,
                            total_provers: updated_job.required_provers as u32,
                        },
                    )),
                };

                // Send message through the prover's channel (bounded channels require async send)
                match prover.message_sender.try_send(message) {
                    Ok(()) => {
                        info!("Sent ProvePhase1 message to prover {} (rank {})", prover_id, rank);
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        warn!("Message buffer full for prover {}, dropping message", prover_id);
                        // TODO: Handle backpressure - maybe queue for retry or mark prover as slow
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        error!("Channel closed for prover {}", prover_id);
                        // TODO: Handle closed channel - maybe reassign job
                    }
                }
            }
        }

        info!(
            "Assigned job {} to {} provers with input path: {}",
            updated_job.job_id,
            selected_provers.len(),
            input_path
        );

        Ok(job_id)
    }

    /// Get all connected prover IDs for broadcasting - transport layer handles actual sending
    pub async fn get_all_prover_ids(&self) -> Vec<String> {
        let provers = self.provers.read().await;
        provers.keys().cloned().collect()
    }

    /// Handle incoming message from a prover
    pub async fn handle_prover_message(
        &self,
        prover_id: &str,
        message: ProverMessage,
    ) -> Result<Option<CoordinatorMessage>> {
        // Update last heartbeat
        if let Some(prover) = self.provers.write().await.get_mut(prover_id) {
            prover.last_heartbeat = Utc::now();
        }

        // Handle specific message types
        if let Some(payload) = message.payload {
            match payload {
                prover_message::Payload::Phase1Result(phase1_result) => {
                    info!(
                        "Phase 1 result from prover {}: {} (job: {})",
                        prover_id, phase1_result.success, phase1_result.job_id
                    );

                    // Convert job_id string back to JobId for lookup
                    let job_id = JobId::from(phase1_result.job_id.clone());

                    // Store the Phase1 result and check if we can proceed to Phase2
                    if let Err(e) = self.handle_phase1_result(&job_id, phase1_result).await {
                        error!("Failed to handle Phase1 result: {}", e);
                    }
                }
                prover_message::Payload::FinalProof(final_proof) => {
                    info!("Final proof from prover {}: {}", prover_id, final_proof.success);

                    // Convert job_id string back to JobId for completion
                    let job_id = JobId::from(final_proof.job_id.clone());

                    if final_proof.success {
                        // Mark job as complete and free provers
                        if let Err(e) = self.complete_job(&job_id).await {
                            error!("Failed to complete job {}: {}", job_id, e);
                        }
                    } else {
                        // Mark job as failed and free provers
                        if let Err(e) = self
                            .fail_job(&job_id, "Final proof generation failed".to_string())
                            .await
                        {
                            error!("Failed to mark job {} as failed: {}", job_id, e);
                        }
                    }
                }
                prover_message::Payload::Error(prover_error) => {
                    error!("Prover {} error: {}", prover_id, prover_error.error_message);

                    // If the error includes a job_id, we should fail that job
                    if !prover_error.job_id.is_empty() {
                        let job_id = JobId::from(prover_error.job_id.clone());
                        if let Err(e) =
                            self.fail_job(&job_id, prover_error.error_message.clone()).await
                        {
                            error!(
                                "Failed to mark job {} as failed after prover error: {}",
                                job_id, e
                            );
                        }
                    }
                }
                prover_message::Payload::HeartbeatAck(_) => {
                    // Already updated heartbeat above
                }
                _ => {
                    // Other message types handled elsewhere
                }
            }
        }

        Ok(None)
    }

    /// Access to provers for direct manipulation (used by ConsensusService)
    pub fn provers(&self) -> &Arc<RwLock<HashMap<String, ProverConnection>>> {
        &self.provers
    }

    /// Get the coordinator configuration
    pub fn config(&self) -> &CoordinatorConfig {
        &self.config
    }

    /// Get current prover count
    pub async fn get_prover_count(&self) -> usize {
        self.provers.read().await.len()
    }

    /// Complete a job and reset all assigned provers back to Idle status
    pub async fn complete_job(&self, job_id: &JobId) -> Result<()> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get(job_id) {
            let prover_ids = job.assigned_provers.clone();

            // Remove the job from tracking
            jobs.remove(job_id);
            drop(jobs); // Release the jobs lock early

            // Reset prover statuses back to Idle
            let mut provers = self.provers.write().await;
            for prover_id in &prover_ids {
                if let Some(prover) = provers.get_mut(prover_id) {
                    if matches!(prover.status, ProverStatus::Working { job_id: ref working_job_id, .. } if working_job_id == job_id)
                    {
                        prover.status = ProverStatus::Idle;
                        info!(
                            "Reset prover {} back to Idle after completing job {}",
                            prover_id, job_id
                        );
                    }
                }
            }

            info!("Completed job {} and freed {} provers", job_id, prover_ids.len());
            Ok(())
        } else {
            warn!("Attempted to complete job {job_id} but it was not found");
            Err(Error::InvalidRequest(format!("Job {job_id} not found")))
        }
    }

    /// Get all provers assigned to a specific job
    pub async fn get_job_provers(&self, job_id: &JobId) -> Option<Vec<String>> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).map(|job| job.assigned_provers.clone())
    }

    /// Mark a job as failed and reset prover statuses
    pub async fn fail_job(&self, job_id: &JobId, reason: String) -> Result<()> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            let prover_ids = job.assigned_provers.clone();

            // Update job status to failed
            job.status = JobStatus::Failed { reason: reason.clone() };
            drop(jobs); // Release the jobs lock early

            // Reset prover statuses back to Idle
            let mut provers = self.provers.write().await;
            for prover_id in &prover_ids {
                if let Some(prover) = provers.get_mut(prover_id) {
                    if matches!(prover.status, ProverStatus::Working { job_id: ref working_job_id, .. } if working_job_id == job_id)
                    {
                        prover.status = ProverStatus::Idle;
                        info!(
                            "Reset prover {} back to Idle after job {} failed",
                            prover_id, job_id
                        );
                    }
                }
            }

            error!(
                "Failed job {} (reason: {}) and freed {} provers",
                job_id,
                reason,
                prover_ids.len()
            );
            Ok(())
        } else {
            warn!("Attempted to fail job {job_id} but it was not found");
            Err(Error::InvalidRequest(format!("Job {job_id} not found")))
        }
    }

    /// Handle Phase1 result and check if we can proceed to Phase2
    pub async fn handle_phase1_result(
        &self,
        job_id: &JobId,
        phase1_result: consensus_api::ProvePhase1Result,
    ) -> Result<()> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            // Store the Phase1 result
            let internal_result = ProvePhase1Result {
                prover_id: phase1_result.prover_id.clone(),
                rank_id: phase1_result.rank_id,
                result_data: phase1_result.result_data,
                success: phase1_result.success,
                error_message: if phase1_result.error_message.is_empty() {
                    None
                } else {
                    Some(phase1_result.error_message)
                },
            };

            job.phase1_results.insert(phase1_result.prover_id.clone(), internal_result);

            info!("Stored Phase1 result for prover {} in job {}.", phase1_result.prover_id, job_id,);

            // Check if we have all Phase1 results
            if job.phase1_results.len() == job.required_provers {
                // Check if all results are successful
                let all_successful = job.phase1_results.values().all(|result| result.success);

                if all_successful {
                    info!("All Phase1 results successful for job {}. Starting Phase2", job_id);
                    job.status = JobStatus::Phase1Complete;

                    // Get the assigned provers and release the jobs lock
                    let assigned_provers = job.assigned_provers.clone();
                    let job_id_clone = job.job_id.clone();
                    drop(jobs); // Release jobs lock early

                    // Start Phase2 for all provers
                    if let Err(e) = self.start_phase2(&job_id_clone, &assigned_provers).await {
                        error!("Failed to start Phase2 for job {}: {}", job_id_clone, e);
                        // Mark job as failed
                        if let Err(fail_err) = self
                            .fail_job(&job_id_clone, format!("Failed to start Phase2: {e}"))
                            .await
                        {
                            error!("Failed to mark job {} as failed: {}", job_id_clone, fail_err);
                        }
                    }
                } else {
                    // Some Phase1 results failed
                    let failed_provers: Vec<String> =
                        job.phase1_results
                            .iter()
                            .filter_map(|(prover_id, result)| {
                                if !result.success {
                                    Some(prover_id.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();

                    warn!("Phase1 failed for provers {:?} in job {}", failed_provers, job_id);
                    let reason = format!("Phase1 failed for provers: {failed_provers:?}");

                    // Release jobs lock before calling fail_job
                    let job_id_clone = job.job_id.clone();
                    drop(jobs);

                    if let Err(e) = self.fail_job(&job_id_clone, reason).await {
                        error!("Failed to mark job {} as failed: {}", job_id_clone, e);
                    }
                }
            }

            Ok(())
        } else {
            warn!("Received Phase1 result for unknown job: {job_id}");
            Err(Error::InvalidRequest(format!("Job {job_id} not found")))
        }
    }

    /// Start Phase2 for all provers that completed Phase1
    async fn start_phase2(&self, job_id: &JobId, assigned_provers: &[String]) -> Result<()> {
        info!("Starting Phase2 for job {} with {} provers", job_id, assigned_provers.len());

        // Update job status to Phase2InProgress
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = JobStatus::Phase2InProgress;
            }
        }

        // Generate global challenge for Phase2 (this would typically be derived from Phase1 results)
        let global_challenge = self.generate_global_challenge(job_id).await?;

        // Update prover statuses and send Phase2 messages
        let mut provers = self.provers.write().await;
        for prover_id in assigned_provers {
            if let Some(prover) = provers.get_mut(prover_id) {
                // Prover should still be in Working status from Phase1
                if matches!(prover.status, ProverStatus::Working { job_id: ref working_job_id, .. } if working_job_id == job_id)
                {
                    let message = CoordinatorMessage {
                        payload: Some(coordinator_message::Payload::ProvePhase2(
                            consensus_api::ProvePhase2 {
                                job_id: job_id.as_string(),
                                global_challenge: global_challenge.clone(),
                            },
                        )),
                    };

                    // Send Phase2 message
                    match prover.message_sender.try_send(message) {
                        Ok(()) => {
                            info!("Sent ProvePhase2 message to prover {}", prover_id);
                        }
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            warn!(
                                "Message buffer full for prover {}, dropping Phase2 message",
                                prover_id
                            );
                            return Err(Error::InvalidRequest(format!(
                                "Failed to send Phase2 message to prover {prover_id}: buffer full"
                            )));
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            error!("Channel closed for prover {}", prover_id);
                            return Err(Error::InvalidRequest(format!(
                                "Failed to send Phase2 message to prover {prover_id}: channel closed"
                            )));
                        }
                    }
                } else {
                    warn!("Prover {} is not in working state for job {}", prover_id, job_id);
                }
            } else {
                warn!("Prover {} not found when starting Phase2", prover_id);
            }
        }

        info!(
            "Successfully started Phase2 for job {} with {} provers",
            job_id,
            assigned_provers.len()
        );
        Ok(())
    }

    /// Generate global challenge for Phase2 (placeholder implementation)
    async fn generate_global_challenge(&self, job_id: &JobId) -> Result<Vec<u64>> {
        // TODO: Implement actual global challenge generation based on Phase1 results
        // For now, return a simple challenge
        info!("Generating global challenge for job {}", job_id);

        // In a real implementation, this would:
        // 1. Collect all Phase1 results for the job
        // 2. Compute a global challenge based on the results
        // 3. Return the challenge data

        Ok(vec![42, 1337, 12345]) // Placeholder challenge
    }
}
