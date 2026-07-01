//! Main state machine for ZisK.
//!
//! - [`MainInstance`] — computes the witness for a segment of the main trace.
//! - [`MainPlanner`] — emit a `Plan` for each segment of the main trace.

mod error;
mod main_planner;
mod main_sm;

pub use error::*;
pub use main_planner::*;
pub use main_sm::*;
