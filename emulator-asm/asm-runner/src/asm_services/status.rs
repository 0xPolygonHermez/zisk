use super::{
    FromResponsePayload, RequestData, ResponseData, ToRequestPayload, CMD_PING_REQUEST_ID,
    CMD_PING_RESPONSE_ID,
};

pub(crate) struct PingRequest;

impl ToRequestPayload for PingRequest {
    fn to_request_payload(&self) -> RequestData {
        [CMD_PING_REQUEST_ID, 0, 0, 0, 0]
    }
}

// Fields mirror the on-wire ping response; these document the protocol layout.
#[allow(dead_code)]
pub(crate) struct PingResponse {
    /// The producer's generation method.
    pub generation_method: u64,
    /// The total allocated size of the shared memory segment.
    pub allocated_size: u64,
}

impl FromResponsePayload for PingResponse {
    fn from_response_payload(payload: ResponseData) -> Self {
        assert!(
            payload[0] == CMD_PING_RESPONSE_ID,
            "Expected CMD_PING_RESPONSE_ID but got {}",
            payload[0]
        );
        PingResponse { generation_method: payload[1], allocated_size: payload[2] }
    }
}
