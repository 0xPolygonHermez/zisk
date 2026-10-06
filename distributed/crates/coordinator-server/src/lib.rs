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

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

/// Backend abstraction and its mock / in-process coordinator implementations.
pub mod backend;
/// Server configuration.
pub mod config;
/// API error and result types.
pub mod errors;
/// gRPC adapter wiring the service to a [`backend::BackendService`].
mod grpc;
/// Request handler translating API calls into backend operations.
mod handler;
/// Prometheus metrics.
pub mod metrics;
/// Server bootstrap and lifecycle.
pub mod server;
/// Graceful-shutdown coordination.
pub mod shutdown;

/// Proto-generated types for `zisk.coordinator.v1` (crate-internal; external
/// consumers should depend on `zisk_coordinator_api::grpc::proto` directly).
pub(crate) use zisk_coordinator_api::grpc::proto;

pub use config::Config as CoordinatorServerConfig;
pub use errors::{ApiError, ApiResult};
pub use grpc::GrpcAdapter;
pub use handler::CoordinatorHandler;
pub use server::CoordinatorServer;
