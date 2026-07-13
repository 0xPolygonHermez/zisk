#![no_std]

mod syscall;
pub use syscall::*;

mod profile;
pub use profile::*;

mod labels;
pub use labels::*;

pub mod hints;
pub use hints::*;

// Constants, in two feature-gated views: without `gen`, consumers compile the
// generated plain `pub const`s (`generated`, zero-dep); with `gen`, the regen build
// compiles the `#[constants]` source (`constants`, `ZISK_CONSTANTS`). Mutually
// exclusive so the `gen` build never depends on files it is about to regenerate.
#[cfg(not(feature = "gen"))]
mod generated;
// The glob re-exports nothing while `generated` is an empty stub; the allow is a
// no-op once real constants populate it.
#[cfg(not(feature = "gen"))]
#[allow(unused_imports)]
pub use generated::*;

#[cfg(feature = "gen")]
mod constants;
#[cfg(feature = "gen")]
pub use constants::ZISK_CONSTANTS;
