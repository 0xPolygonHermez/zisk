use super::{FromResponsePayload, RequestData, ResponseData, ToRequestPayload, CMD_MO_REQUEST_ID};

pub struct MemoryOperationsRequest {
    max_steps: u64,
    chunk_len: u64,
}

impl ToRequestPayload for MemoryOperationsRequest {
    fn to_request_payload(&self) -> RequestData {
        [CMD_MO_REQUEST_ID, self.max_steps, self.chunk_len, 0, 0]
    }
}

pub struct MemoryOperationsResponse {
    result: u8,
    allocated_len: u64,
    trace_len: u64,
}

impl FromResponsePayload for MemoryOperationsResponse {
    fn from_response_payload(payload: ResponseData) -> Self {
        MemoryOperationsResponse {
            result: payload[1] as u8,
            allocated_len: payload[2],
            trace_len: payload[3],
        }
    }
}
