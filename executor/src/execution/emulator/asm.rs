//! ASM-emulator support modules.
//!
//! Cross-platform pieces (`resources`, `transport`, `supervisor`,
//! `mt_chunk`) compile everywhere via internal `#[cfg]` gates — they
//! contain stub-free type definitions plus inherent methods that touch
//! Linux-x86_64-only `asm_runner` internals. Only the inherent-impl
//! glue that drives `AsmRunnerMT::run_and_count` (i.e. [`emulator`])
//! is gated to that target; on other targets [`stub`] supplies an
//! `EmulatorAsm` of the same shape whose methods panic with a clear
//! "Linux x86_64 only" message. Either way, `EmulatorAsm` is in scope
//! uniformly so [`crate::execution::ExecutionPhase`] stays
//! platform-agnostic.

pub mod mt_chunk;
pub mod resources;
pub mod supervisor;
pub mod transport;

pub use mt_chunk::*;
pub use resources::*;
pub use supervisor::*;
pub use transport::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod emulator;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use emulator::*;

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod stub;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use stub::*;
