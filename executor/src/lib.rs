//! Executor crate is the core of the execution engine, responsible for orchestrating
//! the execution of state machines. It provides the main `ZiskExecutor` struct.

#![deny(missing_docs)]
#![deny(rustdoc::all)]

mod adapters;
mod bus;
mod error;
mod execution;
mod executor;
mod executor_test;
mod plan;
mod ports;
mod sm;
mod state;
/// Post-hoc trace-row hooks for the unit-test executor path.
pub mod unit_test_hooks;
/// Registry of per-SM unit-test targets and the AIR-id → inner-SM manager map.
pub mod unit_test_targets;
/// Raw trace-authoring overrides that bypass `compute_witness`.
pub mod unit_test_trace_override;
mod witness;

// External API
pub use asm_runner::GpuBufferSource;
pub use execution::asm::{AsmResources, AsmSharedResources, EmulatorAsm}; // (Linux x86_64) / stub elsewhere
pub use executor::*; // ZiskExecutor
pub use executor_test::*; // ZiskExecutorTest (unit-test path)
pub use witness::AirClassifier; // AIR id → display name (used to label remote execution plans)

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
