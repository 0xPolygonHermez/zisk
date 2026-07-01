//! Zisk standard library for guest programs.
//!
//! Provides three layers:
//! - [`lib`] — High-level arithmetic, hashing, and elliptic curve operations backed by syscalls.
//! - [`fcalls`] — Free-input call wrappers (hints) for operations that are not zk-friendly.
//! - [`fcalls_impl`] — Software implementations of fcalls, used on native targets and for trace
//!   generation.

mod fcalls;
#[cfg(not(zisk_guest))]
mod fcalls_impl;
pub mod lib;

pub use fcalls::*;
#[cfg(not(zisk_guest))]
pub use fcalls_impl::*;
pub use lib::*;
