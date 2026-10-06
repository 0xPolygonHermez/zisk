//! Coordinator API definitions: the client-facing gRPC contract for the ZisK
//! coordinator.
//!
//! [`grpc`] holds the tonic-generated proto types and the client/server stubs;
//! [`dto`] holds the hand-written domain types and the proto ↔ domain
//! conversions used by the coordinator, server, and clients.

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

/// Domain types and proto ↔ domain conversions.
pub mod dto;
/// gRPC transport: generated proto types and client/server stubs.
pub mod grpc;
