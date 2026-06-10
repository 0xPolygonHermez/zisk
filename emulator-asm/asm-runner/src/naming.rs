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
pub(crate) const NAMESPACE: &str = "ZISK";

fn build_service_shmem_name(prefix: &str, asm_service: AsmService, suffix: &str) -> String {
    format!("{}_{}_{}", prefix, asm_service.as_str(), suffix)
}

fn build_shmem_name(prefix: &str, suffix: &str) -> String {
    format!("{}_{}", prefix, suffix)
}

fn build_sem_name(prefix: &str, asm_service: AsmService, suffix: &str) -> String {
    format!("/{}_{}_{}", prefix, asm_service.as_str(), suffix)
}

/// Shared memory name for the input data (shared across services).
pub(crate) fn shmem_input_name(shm_prefix: &str) -> String {
    build_shmem_name(shm_prefix, "input")
}

/// Semaphore name for input availability (per service)
pub(crate) fn sem_input_avail_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "input_avail")
}

/// Shared memory name for precompile hints data.
pub(crate) fn shmem_precompile_name(prefix: &str) -> String {
    build_shmem_name(prefix, "precompile")
}

/// Semaphore name for precompile hints data availability (per service).
pub(crate) fn sem_prec_available_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "prec_avail")
}

/// Semaphore name for precompile hints data read (per service).
pub(crate) fn sem_prec_read_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "prec_read")
}

/// Shared memory name for precompile hints data control.
pub(crate) fn shmem_control_input_name(prefix: &str) -> String {
    build_shmem_name(prefix, "control_input")
}

/// Shared memory name for control output (per service).
pub(crate) fn shmem_control_output_name(prefix: &str, asm_service: AsmService) -> String {
    build_service_shmem_name(prefix, asm_service, "control_output")
}

/// Shared memory name for the main output (per service, optional numeric suffix for multiple outputs).
pub(crate) fn shmem_output_name(
    prefix: &str,
    asm_service: AsmService,
    suffix: Option<isize>,
) -> String {
    if let Some(n) = suffix {
        build_service_shmem_name(prefix, asm_service, &format!("output_{n}"))
    } else {
        build_service_shmem_name(prefix, asm_service, "output")
    }
}

/// Semaphore name for chunk completion (per service).
pub(crate) fn sem_chunk_done_name(prefix: &str, asm_service: AsmService) -> String {
    build_sem_name(prefix, asm_service, "chunk_done")
}

/// True if `file_name` (a `/dev/shm` entry) is a ZisK shmem segment, i.e. it
/// starts with `{NAMESPACE}_`. Matches the prefix built in `AsmServices::new`.
pub(crate) fn is_zisk_shmem_file(file_name: &str) -> bool {
    file_name.strip_prefix(NAMESPACE).is_some_and(|rest| rest.starts_with('_'))
}

/// True if `file_name` is the `/dev/shm` backing file of a ZisK named
/// semaphore, i.e. `sem.{NAMESPACE}_...`.
pub(crate) fn is_zisk_sem_file(file_name: &str) -> bool {
    file_name.strip_prefix("sem.").is_some_and(is_zisk_shmem_file)
}

/// Convert a `/dev/shm` semaphore backing-file name (`sem.FOO`) to the POSIX
/// semaphore name (`/FOO`) accepted by `sem_unlink`. Returns `None` if the
/// name is not a `sem.` entry.
pub(crate) fn sem_file_to_posix_name(file_name: &str) -> Option<String> {
    file_name.strip_prefix("sem.").map(|rest| format!("/{rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AsmService;

    #[test]
    fn shmem_names_match_dev_shm_suffixes() {
        let p = "ZISK_42_0";
        assert_eq!(shmem_input_name(p), "ZISK_42_0_input");
        assert_eq!(shmem_precompile_name(p), "ZISK_42_0_precompile");
        assert_eq!(shmem_control_input_name(p), "ZISK_42_0_control_input");
        assert_eq!(shmem_control_output_name(p, AsmService::MO), "ZISK_42_0_MO_control_output");
        assert_eq!(shmem_output_name(p, AsmService::MT, None), "ZISK_42_0_MT_output");
        assert_eq!(shmem_output_name(p, AsmService::MT, Some(3)), "ZISK_42_0_MT_output_3");
    }

    #[test]
    fn sem_names_are_posix_absolute_and_per_service() {
        let p = "ZISK_42_h_0";
        assert_eq!(sem_input_avail_name(p, AsmService::MO), "/ZISK_42_h_0_MO_input_avail");
        assert_eq!(sem_prec_available_name(p, AsmService::RH), "/ZISK_42_h_0_RH_prec_avail");
        assert_eq!(sem_prec_read_name(p, AsmService::RH), "/ZISK_42_h_0_RH_prec_read");
        assert_eq!(sem_chunk_done_name(p, AsmService::MT), "/ZISK_42_h_0_MT_chunk_done");
    }

    #[test]
    fn shmem_predicate_requires_namespace_and_underscore_boundary() {
        assert!(is_zisk_shmem_file("ZISK_42_0_input"));
        assert!(is_zisk_shmem_file(&format!("{NAMESPACE}_x")));
        // No underscore right after the namespace → not ours (guards against `ZISKFOO`).
        assert!(!is_zisk_shmem_file("ZISKX"));
        assert!(!is_zisk_shmem_file("OTHER_42_0"));
        // A semaphore backing file is not a shmem segment.
        assert!(!is_zisk_shmem_file("sem.ZISK_42_0"));
    }

    #[test]
    fn sem_file_predicate_and_posix_conversion() {
        assert!(is_zisk_sem_file("sem.ZISK_42_0_MO_chunk_done"));
        assert!(!is_zisk_sem_file("ZISK_42_0_input")); // shmem, not a sem backing file
        assert_eq!(sem_file_to_posix_name("sem.ZISK_42_0_x").as_deref(), Some("/ZISK_42_0_x"));
        assert_eq!(sem_file_to_posix_name("ZISK_42_0_x"), None);
    }

    #[test]
    fn builder_output_is_recognized_by_the_cleanup_scanner() {
        // The /dev/shm janitor must recognize every name the builders produce,
        // otherwise leaked segments would never be reaped.
        let p = format!("{NAMESPACE}_99_0");
        assert!(is_zisk_shmem_file(&shmem_input_name(&p)));
        assert!(is_zisk_shmem_file(&shmem_output_name(&p, AsmService::RH, Some(0))));

        // A POSIX sem name "/X" has the /dev/shm backing file "sem.X"; the scanner
        // must round-trip it back to the POSIX name for sem_unlink.
        let posix = sem_chunk_done_name(&p, AsmService::MO); // "/ZISK_99_0_MO_chunk_done"
        let backing = format!("sem.{}", &posix[1..]);
        assert!(is_zisk_sem_file(&backing));
        assert_eq!(sem_file_to_posix_name(&backing).as_deref(), Some(posix.as_str()));
    }
}
