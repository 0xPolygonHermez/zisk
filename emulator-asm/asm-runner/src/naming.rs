//! Shared-memory and semaphore naming — the wire contract with the C/ASM side.
//!
//! Every name produced here must match what the `ziskemuasm` C binary expects
//! (`shm_prefix`/`sem_prefix` plus a fixed per-purpose suffix). Any change to a
//! suffix, separator, or the [`NAMESPACE`] root MUST be mirrored on the C side
//! or the two processes will silently fail to find each other's segments.
//!
//! The cleanup scanners ([`is_zisk_shmem_file`], [`is_zisk_sem_file`]) match on
//! the same [`NAMESPACE`] the prefixes are built from, so the builder and the
//! scanner cannot drift apart.

use crate::AsmService;

/// Root namespace shared by every ZisK shmem segment and semaphore.
///
/// `shm_prefix`/`sem_prefix` are built as `{NAMESPACE}_{pid}_...` (see
/// `AsmServices::new`); the `/dev/shm` cleanup scanners match on it.
pub const NAMESPACE: &str = "ZISK";

fn build_service_shmem_name(prefix: &str, asm_service: AsmService, suffix: &str) -> String {
    format!("{}_{}_{}", prefix, asm_service.as_str(), suffix)
}

fn build_shmem_name(prefix: &str, suffix: &str) -> String {
    format!("{}_{}", prefix, suffix)
}

fn build_sem_name(prefix: &str, asm_service: AsmService, suffix: &str) -> String {
    format!("/{}_{}_{}", prefix, asm_service.as_str(), suffix)
}

pub fn shmem_input_name(shm_prefix: &str) -> String {
    build_shmem_name(shm_prefix, "input")
}

/// Semaphore name for input availability (per service)
pub fn sem_input_avail_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "input_avail")
}

/// Per-service shared memory name for precompile hints data.
/// Each ASM service has its own precompile shmem; Rust writes the same data to all of them.
pub fn shmem_precompile_name(prefix: &str) -> String {
    build_shmem_name(prefix, "precompile")
}

/// Semaphore name for precompile hints data availability (per service)
pub fn sem_prec_available_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "prec_avail")
}

/// Semaphore name for precompile hints data read (per service)
pub fn sem_prec_read_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "prec_read")
}

/// Shared memory name for precompile hints data control
pub fn shmem_control_input_name(prefix: &str) -> String {
    build_shmem_name(prefix, "control_input")
}

pub fn shmem_control_output_name(prefix: &str, asm_service: AsmService) -> String {
    build_service_shmem_name(prefix, asm_service, "control_output")
}

pub fn shmem_output_name(prefix: &str, asm_service: AsmService, suffix: Option<isize>) -> String {
    if let Some(n) = suffix {
        build_service_shmem_name(prefix, asm_service, &format!("output_{n}"))
    } else {
        build_service_shmem_name(prefix, asm_service, "output")
    }
}

pub fn sem_chunk_done_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "chunk_done")
}

/// True if `file_name` (a `/dev/shm` entry) is a ZisK shmem segment, i.e. it
/// starts with `{NAMESPACE}_`. Matches the prefix built in `AsmServices::new`.
pub fn is_zisk_shmem_file(file_name: &str) -> bool {
    file_name.strip_prefix(NAMESPACE).is_some_and(|rest| rest.starts_with('_'))
}

/// True if `file_name` is the `/dev/shm` backing file of a ZisK named
/// semaphore, i.e. `sem.{NAMESPACE}_...`.
pub fn is_zisk_sem_file(file_name: &str) -> bool {
    file_name.strip_prefix("sem.").is_some_and(is_zisk_shmem_file)
}

/// Convert a `/dev/shm` semaphore backing-file name (`sem.FOO`) to the POSIX
/// semaphore name (`/FOO`) accepted by `sem_unlink`. Returns `None` if the
/// name is not a `sem.` entry.
pub fn sem_file_to_posix_name(file_name: &str) -> Option<String> {
    file_name.strip_prefix("sem.").map(|rest| format!("/{rest}"))
}
