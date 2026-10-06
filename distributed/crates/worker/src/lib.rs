//! # ZisK Worker Library
//!
//! This library provides the core functionality for a ZisK Worker, which connects to a ZisK Coordinator
//! to receive and process proof generation jobs. It includes configuration management, gRPC
//! communication, and job handling capabilities.

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

/// Worker configuration.
pub mod config;
mod stream_ordering;
/// Prover-facing worker: runs the proving work for assigned jobs.
pub mod worker;
/// Worker node: connects to the coordinator and drives the job lifecycle.
pub mod worker_node;

pub use worker::{ProverConfig, Worker};
pub use worker_node::*;
