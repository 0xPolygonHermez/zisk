//! Shared test utilities for in-crate unit tests.

use crate::workers_pool::WorkersPool;
use crate::{coordinator_errors::CoordinatorResult, worker_handlers::MessageSender};
use std::sync::{Arc, Mutex};
use zisk_cluster_common::{CoordinatorMessageDto, WorkerId};

pub struct MockMessageSender {
    pub messages: Arc<Mutex<Vec<CoordinatorMessageDto>>>,
}

impl MockMessageSender {
    pub fn new() -> (Self, Arc<Mutex<Vec<CoordinatorMessageDto>>>) {
        let messages = Arc::new(Mutex::new(Vec::new()));
        (Self { messages: messages.clone() }, messages)
    }
}

impl MessageSender for MockMessageSender {
    fn send(&self, msg: CoordinatorMessageDto) -> CoordinatorResult<()> {
        self.messages.lock().unwrap().push(msg);
        Ok(())
    }
}

/// `MessageSender` that always returns a `channel closed` error.
pub struct FailingMessageSender;

impl MessageSender for FailingMessageSender {
    fn send(&self, _msg: CoordinatorMessageDto) -> CoordinatorResult<()> {
        Err(crate::coordinator_errors::CoordinatorError::Internal(
            "Failed to send message: channel closed".to_string(),
        ))
    }
}

pub async fn register_test_worker(
    pool: &WorkersPool,
    id: &str,
) -> (WorkerId, Arc<Mutex<Vec<CoordinatorMessageDto>>>) {
    use zisk_cluster_common::WorkerState;
    let worker_id = WorkerId::from(id.to_string());
    let (sender, messages) = MockMessageSender::new();
    pool.register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Idle)
        .await
        .unwrap();
    (worker_id, messages)
}
