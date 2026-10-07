//! The assembly microservices, and the shared memory they use.
//!
//! Each program set up for the assembly executor runs three service processes, one per
//! [`AsmService`]: memory operations (MO), minimal traces (MT) and the ROM histogram (RH). The
//! parent talks to each over its stdin and stdout, one request at a time, and the services
//! exchange the bulk data with it through shared-memory segments in `/dev/shm`.
//!
//! # One set of segments per process, shared by every program
//!
//! The segments are many gigabytes, locked in memory, so a process that sets up several programs
//! (a worker, or a client proving several guests) cannot afford a set per program. There is one
//! set per process, rank and hints mode, named by `shm_prefix_for`, and every program set up on
//! it shares it. What is per program are the service processes and their semaphores, named by
//! `sem_prefix_for`.
//!
//! Sharing rests on three rules, each enforced in one place:
//!
//! - **Every segment fits every program.** All are fixed-size except the ROM histogram's output,
//!   which grows with the program's ROM. That one is created at its upper bound,
//!   `TRACE_INITIAL_SIZE_RH`, and each service maps and zeroes only what its own program needs
//!   (`trace.c`); the parent's reader maps again when a larger histogram arrives.
//! - **The segments outlive every program on them, and no longer.** The first program's setup
//!   creates them, and each holds a lease on them; the last lease released unlinks them
//!   (`PREFIX_LEASES`). A program's own teardown removes only its semaphores.
//! - **They serve one program at a time.** Which one is recorded beside the lease count, so every
//!   client in the process sees the same answer. [`AsmServices::activate`] hands them to another
//!   program: it waits until the active program's services have finished with them, rebinds the
//!   parent's writers to the new program's semaphores, and has its services rebuild their guest
//!   RAM and ROM (the reset request). Starting a new program's services waits the same way, since
//!   starting writes those segments too.
//!
//! # Failures
//!
//! A service whose process exits is not restarted. The first request to find it gone reports how
//! it exited, and every later one fails with [`AsmRunError::ServiceDied`](crate::AsmRunError),
//! as does activating its program.
//!
//! # Cached binaries
//!
//! The services' binaries are generated from a program's ELF and cached under a name made of the
//! ELF hash, the hints mode and `ASM_PROTOCOL_VERSION` (in `zisk-rom-setup`). Bump the version
//! whenever a change to `emulator-asm` alters how the binaries behave, or a binary built from the
//! old sources is reused.
//!
//! # Tests
//!
//! The unit tests here need no binaries. Running two programs in one process needs the real
//! services: `integration-tests/tests/asm_two_programs.rs` (two programs on one client) and
//! `asm_two_execute_clients.rs` (two clients), both ignored by default.

mod codec;
mod janitor;
mod memory_ops;
mod minimal_traces;
mod reset;
mod rom_histogram;
mod services;
mod shutdown;
mod status;
mod stdio;

// Only `services` (AsmService / AsmServices) is public API; the per-command
// request/response payload types and the wire traits are crate-internal.
pub(crate) use memory_ops::*;
pub(crate) use minimal_traces::*;
pub(crate) use reset::*;
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
const CMD_RESET_REQUEST_ID: u64 = 19;
const CMD_RESET_RESPONSE_ID: u64 = 20;
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
        assert_eq!(ResetRequest.to_request_payload(), [CMD_RESET_REQUEST_ID, 0, 0, 0, 0]);
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

        let r = ResetResponse::from_response_payload([CMD_RESET_RESPONSE_ID, 3, 4096, 0, 0]);
        assert_eq!(r.result, 3);
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
            CMD_RESET_REQUEST_ID,
            CMD_RESET_RESPONSE_ID,
            CMD_SHUTDOWN_REQUEST_ID,
            CMD_SHUTDOWN_RESPONSE_ID,
        ];
        let mut sorted = ids.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "command ids must be unique");
    }
}
