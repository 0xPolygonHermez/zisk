use crate::trace::Trace;
use std::sync::Arc;

#[derive(Debug)]
pub enum Payload {
    NewTrace {
        subproof_id: usize,
        trace: Arc<Box<dyn Trace>>,
    },
    Halt,
    Finished,
}

impl Payload {
    pub fn new_trace(subproof_id: usize, trace: Box<dyn Trace>) -> Self {
        Payload::NewTrace {
            subproof_id,
            trace: Arc::new(trace),
        }
    }

    pub fn new_halt() -> Self {
        Payload::Halt
    }
}

impl Clone for Payload {
    fn clone(&self) -> Self {
        match self {
            Payload::NewTrace { subproof_id, trace } => Payload::NewTrace {
                subproof_id: *subproof_id,
                trace: Arc::clone(trace),
            },
            Payload::Halt => Payload::Halt,
            Payload::Finished => Payload::Finished,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub src: String,
    pub dst: String,
    pub payload: Payload,
}
