#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    NewTrace {
        subproof_id: usize,
        air_id: usize,
    },
    Halt
}

impl Payload {
    pub fn new_trace(subproof_id: usize, air_id: usize) -> Self {
        Payload::NewTrace {
            subproof_id,
            air_id,
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
