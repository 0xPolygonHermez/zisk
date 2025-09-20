//! # ZisK Worker Library
//!
//! This library provides the core functionality for a ZisK Worker, which connects to a ZisK Coordinator
//! to receive and process proof generation jobs. It includes configuration management, gRPC
//! communication, and job handling capabilities.

pub mod config;
pub mod proof_generator;
pub mod worker_grpc_endpoint;
pub mod worker_service;

pub use proof_generator::ProofGenerator;
pub use worker_grpc_endpoint::*;
pub use worker_service::{JobContext, ProverServiceConfig, WorkerService};
