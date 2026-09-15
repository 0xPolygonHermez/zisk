//! The reset command: re-initialize a service's guest RAM and ROM.
//!
//! The `_ram`/`_rom` segments are keyed by `pid`+`local_rank` only, so every
//! program set up on a worker shares them. A service's own
//! `server_reset_slow` runs *after* each emulation, which leaves its memory
//! correct for its own next run but says nothing about what another program's
//! services did to those segments in between. Sending this before the first
//! emulation after a program switch is what restores the invariant.
//!
//! The C side services this synchronously — the response is only written once
//! the re-initialization has completed, so receiving it is the guarantee the
//! caller needs before issuing an emulation request.

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

/// Fields mirror the on-wire reset response; they document the protocol layout.
#[allow(dead_code)]
pub(crate) struct ResetResponse {
    /// `0` on success.
    pub result: u64,
    /// The producer's currently allocated trace size, echoed back.
    pub allocated_len: u64,
}

impl FromResponsePayload for ResetResponse {
    fn from_response_payload(payload: ResponseData) -> Self {
        assert!(
            payload[0] == CMD_RESET_RESPONSE_ID,
            "Expected CMD_RESET_RESPONSE_ID but got {}",
            payload[0]
        );
        ResetResponse { result: payload[1], allocated_len: payload[2] }
    }
}
