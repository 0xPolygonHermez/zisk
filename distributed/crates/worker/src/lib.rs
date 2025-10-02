//! # ZisK Worker Library
//!
//! This library provides the core functionality for a ZisK Worker, which connects to a ZisK Coordinator
//! to receive and process proof generation jobs. It includes configuration management, gRPC
//! communication, and job handling capabilities.

pub mod config;
pub mod worker;
pub mod worker_node;

pub use worker::{ProverConfig, Worker};
pub use worker_node::*;
