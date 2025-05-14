mod memory_ops;
mod minimal_traces;
mod rom_histogram;
mod shutdown;
mod status;

pub use memory_ops::*;
pub use minimal_traces::*;
pub use rom_histogram::*;
pub use shutdown::*;
pub use status::*;

pub type RequestData = [u64; 4];
pub type ResponseData = [u64; 4];

pub trait ToRequestPayload {
    fn to_request_payload(&self) -> RequestData;
}

pub trait FromResponsePayload {
    fn from_response_payload(payload: ResponseData) -> Self;
}

pub enum AsCmdRequest {
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
