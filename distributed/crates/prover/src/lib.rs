//! # Coordinator Client Library
//!
//! This library provides the core functionality for a coordinator network prover client.
//! It includes configuration management, proof generation, gRPC communication,
//! and prover service management.
pub mod config;
pub mod proof_generator;
pub mod prover_service;

pub use proof_generator::ProofGenerator;
pub use prover_service::{JobContext, ProverService, ProverServiceConfig};
