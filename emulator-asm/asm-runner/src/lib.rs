extern crate libc;

mod asm_mo;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod asm_mo_runner;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod asm_mo_runner_stub;
mod asm_mt;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod asm_mt_runner;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod asm_mt_runner_stub;
mod asm_rh;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod asm_rh_runner;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod asm_rh_runner_stub;
mod asm_runner;
mod asm_services;
mod control_shmem;
mod hints_file;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod hints_shmem;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod hints_shmem_stub;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod inputs_shmem;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod inputs_shmem_stub;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod multi_shmem;
mod shmem_reader;
mod shmem_utils;
mod shmem_writer;

pub use asm_mo::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm_mo_runner::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_mo_runner_stub::*;
pub use asm_mt::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm_mt_runner::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_mt_runner_stub::*;
pub use asm_rh::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm_rh_runner::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_rh_runner_stub::*;
pub use asm_runner::*;
pub use asm_services::*;
pub use control_shmem::*;
pub use hints_file::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use hints_shmem::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use hints_shmem_stub::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use inputs_shmem::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use inputs_shmem_stub::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use multi_shmem::*;
pub use shmem_reader::*;
pub use shmem_utils::*;
pub use shmem_writer::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TRACE_INITIAL_SIZE: usize = 0x180000000; // 6GB
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TRACE_DELTA_SIZE: usize = 0x080000000; // 2GB
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TRACE_MAX_SIZE: usize = 0x1000000000; // 64GB

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const SEM_CHUNK_DONE_WAIT_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

fn build_shmem_name(prefix: &str, asm_service: AsmService, suffix: &str) -> String {
    format!("{}_{}_{}", prefix, asm_service.as_str(), suffix)
}

fn build_shmem_name2(prefix: &str, suffix: &str) -> String {
    format!("{}_{}", prefix, suffix)
}

fn build_sem_name(prefix: &str, asm_service: AsmService, suffix: &str) -> String {
    format!("/{}_{}_{}", prefix, asm_service.as_str(), suffix)
}

pub fn shmem_input_name(shm_prefix: &str) -> String {
    build_shmem_name2(shm_prefix, "input")
}

/// Semaphore name for input availability (per service)
pub fn sem_input_avail_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "input_avail")
}

/// Per-service shared memory name for precompile hints data.
/// Each ASM service has its own precompile shmem; Rust writes the same data to all of them.
pub fn shmem_precompile_name(prefix: &str, service: AsmService) -> String {
    build_shmem_name(prefix, service, "precompile")
}

/// Shared memory name for precompile hints data
pub fn sem_available_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "prec_avail")
}

/// Shared memory name for precompile hints data
pub fn sem_read_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "prec_read")
}

/// Shared memory name for precompile hints data control
pub fn shmem_control_writer_name(shm_prefix: &str) -> String {
    build_shmem_name2(shm_prefix, "control_input")
}

pub fn shmem_control_reader_name(prefix: &str, asm_service: AsmService) -> String {
    build_shmem_name(prefix, asm_service, "control_output")
}

pub fn shmem_output_name(prefix: &str, asm_service: AsmService, suffix: Option<isize>) -> String {
    if let Some(n) = suffix {
        build_shmem_name(prefix, asm_service, &format!("output_{n}"))
    } else {
        build_shmem_name(prefix, asm_service, "output")
    }
}

pub fn sem_chunk_done_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "chunk_done")
}
