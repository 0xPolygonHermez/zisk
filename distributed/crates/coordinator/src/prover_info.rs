use std::fmt::Display;

use crate::coordinator_service::MessageSender;

use chrono::{DateTime, Utc};
use distributed_common::{ComputeCapacity, ProverId, ProverState};

/// Information about a connected prover client
pub struct ProverInfo {
    pub prover_id: ProverId,
    pub state: ProverState,
    pub compute_capacity: ComputeCapacity,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub msg_sender: Box<dyn MessageSender + Send + Sync>,
}

impl ProverInfo {
    /// Create a new ProverConnection2
    pub fn new(
        prover_id: ProverId,
        compute_capacity: ComputeCapacity,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> Self {
        let now = Utc::now();
        Self {
            prover_id,
            state: ProverState::Idle,
            compute_capacity,
            connected_at: now,
            last_heartbeat: now,
            msg_sender,
        }
    }

    pub fn update_last_heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
    }
}

impl Display for ProverInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ProverConnection(state: {}, capacity: {}, connected_at: {}, last_heartbeat: {})",
            self.state, self.compute_capacity, self.connected_at, self.last_heartbeat
        )
    }
}
