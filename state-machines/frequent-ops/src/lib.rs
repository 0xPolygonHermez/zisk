//! The `FrequentOps` module implements the Frequent Operations State Machine,
//! which manages and tracks frequently used arithmetic operations for optimization.
//!
//! Key components of this module include:
//! - The `FrequentOpsSM` struct, managing the state machine for frequent operations.
//! - `FrequentOpsTable` for efficient lookup of common operation patterns.
//! - Input collectors, counters, and planners for managing frequent operations data.
//! - Instance management for witness computation of frequent operations.

mod frequent_ops;
mod frequent_ops_collector;
mod frequent_ops_counter;
mod frequent_ops_instance;
mod frequent_ops_planner;
mod frequent_ops_table;

pub use frequent_ops::*;
pub use frequent_ops_collector::*;
pub use frequent_ops_counter::*;
pub use frequent_ops_instance::*;
pub use frequent_ops_planner::*;
pub use frequent_ops_table::*;
