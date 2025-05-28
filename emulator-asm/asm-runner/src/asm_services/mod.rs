mod memory_ops;
mod minimal_traces;
mod rom_histogram;
mod services;
mod shutdown;
mod status;

pub use memory_ops::*;
pub use minimal_traces::*;
pub use rom_histogram::*;
pub use services::*;
pub use shutdown::*;
pub use status::*;

pub type RequestData = [u64; 5];
pub type ResponseData = [u64; 5];

pub trait ToRequestPayload {
    fn to_request_payload(&self) -> RequestData;
}

pub trait FromResponsePayload {
    fn from_response_payload(payload: ResponseData) -> Self;
}

const CMD_PING_REQUEST_ID: u64 = 1;
const CMD_PING_RESPONSE_ID: u64 = 2;
const CMD_MT_REQUEST_ID: u64 = 3;
const CMD_MT_RESPONSE_ID: u64 = 4;
const CMD_RH_REQUEST_ID: u64 = 5;
const CMD_RH_RESPONSE_ID: u64 = 6;
const CMD_MO_REQUEST_ID: u64 = 7;
const CMD_MO_RESPONSE_ID: u64 = 8;
const CMD_SHUTDOWN_REQUEST_ID: u64 = 1000000;
const CMD_SHUTDOWN_RESPONSE_ID: u64 = 1000001;

pub enum AsmCmdRequest {
    Ping(PingRequest),
    MinimalTrace(MinimalTraceRequest),
    RomHistogram(RomHistogramRequest),
    MemoryOperations(MemoryOperationsRequest),
    Shutdown(ShutdownRequest),
}

pub enum AsmCmdResponse {
    Ping(PingResponse),
    MinimalTrace(MinimalTraceResponse),
    RomHistogram(RomHistogramResponse),
    MemoryOperations(MemoryOperationsResponse),
    Shutdown(ShutdownResponse),
}
