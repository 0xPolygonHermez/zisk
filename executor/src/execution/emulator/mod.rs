//! Emulator backends used by [`crate::execution::ExecutionPhase`].
//!
//! Both backends expose an inherent `execute` method returning a uniform
//! [`crate::execution::output::ExecutionOutput`]; backend-specific async
//! work (ASM-only MO + RH handles) is encapsulated in
//! [`crate::execution::output::BackendArtifacts`]. Dispatch is via the
//! `EmulatorBackend` enum inside `ExecutionPhase`, not via dyn trait.
//!
//! Cross-platform shape: `EmulatorAsm` is provided uniformly on every
//! target. On Linux x86_64 it's the real ASM-backed implementation; on
//! other targets a sibling stub of the same name panics with a clear
//! message if any of its methods is actually invoked. This keeps
//! `ExecutionPhase` platform-agnostic.

pub mod asm;
pub mod rust;

pub use asm::*;
pub use rust::*;
