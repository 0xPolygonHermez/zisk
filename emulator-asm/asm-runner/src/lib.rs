extern crate libc;

mod asm_min_traces;
mod asm_rom_histogram;
mod asm_runner;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod asm_min_traces_runner_linux;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod asm_min_traces_runner_stub;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod asm_rom_histogram_runner_linux;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod asm_rom_histogram_runner_stub;

mod asm_min_traces_runner {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub use super::asm_min_traces_runner_linux::*;
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    pub use super::asm_min_traces_runner_stub::*;
}

mod asm_rom_histogram_runner {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub use super::asm_rom_histogram_runner_linux::*;
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    pub use super::asm_rom_histogram_runner_stub::*;
}

pub use asm_min_traces::*;
pub use asm_min_traces_runner::*;
pub use asm_rom_histogram::*;
pub use asm_rom_histogram_runner::*;
pub use asm_runner::*;
