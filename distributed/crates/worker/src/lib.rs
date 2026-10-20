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
/// A requested stop (SIGTERM, SIGINT, SIGUSR1): every rank cleans up and exits with status 0.
pub mod stop;
mod stream_ordering;
/// What the worker reports to its coordinator about its host and processes.
pub mod telemetry;
/// Prover-facing worker: runs the proving work for assigned jobs.
pub mod worker;
/// Worker node: connects to the coordinator and drives the job lifecycle.
pub mod worker_node;

pub use worker::{ProverConfig, Worker};
pub use worker_node::*;
