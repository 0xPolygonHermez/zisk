mod codec;
mod janitor;
mod memory_ops;
mod minimal_traces;
mod rom_histogram;
mod services;
mod shutdown;
mod status;
mod stdio;

// Only `services` (AsmService / AsmServices) is public API; the per-command
// request/response payload types and the wire traits are crate-internal.
pub(crate) use memory_ops::*;
pub(crate) use minimal_traces::*;
pub(crate) use rom_histogram::*;
pub use services::*;
pub(crate) use shutdown::*;
pub(crate) use status::*;

pub(crate) type RequestData = [u64; 5];
pub(crate) type ResponseData = [u64; 5];

pub(crate) trait ToRequestPayload {
    fn to_request_payload(&self) -> RequestData;
}

pub(crate) trait FromResponsePayload {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_payloads_carry_correct_command_ids() {
        assert_eq!(PingRequest.to_request_payload(), [CMD_PING_REQUEST_ID, 0, 0, 0, 0]);
        assert_eq!(
            MinimalTraceRequest { max_steps: 9, chunk_len: 4 }.to_request_payload(),
            [CMD_MT_REQUEST_ID, 9, 4, 0, 0]
        );
        assert_eq!(
            RomHistogramRequest { max_steps: 5 }.to_request_payload(),
            [CMD_RH_REQUEST_ID, 5, 0, 0, 0]
        );
        assert_eq!(
            MemoryOperationsRequest { max_steps: 1, chunk_len: 2 }.to_request_payload(),
            [CMD_MO_REQUEST_ID, 1, 2, 0, 0]
        );
        assert_eq!(ShutdownRequest.to_request_payload(), [CMD_SHUTDOWN_REQUEST_ID, 0, 0, 0, 0]);
    }

    #[test]
    fn response_payloads_parse_their_fields() {
        let r = MinimalTraceResponse::from_response_payload([CMD_MT_RESPONSE_ID, 1, 100, 50, 0]);
        assert_eq!((r.result, r.allocated_len, r.trace_len), (1, 100, 50));

        let r = MemoryOperationsResponse::from_response_payload([CMD_MO_RESPONSE_ID, 0, 8, 4, 0]);
        assert_eq!((r.result, r.allocated_len, r.trace_len), (0, 8, 4));

        let r = RomHistogramResponse::from_response_payload([CMD_RH_RESPONSE_ID, 0, 9, 3, 77]);
        assert_eq!((r.result, r.allocated_len, r.trace_len, r.last_step), (0, 9, 3, 77));
    }

    #[test]
    #[should_panic]
    fn response_parse_rejects_mismatched_command_id() {
        // A frame with the wrong response id indicates a protocol desync.
        let _ = MinimalTraceResponse::from_response_payload([0xBAD, 0, 0, 0, 0]);
    }

    #[test]
    fn all_command_ids_are_unique() {
        let ids = [
            CMD_PING_REQUEST_ID,
            CMD_PING_RESPONSE_ID,
            CMD_MT_REQUEST_ID,
            CMD_MT_RESPONSE_ID,
            CMD_RH_REQUEST_ID,
            CMD_RH_RESPONSE_ID,
            CMD_MO_REQUEST_ID,
            CMD_MO_RESPONSE_ID,
            CMD_SHUTDOWN_REQUEST_ID,
            CMD_SHUTDOWN_RESPONSE_ID,
        ];
        let mut sorted = ids.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "command ids must be unique");
    }
}
