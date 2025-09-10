use chrono::{DateTime, Utc};
use distributed_common::{ComputeCapacity, ProverId, ProverState};
use distributed_grpc_api::CoordinatorMessage;
use tokio::sync::mpsc;

/// Information about a connected prover client
pub struct ProverConnection {
    pub prover_id: ProverId,
    pub state: ProverState,
    pub compute_capacity: ComputeCapacity,
    connected_at: DateTime<Utc>,
    last_heartbeat: DateTime<Utc>,
    pub message_sender: mpsc::Sender<CoordinatorMessage>,
}

impl ProverConnection {
    /// Create a new ProverConnection2
    pub fn new(
        prover_id: ProverId,
        compute_capacity: ComputeCapacity,
        message_sender: mpsc::Sender<CoordinatorMessage>,
    ) -> Self {
        let now = Utc::now();
        Self {
            prover_id,
            state: ProverState::Idle,
            compute_capacity,
            connected_at: now,
            last_heartbeat: now,
            message_sender,
        }
    }

    pub fn update_last_heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
    }
}
