#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    NewTrace {
        subproof_id: usize,
        air_id: usize,
        trace_id: usize,
    },
    Halt,
    Finished,
}

impl Payload {
    pub fn new_trace(subproof_id: usize, air_id: usize, trace_id: usize) -> Self {
        Payload::NewTrace {
            subproof_id,
            air_id,
            trace_id,
        }
    }

    pub fn new_halt() -> Self {
        Payload::Halt
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub src: String,
    pub dst: String,
    pub payload: Payload,
}
