use anyhow::Result;
use chrono::{DateTime, Utc};
use distributed_common::JobId;
use distributed_common::{ComputeCapacity, ProverId};
use distributed_config::Config;
use distributed_grpc_api::{
    CoordinatorMessage, ExecuteTaskResponse, HeartbeatAck, ProverError, ProverReconnectRequest,
    ProverRegisterRequest,
};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::mpsc;
use tonic::Status;
use tracing::{error, info, instrument};

use crate::dto::{
    JobStatusDto, JobsListDto, MetricsDto, ProverReconnectRequestDto, ProverRegisterRequestDto,
    ProversListDto, StartProofRequestDto, StartProofResponseDto, StatusInfoDto, SystemStatusDto,
};
use crate::Coordinator;

/// Represents the runtime state of the service
pub struct CoordinatorService {
    config: Config,
    start_time_utc: DateTime<Utc>,
    active_connections: Arc<AtomicU32>,
    coordinator: Arc<Coordinator>,
}

impl CoordinatorService {
    #[instrument(skip(config))]
    pub async fn new(config: Config) -> distributed_common::Result<Self> {
        info!("Initializing service state");

        let start_time_utc = Utc::now();

        // Create ProverManager with configuration from config
        let coordinator = Arc::new(Coordinator::new(config.coordinator.clone()));

        Ok(Self {
            config,
            start_time_utc,
            active_connections: Arc::new(AtomicU32::new(0)),
            coordinator,
        })
    }

    pub fn active_connections(&self) -> Arc<AtomicU32> {
        self.active_connections.clone()
    }

    pub fn max_concurrent_connections(&self) -> u32 {
        self.config.coordinator.max_concurrent_connections
    }

    pub fn status_info(&self) -> StatusInfoDto {
        let uptime_seconds = (Utc::now() - self.start_time_utc).num_seconds() as u64;

        let metrics =
            MetricsDto { active_connections: self.active_connections.load(Ordering::SeqCst) };

        StatusInfoDto::new(
            "Distributed Prover Service".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds,
            self.start_time_utc,
            metrics,
        )
    }

    pub fn jobs_list(&self) -> JobsListDto {
        // TODO: Implement actual job retrieval from database
        JobsListDto { jobs: Vec::new() }
    }

    pub fn provers_list(&self) -> ProversListDto {
        // TODO: Implement actual prover retrieval from database
        ProversListDto { provers: Vec::new() }
    }

    pub fn job_status(&self, job_id: &JobId) -> JobStatusDto {
        // TODO: Implement actual job retrieval from database
        JobStatusDto {
            job_id: job_id.to_string(),
            block_id: "block123".to_string(),
            phase: "proving".to_string(),
            status: "in_progress".to_string(),
            assigned_provers: vec!["prover1".to_string(), "prover2".to_string()],
            start_time: Utc::now().timestamp() as u64,
            duration_ms: 5000,
        }
    }

    pub async fn handle_system_status(&self) -> SystemStatusDto {
        // Get actual system status from ProverManager
        let total_provers = self.coordinator.num_provers().await;
        let compute_capacity = self.coordinator.compute_capacity().await;
        let idle_provers = self.coordinator.num_provers().await;
        let busy_provers = total_provers.saturating_sub(idle_provers);

        SystemStatusDto {
            total_provers: total_provers as u32,
            compute_capacity,
            idle_provers: idle_provers as u32,
            busy_provers: busy_provers as u32,
            active_jobs: 0,                // TODO: Implement actual job counting
            pending_jobs: 0,               // TODO: Implement actual job counting
            completed_jobs_last_minute: 0, // TODO: Implement actual metrics
            job_completion_rate: 0.0,      // TODO: Implement actual metrics
            prover_utilization: if total_provers > 0 {
                (busy_provers as f64) / (total_provers as f64)
            } else {
                0.0
            },
        }
    }

    pub async fn start_proof(
        &self,
        request: StartProofRequestDto,
    ) -> Result<StartProofResponseDto> {
        let result = self
            .coordinator
            .start_proof(
                request.block_id,
                ComputeCapacity { compute_units: request.compute_units },
                request.input_path,
            )
            .await;

        match result {
            Ok(job_id) => {
                let job_id_str: String = job_id.into();
                info!("Successfully started proof job: {}", job_id_str);
                Ok(StartProofResponseDto { job_id: job_id_str })
            }
            Err(e) => {
                error!("Failed to start proof job: {}", e);
                let error_response = format!("Failed to start proof: {e}");

                Err(anyhow::anyhow!(error_response))
            }
        }
    }

    /// Handle registration directly in stream context (static version to avoid lifetime issues)
    pub async fn handle_stream_registration(
        &self,
        req: ProverRegisterRequestDto,
        msg_sender: mpsc::UnboundedSender<CoordinatorMessage>,
    ) -> Result<ProverId, Status> {
        self.coordinator
            .register_prover(ProverId::from(req.prover_id), req.compute_capacity, msg_sender)
            .await
            .map_err(|e| Status::internal(format!("Registration failed: {e}")))
    }

    /// Handle reconnection directly in stream context (static version to avoid lifetime issues)
    pub async fn handle_stream_reconnection(
        &self,
        req: ProverReconnectRequestDto,
        msg_sender: mpsc::UnboundedSender<CoordinatorMessage>,
    ) -> Result<ProverId, Status> {
        self.coordinator
            .register_prover(ProverId::from(req.prover_id), req.compute_capacity, msg_sender)
            .await
            .map_err(|e| Status::internal(format!("Reconnection failed: {e}")))
    }

    /// Unregister a prover by its ID
    pub async fn unregister_prover(&self, prover_id: &ProverId) -> Result<()> {
        Ok(self.coordinator.unregister_prover(prover_id).await?)
    }

    pub async fn handle_stream_heartbeat_ack(
        &self,
        prover_id: &ProverId,
        message: HeartbeatAck,
    ) -> Result<()> {
        self.coordinator
            .handle_stream_heartbeat_ack(prover_id, message)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn handle_stream_error(
        &self,
        prover_id: &ProverId,
        message: ProverError,
    ) -> Result<()> {
        self.coordinator
            .handle_stream_error(prover_id, message)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn handle_stream_register(
        &self,
        prover_id: &ProverId,
        message: ProverRegisterRequest,
    ) -> Result<()> {
        self.coordinator
            .handle_stream_register(prover_id, message)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn handle_stream_reconnect(
        &self,
        prover_id: &ProverId,
        message: ProverReconnectRequest,
    ) -> Result<()> {
        self.coordinator
            .handle_stream_reconnect(prover_id, message)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn handle_stream_execute_task_response(
        &self,
        prover_id: &ProverId,
        message: ExecuteTaskResponse,
    ) -> Result<()> {
        self.coordinator
            .handle_stream_execute_task_response(prover_id, message)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
