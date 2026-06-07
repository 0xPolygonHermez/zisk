use crate::asm_services::CMD_SHUTDOWN_RESPONSE_ID;

use super::{
    FromResponsePayload, RequestData, ResponseData, ToRequestPayload, CMD_SHUTDOWN_REQUEST_ID,
};

pub(crate) struct ShutdownRequest;

impl ToRequestPayload for ShutdownRequest {
    fn to_request_payload(&self) -> RequestData {
        [CMD_SHUTDOWN_REQUEST_ID, 0, 0, 0, 0]
    }
}
pub(crate) struct ShutdownResponse;

impl FromResponsePayload for ShutdownResponse {
    fn from_response_payload(payload: ResponseData) -> Self {
        assert!(
            payload[0] == CMD_SHUTDOWN_RESPONSE_ID,
            "Expected CMD_SHUTDOWN_RESPONSE_ID but got {}",
            payload[0]
        );
        ShutdownResponse {}
    }
}
