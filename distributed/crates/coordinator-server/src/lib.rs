//! ZisK Coordinator Server — public API façade for the ZisK proving system.
//!
//! # Overview
//!
//! This crate implements the coordinator API defined in
//! `book/developer/coordinator_api.md`. It exposes the `ZiskCoordinatorApi` gRPC service.
//!
//! # Backend
//!
//! Business logic is delegated to a [`backend::BackendService`] implementation:
//!
//! - [`backend::mock::MockBackend`] — in-memory, no coordinator required;
//!   suitable for testing.
//! - [`backend::coordinator::CoordinatorBackend`] — runs the coordinator in-process.

pub mod auth;
pub mod backend;
pub mod config;
pub mod errors;
pub mod grpc;
pub mod handler;
pub mod metrics;
pub mod server;
pub mod shutdown;

/// Large proof/result messages can otherwise spend long periods constrained by
/// tonic's default 64 KiB HTTP/2 flow-control windows. Keep these settings
/// shared across the public API server and the worker-facing cluster server so
/// proof result uploads are not accidentally left on the small default window.
pub const HTTP2_CONNECTION_WINDOW_SIZE: u32 = 16 * 1024 * 1024; // 16 MB
pub const HTTP2_STREAM_WINDOW_SIZE: u32 = 8 * 1024 * 1024; // 8 MB

/// Proto-generated types for `zisk.coordinator.v1`.
pub use zisk_coordinator_api::grpc::proto;

pub use config::Config as CoordinatorServerConfig;
pub use errors::{ApiError, ApiResult};
pub use grpc::GrpcAdapter;
pub use handler::CoordinatorHandler;
pub use server::CoordinatorServer;
