#[repr(C)]
#[derive(Debug)]
pub struct AsmRunnerInputC {
    pub chunk_size: u64,
    pub max_steps: u64,
    pub initial_trace_size: u64,
    pub input_data_size: u64,
}

impl AsmRunnerInputC {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(&self.chunk_size.to_le_bytes());
        bytes.extend_from_slice(&self.max_steps.to_le_bytes());
        bytes.extend_from_slice(&self.initial_trace_size.to_le_bytes());
        bytes.extend_from_slice(&self.input_data_size.to_le_bytes());
        bytes
    }
}
