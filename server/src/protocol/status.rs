use super::{FromResponsePayload, RequestData, ToRequestPayload};

pub struct PingRequest {}

impl ToRequestPayload for PingRequest {
    fn to_request_payload(&self) -> RequestData {
        [0u64; 4]
    }
}
pub struct PingResponse {
    generation_method: u64,
    allocated_size: u64,
}

impl FromResponsePayload for PingResponse {
    fn from_response_payload(payload: RequestData) -> Self {
        PingResponse { generation_method: payload[0], allocated_size: payload[1] }
    }
}
