//! ZisK coordinator: the core that drives distributed proving.
//!
//! A [`Coordinator`] accepts jobs, hands work to a pool of connected workers,
//! relays their partial proofs through the aggregation and wrap phases, and
//! reports progress via [`job_events`]. It is transport-agnostic; the gRPC
//! worker-facing surface lives alongside it and the client-facing API is
//! served by `zisk-coordinator-server`.

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

mod config;
mod coordinator;
mod coordinator_errors;
mod coordinator_grpc;
mod hints_relay;
mod hooks;
/// Job lifecycle events emitted by the coordinator.
pub mod job_events;
mod metrics;
mod shutdown;
mod workers_pool;

#[cfg(test)]
pub(crate) mod test_utils;

pub use config::*;
pub use coordinator::*;
pub use coordinator_errors::*;
pub use coordinator_grpc::*;
pub use hints_relay::*;
pub use shutdown::*;
pub use workers_pool::*;
