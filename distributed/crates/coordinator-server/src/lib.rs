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

pub mod backend;
pub mod config;
pub mod errors;
pub mod grpc;
pub mod handler;
pub mod metrics;
pub mod server;
pub mod shutdown;

/// Proto-generated types for `zisk.coordinator.v1`.
pub use zisk_coordinator_api::grpc::proto;

pub use config::Config as CoordinatorServerConfig;
pub use errors::{ApiError, ApiResult};
pub use grpc::GrpcAdapter;
pub use handler::CoordinatorHandler;
pub use server::CoordinatorServer;
