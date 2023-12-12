#[derive(Debug, Clone)]
pub enum Payload {
    NewTrace {
        subproof_id: u32,
        air_id: u32,
    },
    Halt
}

#[derive(Debug, Clone)]
pub struct Message {
    pub src: String,
    pub dst: String,
    pub payload: Payload,
}
