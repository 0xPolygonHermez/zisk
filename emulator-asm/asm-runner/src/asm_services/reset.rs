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
    pub result: u8,
}

impl FromResponsePayload for ResetResponse {
    fn from_response_payload(payload: ResponseData) -> Self {
        // Not an assert, unlike the other decoders: a reset that cannot be read fails
        // the switch with an error, through the non-zero result, rather than aborting
        // the process.
        if payload[0] != CMD_RESET_RESPONSE_ID {
            tracing::error!("Expected CMD_RESET_RESPONSE_ID but got {}", payload[0]);
            return ResetResponse { result: u8::MAX };
        }
        ResetResponse { result: payload[1] as u8 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_response_with_another_id_is_a_failed_reset_not_a_panic() {
        let response =
            ResetResponse::from_response_payload([CMD_RESET_RESPONSE_ID + 1, 0, 0, 0, 0]);
        assert_ne!(response.result, 0);
        let response = ResetResponse::from_response_payload([CMD_RESET_RESPONSE_ID, 0, 0, 0, 0]);
        assert_eq!(response.result, 0);
    }
}
