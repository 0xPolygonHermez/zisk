use crate::asm_services::CMD_MO_RESPONSE_ID;

use super::{FromResponsePayload, RequestData, ResponseData, ToRequestPayload, CMD_MO_REQUEST_ID};

pub struct MemoryOperationsRequest {
    pub max_steps: u64,
    pub chunk_len: u64,
}

impl ToRequestPayload for MemoryOperationsRequest {
    fn to_request_payload(&self) -> RequestData {
        [CMD_MO_REQUEST_ID, self.max_steps, self.chunk_len, 0, 0]
    }
}

pub struct MemoryOperationsResponse {
    pub result: u8,
    pub allocated_len: u64,
    pub trace_len: u64,
}

impl FromResponsePayload for MemoryOperationsResponse {
    fn from_response_payload(payload: ResponseData) -> Self {
        assert!(
            payload[0] == CMD_MO_RESPONSE_ID,
            "Expected CMD_MO_RESPONSE_ID but got {}",
            payload[0]
        );
        MemoryOperationsResponse {
            result: payload[1] as u8,
            allocated_len: payload[2],
            trace_len: payload[3],
        }
    }
}
