//! The `asm-runner` crate provides the core logic for managing the assembly runner process,
//! including shared memory management, synchronization primitives,
//! and communication with the C++ side of the emulator.
//! It defines the main types and functions used by the assembly runner to execute assembly code 
//! and interact with the rest of the emulator.

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]
// On non-Linux-x86_64 the ASM runner is a stub shell: the real runners and the
// shared-memory machinery they drive are `cfg`-gated out, so most of the crate's
// types/fns have no caller there. Silence the resulting dead-code/unused-import
// noise off the supported platform; Linux-x86_64 keeps full lint enforcement.
#![cfg_attr(
    not(all(target_os = "linux", target_arch = "x86_64")),
    allow(dead_code, unused_imports)
)]

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
// `HintsFile` is a public file-based StreamSink alternative to HintsShmem.
pub use hints_file::*;
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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use super::*;
    use named_sem::NamedSemaphore;

    #[test]
    fn drain_chunk_done_sweeps_all_pending_posts() {
        let name = format!("/ZISK_unittest_drain_{}", std::process::id());
        let mut sem = NamedSemaphore::create(&name, 0).unwrap();
        for _ in 0..3 {
            sem.post().unwrap();
        }
        assert_eq!(drain_chunk_done(&mut sem), 3, "should sweep the 3 pending posts");
        assert_eq!(drain_chunk_done(&mut sem), 0, "nothing left to sweep");

        let c = std::ffi::CString::new(name).unwrap();
        unsafe { libc::sem_unlink(c.as_ptr()) };
    }
}
