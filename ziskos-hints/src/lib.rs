//! ziskos-hints: ziskos compiled with hints feature enabled
//!
//! This crate compiles the symlinked core/ (which points to ziskos/entrypoint/src)
//! with the hints feature enabled, and adds hints-specific processing utilities.

// Include the symlinked source as a module
#[path = "core/lib.rs"]
mod core;

// Re-export everything from the symlinked implementation
pub use core::*;

// The symlinked source references INPUT_POS/INPUT_INITIAL_OFFSET as crate-root
// items (since in the `ziskos` crate, core/lib.rs *is* the crate root). Mirror
// that here so the same source compiles in both crates.
pub(crate) use core::{INPUT_INITIAL_OFFSET, INPUT_POS};

// Add hints-specific modules that only exist in ziskos-hints
pub mod handlers;
