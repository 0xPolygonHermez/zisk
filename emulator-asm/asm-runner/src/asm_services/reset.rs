//! The reset request: rebuild a service's guest RAM and ROM, or clear its program's init data
//! from them. See [`AsmServices::activate`](super::AsmServices::activate) for when and why.

use super::{
    FromResponsePayload, RequestData, ResponseData, ToRequestPayload, CMD_RESET_REQUEST_ID,
    CMD_RESET_RESPONSE_ID,
};

/// What a reset asks of a service (`RS_MODE_*` in `emulator-asm/src/constants.hpp`).
///
/// The service echoes the mode it served. A binary built before the modes existed ignores the
/// mode, serves every reset as [`Full`](Self::Full) and echoes 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResetMode {
    /// Zero the guest RAM and ROM, 640 MiB, then write the program's init data.
    Full = 0,
    /// Clear the program's init data, which leaves the guest RAM and ROM all zero: the service
    /// zeroes its RAM after every run, before it reads another request.
    Release = 1,
    /// Write the program's init data into guest RAM and ROM that a release left all zero.
    Acquire = 2,
}

pub(crate) struct ResetRequest {
    pub mode: ResetMode,
}

impl ToRequestPayload for ResetRequest {
    fn to_request_payload(&self) -> RequestData {
        [CMD_RESET_REQUEST_ID, self.mode as u64, 0, 0, 0]
    }
}

#[derive(Debug)]
pub(crate) struct ResetResponse {
    pub result: u64,
    /// The mode the service served, as `ResetMode as u64`.
    pub served: u64,
}

impl FromResponsePayload for ResetResponse {
    const RESPONSE_ID: u64 = CMD_RESET_RESPONSE_ID;

    fn from_response_payload(payload: ResponseData) -> Self {
        ResetResponse { result: payload[1], served: payload[2] }
    }
}
