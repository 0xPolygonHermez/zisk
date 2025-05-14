use super::{FromResponsePayload, RequestData, ToRequestPayload};

pub struct MinimalTraceRequest {
    chunk_len: u32,
    max_steps: u32,
}

impl ToRequestPayload for MinimalTraceRequest {
    fn to_request_payload(&self) -> RequestData {
        [self.chunk_len as u64, self.max_steps as u64, 0, 0]
    }
}

pub struct MinimalTraceResponse {
    result: u8,
    allocated_len: u64,
    trace_len: u64,
}

impl FromResponsePayload for MinimalTraceResponse {
    fn from_response_payload(payload: RequestData) -> Self {
        MinimalTraceResponse {
            result: payload[0] as u8,
            allocated_len: payload[1],
            trace_len: payload[2],
        }
    }
}
