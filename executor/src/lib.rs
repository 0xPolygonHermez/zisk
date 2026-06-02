//! Executor crate is the core of the execution engine, responsible for orchestrating
//! the execution of state machines. It provides the main `ZiskExecutor` struct.

#![deny(missing_docs)]
#![deny(rustdoc::all)]

mod adapters;
mod bus;
mod error;
mod execution;
mod executor;
mod plan;
mod ports;
mod sm;
mod state;
mod witness;

// External API
pub use asm_runner::GpuBufferSource;
pub use execution::asm::{AsmResources, AsmSharedResources, EmulatorAsm}; // (Linux x86_64) / stub elsewhere
pub use executor::*; // ZiskExecutor

pub(crate) use adapters::*;
pub(crate) use bus::*;
pub(crate) use execution::*;
pub(crate) use plan::*;
pub(crate) use sm::*;
pub(crate) use state::*;
pub(crate) use witness::*;

use std::collections::HashMap;

/// Type alias for chunk counters, mapping SM type ID to a list of device metrics by chunk.
pub(crate) type CountersChunkMetrics = HashMap<usize, Vec<DeviceMetricsByChunk>>;
