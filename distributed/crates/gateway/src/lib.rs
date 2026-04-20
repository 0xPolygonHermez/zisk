//! ZisK Gateway — public API façade for the ZisK proving system.
//!
//! # Overview
//!
//! This crate implements the gateway API defined in
//! `book/developer/gateway_api.md`. It exposes the `ZiskGatewayApi` gRPC service.
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
pub mod health;
pub mod metrics;
pub mod server;
pub mod shutdown;

/// Proto-generated types for `zisk.gateway.v1`.
pub use zisk_gateway_api::grpc::proto;

pub use config::Config as GatewayConfig;
pub use errors::{GatewayError, GatewayResult};
pub use grpc::GrpcAdapter;
pub use handler::GatewayHandler;
pub use server::GatewayServer;
