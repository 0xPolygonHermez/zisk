//! Main state machine for ZisK.
//!
//! - [`MainInstance`] — computes the witness for a segment of the main trace.
//! - [`MainPlanner`] — emit a `Plan` for each segment of the main trace.

#![warn(missing_docs)] // ratchet up to deny once clean
#![warn(rustdoc::all)] // broken intra-doc links, invalid HTML, bare URLs
#![deny(rustdoc::missing_crate_level_docs)]

mod error;
mod main_planner;
mod main_sm;

pub use error::*;
pub use main_planner::*;
pub use main_sm::*;
