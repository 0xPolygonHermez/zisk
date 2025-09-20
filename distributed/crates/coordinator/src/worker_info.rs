use std::fmt::Display;

use crate::coordinator_service::MessageSender;

use chrono::{DateTime, Utc};
use zisk_distributed_common::{ComputeCapacity, WorkerId, WorkerState};

/// Information about a connected worker
pub struct WorkerInfo {
    pub worker_id: WorkerId,
    pub state: WorkerState,
    pub compute_capacity: ComputeCapacity,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub msg_sender: Box<dyn MessageSender + Send + Sync>,
}

impl WorkerInfo {
    pub fn new(
        worker_id: WorkerId,
        compute_capacity: ComputeCapacity,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> Self {
        let now = Utc::now();
        Self {
            worker_id,
            state: WorkerState::Idle,
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

impl Display for WorkerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WorkerInfo(state: {}, capacity: {}, connected_at: {}, last_heartbeat: {})",
            self.state, self.compute_capacity, self.connected_at, self.last_heartbeat
        )
    }
}
