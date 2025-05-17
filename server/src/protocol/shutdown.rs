use super::{FromResponsePayload, RequestData, ToRequestPayload};

pub struct ShutdownRequest {}

impl ToRequestPayload for ShutdownRequest {
    fn to_request_payload(&self) -> RequestData {
        [0u64; 4]
    }
}
pub struct ShutdownResponse {}

impl FromResponsePayload for ShutdownResponse {
    fn from_response_payload(_payload: RequestData) -> Self {
        ShutdownResponse {}
    }
}
