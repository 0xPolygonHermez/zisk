//! Shared test utilities for in-crate unit tests.

use std::sync::{Arc, Mutex};

use zisk_cluster_common::{CoordinatorMessageDto, WorkerId};

use crate::coordinator::MessageSender;
use crate::coordinator_errors::CoordinatorResult;
use crate::workers_pool::WorkersPool;

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

pub async fn register_test_worker(
    pool: &WorkersPool,
    id: &str,
) -> (WorkerId, Arc<Mutex<Vec<CoordinatorMessageDto>>>) {
    let worker_id = WorkerId::from(id.to_string());
    let (sender, messages) = MockMessageSender::new();
    pool.register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Idle).await.unwrap();
    (worker_id, messages)
}
