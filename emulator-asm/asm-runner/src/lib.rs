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
mod naming;
mod shmem_reader;
mod shmem_sys;
mod shmem_utils;
mod shmem_writer;

// Internal layout/header structs — not part of the public API.
pub(crate) use asm_mo::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm_mo_runner::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_mo_runner_stub::*;
pub(crate) use asm_mt::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm_mt_runner::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_mt_runner_stub::*;
// `AsmRHData` is public API (read by sm-rom); `AsmRHHeader` is an internal layout struct.
pub use asm_rh::AsmRHData;
pub(crate) use asm_rh::AsmRHHeader;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm_rh_runner::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_rh_runner_stub::*;
pub use asm_runner::*;
pub use asm_services::*;
pub use control_shmem::*;
// `HintsFile` is an internal alternative sink — not part of the public API.
pub(crate) use hints_file::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use hints_shmem::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use hints_shmem_stub::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use inputs_shmem::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use inputs_shmem_stub::*;
// Low-level shmem primitives + naming — crate-internal, not part of the public API.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) use multi_shmem::*;
pub(crate) use naming::*;
pub(crate) use shmem_reader::*;
pub(crate) use shmem_utils::*;
pub(crate) use shmem_writer::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TRACE_INITIAL_SIZE: usize = 0x180000000; // 6GB
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TRACE_DELTA_SIZE: usize = 0x080000000; // 2GB
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TRACE_MAX_SIZE: usize = 0x1000000000; // 64GB

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const SEM_CHUNK_DONE_WAIT_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn drain_chunk_done(sem: &mut named_sem::NamedSemaphore) -> u64 {
    let mut swept = 0;
    while sem.try_wait().is_ok() {
        swept += 1;
    }
    swept
}
