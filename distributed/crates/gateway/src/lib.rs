//! ZisK Gateway — public API façade for the ZisK proving system.
//!
//! # Overview
//!
//! This crate implements the gateway API defined in
//! `book/developer/gateway_api.md`. It exposes a gRPC service
//! (`ZiskGatewayApi`) that clients use to:
//!
//! - Register guest programs ([`RegisterGuestProgram`])
//! - Submit proving jobs ([`JobRequest`])
//! - Poll or stream job status ([`WaitJobResult`], [`WatchJob`])
//! - Feed streaming input ([`PushJobInput`])
//! - Cancel jobs ([`CancelJob`])
//!
//! [`RegisterGuestProgram`]: proto::zisk_gateway_api_server::ZiskGatewayApi::register_guest_program
//! [`JobRequest`]: proto::zisk_gateway_api_server::ZiskGatewayApi::job_request
//! [`WaitJobResult`]: proto::zisk_gateway_api_server::ZiskGatewayApi::wait_job_result
//! [`WatchJob`]: proto::zisk_gateway_api_server::ZiskGatewayApi::watch_job
//! [`PushJobInput`]: proto::zisk_gateway_api_server::ZiskGatewayApi::push_job_input
//! [`CancelJob`]: proto::zisk_gateway_api_server::ZiskGatewayApi::cancel_job
//!
//! # Backend
//!
//! Business logic is delegated to a [`backend::BackendService`] implementation:
//!
//! - [`backend::mock::MockBackend`] — in-memory, no coordinator required;
//!   suitable for testing.
//! - [`backend::coordinator::CoordinatorBackend`] — runs the
//!   coordinator in-process; the production deployment mode.

pub mod backend;
pub mod config;
pub mod errors;
pub mod metrics;
pub mod server;
pub mod service;
pub mod shutdown;

/// Proto-generated types for `zisk.gateway.v1`.
pub mod proto {
    #![allow(clippy::large_enum_variant)]
    tonic::include_proto!("zisk.gateway.v1");
}

pub use config::Config as GatewayConfig;
pub use errors::{GatewayError, GatewayResult};
pub use server::GatewayServer;
