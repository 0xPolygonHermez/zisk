//! Shared types for the ZisK distributed cluster.
//!
//! Common definitions used by both the coordinator and workers: the wire
//! [`dto`] messages exchanged over the cluster API, the core cluster [`types`]
//! (worker ids, states, capacities, job phases), and [`tracing`] setup helpers.

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

/// Wire messages exchanged between coordinator and workers.
pub mod dto;
/// Logging/tracing initialization helpers.
pub mod tracing;
/// Core cluster types: worker ids, states, capacities, and job phases.
pub mod types;

pub use dto::*;
pub use tracing::*;
pub use types::*;
