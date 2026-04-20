extern crate libc;

mod asm_mo;
mod asm_mo_runner;
mod asm_mt;
mod asm_mt_runner;
mod asm_rh;
mod asm_rh_runner;
mod asm_runner;
mod asm_services;
mod control_shmem;
mod hints_file;
mod hints_shmem;
mod inputs_shmem;
mod multi_shmem;
mod shmem_reader;
mod shmem_utils;
mod shmem_writer;

pub use asm_mo::*;
pub use asm_mo_runner::*;
pub use asm_mt::*;
pub use asm_mt_runner::*;
pub use asm_rh::*;
pub use asm_rh_runner::*;
pub use asm_runner::*;
pub use asm_services::*;
pub use control_shmem::*;
pub use hints_file::*;
pub use hints_shmem::*;
pub use inputs_shmem::*;
pub use multi_shmem::*;
pub use shmem_reader::*;
pub use shmem_utils::*;
pub use shmem_writer::*;

pub(crate) const TRACE_INITIAL_SIZE: usize = 0x180000000; // 6GB
pub(crate) const TRACE_DELTA_SIZE: usize = 0x080000000; // 2GB
pub(crate) const TRACE_MAX_SIZE: usize = 0x1000000000; // 64GB

const SEM_CHUNK_DONE_WAIT_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

fn build_name(
    prefix: &str,
    port: u16,
    asm_service: AsmService,
    local_rank: i32,
    suffix: &str,
) -> String {
    format!(
        "{}{}_{}_{}",
        prefix,
        AsmServices::shmem_prefix(port, local_rank),
        asm_service.as_str(),
        suffix
    )
}

fn build_name2(prefix: &str, port: u16, local_rank: i32, suffix: &str) -> String {
    format!("{}{}_{}", prefix, AsmServices::shmem_prefix(port, local_rank), suffix)
}

fn build_shmem_name(port: u16, asm_service: AsmService, local_rank: i32, suffix: &str) -> String {
    build_name("", port, asm_service, local_rank, suffix)
}

fn build_shmem_name2(port: u16, local_rank: i32, suffix: &str) -> String {
    build_name2("", port, local_rank, suffix)
}

fn build_sem_name(port: u16, asm_service: AsmService, local_rank: i32, suffix: &str) -> String {
    build_name("/", port, asm_service, local_rank, suffix)
}

pub fn shmem_input_name(port: u16, local_rank: i32) -> String {
    build_shmem_name2(port, local_rank, "input")
}

pub fn shmem_input_avail_name(port: u16, local_rank: i32) -> String {
    build_shmem_name2(port, local_rank, "input_avail")
}

/// Semaphore name for input availability (per service)
pub fn sem_input_avail_name(port: u16, asm_service: AsmService, local_rank: i32) -> String {
    build_sem_name(port, asm_service, local_rank, "input_avail")
}

/// Shared memory name for precompile hints data
pub fn shmem_precompile_name(port: u16, local_rank: i32) -> String {
    build_shmem_name2(port, local_rank, "precompile")
}

/// Shared memory name for precompile hints data
pub fn sem_available_name(port: u16, asm_service: AsmService, local_rank: i32) -> String {
    build_sem_name(port, asm_service, local_rank, "prec_avail")
}

/// Shared memory name for precompile hints data
pub fn sem_read_name(port: u16, asm_service: AsmService, local_rank: i32) -> String {
    build_sem_name(port, asm_service, local_rank, "prec_read")
}

/// Shared memory name for precompile hints data control
pub fn shmem_control_writer_name(port: u16, local_rank: i32) -> String {
    build_shmem_name2(port, local_rank, "control_input")
}

pub fn shmem_control_reader_name(port: u16, asm_service: AsmService, local_rank: i32) -> String {
    build_shmem_name(port, asm_service, local_rank, "control_output")
}

pub fn shmem_output_name(
    port: u16,
    asm_service: AsmService,
    local_rank: i32,
    suffix: Option<isize>,
) -> String {
    if let Some(suffix) = suffix {
        format!(
            "{}_{}_output_{}",
            AsmServices::shmem_prefix(port, local_rank),
            asm_service.as_str(),
            suffix
        )
    } else {
        build_shmem_name(port, asm_service, local_rank, "output")
    }
}

pub fn sem_chunk_done_name(port: u16, asm_service: AsmService, local_rank: i32) -> String {
    build_sem_name(port, asm_service, local_rank, "chunk_done")
}
