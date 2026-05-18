//! Emulator backends used by [`crate::execution::ExecutionPhase`].
//!
//! Both backends expose an inherent `execute` method returning a uniform
//! [`crate::execution::output::ExecutionOutput`]; backend-specific async
//! work (ASM-only MO + RH handles) is encapsulated in
//! [`crate::execution::output::BackendArtifacts`]. Dispatch is via the
//! `EmulatorBackend` enum inside `ExecutionPhase`, not via dyn trait.

pub mod rust;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub mod asm;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub mod asm_stub;

pub use rust::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_stub::*;
