//! Hot-path data buses.
//!
//! Counter-phase ([`counter`]) and collect-phase ([`collect`]) wire
//! state-machine devices onto the bus during emulation. Touch with
//! care — these are per-row hot paths gated by the perf invariant.

mod collect;
mod counter;
pub mod pub_outs_collector;

pub use collect::*;
pub use counter::*;
