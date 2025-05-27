use crate::asm_services::CMD_RH_RESPONSE_ID;

use super::{FromResponsePayload, RequestData, ResponseData, ToRequestPayload, CMD_RH_REQUEST_ID};

pub struct RomHistogramRequest {
    max_steps: u32,
}

impl ToRequestPayload for RomHistogramRequest {
    fn to_request_payload(&self) -> RequestData {
        [CMD_RH_REQUEST_ID, self.max_steps as u64, 0, 0, 0]
    }
}

pub struct RomHistogramResponse {
    result: u8,
    allocated_len: u64,
    trace_len: u64,
    last_step: u64,
}

impl FromResponsePayload for RomHistogramResponse {
    fn from_response_payload(payload: ResponseData) -> Self {
        assert!(payload[0] == CMD_RH_RESPONSE_ID,);
        RomHistogramResponse {
            result: payload[1] as u8,
            allocated_len: payload[2],
            trace_len: payload[3],
            last_step: payload[4],
        }
    }
}
