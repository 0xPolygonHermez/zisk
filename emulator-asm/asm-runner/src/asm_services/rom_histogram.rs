use crate::asm_services::CMD_RH_RESPONSE_ID;

use super::{FromResponsePayload, RequestData, ResponseData, ToRequestPayload, CMD_RH_REQUEST_ID};

pub struct RomHistogramRequest {
    pub max_steps: u64,
}

impl ToRequestPayload for RomHistogramRequest {
    fn to_request_payload(&self) -> RequestData {
        [CMD_RH_REQUEST_ID, self.max_steps, 0, 0, 0]
    }
}

pub struct RomHistogramResponse {
    pub result: u8,
    pub allocated_len: u64,
    pub trace_len: u64,
    pub last_step: u64,
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
