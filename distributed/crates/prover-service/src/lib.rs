//! # Distributed Client Library
//!
//! This library provides the core functionality for a distributed network prover client.
//! It includes configuration management, proof generation, gRPC communication,
//! and prover service management.
pub mod config;
pub mod prover_grpc_endpoint;

pub use config::*;
pub use prover_grpc_endpoint::*;
