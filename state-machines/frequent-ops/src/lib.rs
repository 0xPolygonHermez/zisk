//! The `FrequentOps` module implements the Frequent Operations State Machine,
//! which manages and tracks frequently used arithmetic operations for optimization.
//!
//! Key components of this module include:
//! - The `FrequentOpsSM` struct, managing the state machine for frequent operations.
//! - `FrequentOpsTable` for efficient lookup of common operation patterns.
//! - Input collectors, counters, and planners for managing frequent operations data.
//! - Instance management for witness computation of frequent operations.

mod frequent_ops_helpers;
mod frequent_ops_table;

pub use frequent_ops_helpers::*;
pub use frequent_ops_table::*;
