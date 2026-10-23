//! The reset request: rebuild a service's guest RAM and ROM. See
//! [`AsmServices::reset_services`](super::AsmServices::reset_services) for when and why.

use super::{
    FromResponsePayload, RequestData, ResponseData, ToRequestPayload, CMD_RESET_REQUEST_ID,
    CMD_RESET_RESPONSE_ID,
};

pub(crate) struct ResetRequest;

impl ToRequestPayload for ResetRequest {
    fn to_request_payload(&self) -> RequestData {
        [CMD_RESET_REQUEST_ID, 0, 0, 0, 0]
    }
}

#[derive(Debug)]
pub(crate) struct ResetResponse {
    pub result: u64,
}

impl FromResponsePayload for ResetResponse {
    const RESPONSE_ID: u64 = CMD_RESET_RESPONSE_ID;

    fn from_response_payload(payload: ResponseData) -> Self {
        ResetResponse { result: payload[1] }
    }
}
