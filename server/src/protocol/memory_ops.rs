use super::{FromResponsePayload, RequestData, ToRequestPayload};

pub struct MemoryOperationsRequest {
    chunk_len: u32,
    max_steps: u32,
}

impl ToRequestPayload for MemoryOperationsRequest {
    fn to_request_payload(&self) -> RequestData {
        [self.chunk_len as u64, self.max_steps as u64, 0, 0]
    }
}

pub struct MemoryOperationsResponse {
    result: u8,
    allocated_len: u64,
    trace_len: u64,
}

impl FromResponsePayload for MemoryOperationsResponse {
    fn from_response_payload(payload: RequestData) -> Self {
        MemoryOperationsResponse {
            result: payload[0] as u8,
            allocated_len: payload[1],
            trace_len: payload[2],
        }
    }
}
