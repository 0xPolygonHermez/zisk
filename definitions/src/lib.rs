#![no_std]

mod profile;
pub use profile::*;

mod labels;
pub use labels::*;

pub mod hints;
pub use hints::*;

// Constants, in two feature-gated views: without `gen`, consumers compile the
// generated plain `pub const`s (`generated`, zero-dep); with `gen`, the sync build
// compiles the `#[constants]` source (`constants`, `ZISK_CONSTANTS`). Mutually
// exclusive so the `gen` build never depends on files it is about to regenerate.
// Syscall ids (and future constant groups) live here — generated from the
// `#[constants]` source in `src/constants/` by the `zisk-definitions-sync` build.
#[cfg(not(feature = "gen"))]
mod generated;
#[cfg(not(feature = "gen"))]
pub use generated::*;
// The syscall ids were historically flat at the crate root (`zisk_definitions::
// SYSCALL_*_ID`); keep that path for the existing consumers. (Newer groups are
// accessed group-qualified, e.g. `zisk_definitions::memory::*`.)
#[cfg(not(feature = "gen"))]
pub use generated::syscall::*;

#[cfg(feature = "gen")]
mod constants;
#[cfg(feature = "gen")]
pub use constants::ZISK_CONSTANTS;
