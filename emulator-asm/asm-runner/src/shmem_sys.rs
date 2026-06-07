//! Low-level POSIX shared-memory syscall wrappers.
//!
//! Single source of truth for `shm_open` + `mmap` across the crate. These two
//! operations were previously reimplemented in `shmem_writer`, `shmem_reader`,
//! `shmem_utils` and `multi_shmem` with subtly divergent error handling (some
//! panicked, some returned `Result`). Everything funnels through here now.
//!
//! All segments are opened *without* `O_CREAT` — the C/ASM side creates them —
//! so the access-mode bits passed to `shm_open` are ignored by the kernel and a
//! fixed `S_IRUSR | S_IWUSR` is used internally.
//!
//! On non-Linux-x86_64 targets every constructor of these primitives is
//! `cfg`-gated away (see `lib.rs` and the executor's asm path), so the stubs
//! below are dead code that exists only to keep the crate compiling.

use std::io;
use std::os::raw::c_void;

/// Open an existing POSIX shared-memory object by name and return its fd.
///
/// `flags` is the `oflag` for `shm_open` (e.g. `libc::O_RDONLY`, `libc::O_RDWR`).
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn open(name: &str, flags: i32) -> io::Result<i32> {
    use libc::{shm_open, S_IRUSR, S_IWUSR};

    let c_name = std::ffi::CString::new(name).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("shared memory name '{name}' contains a null byte"),
        )
    })?;

    let fd = unsafe { shm_open(c_name.as_ptr(), flags, S_IRUSR | S_IWUSR) };
    if fd == -1 {
        let errno = unsafe { *libc::__errno_location() };
        let err = io::Error::from_raw_os_error(errno);
        return Err(io::Error::new(err.kind(), format!("shm_open('{name}') failed: {err}")));
    }
    Ok(fd)
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub(crate) fn open(_name: &str, _flags: i32) -> io::Result<i32> {
    Ok(0)
}

/// `mmap` `size` bytes of `fd` with protection `prot`.
///
/// When `lock` is true the mapping uses `MAP_LOCKED` so its pages stay resident
/// (the crate's perf-sensitive shared buffers rely on this); pass `false` to opt
/// out — this is the `unlock_mapped_memory` knob, negated.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn map(
    fd: i32,
    size: usize,
    prot: i32,
    lock: bool,
    name: &str,
) -> io::Result<*mut c_void> {
    use libc::{mmap, MAP_FAILED, MAP_SHARED};

    let mut flags = MAP_SHARED;
    if lock {
        flags |= libc::MAP_LOCKED;
    }

    let mapped = unsafe { mmap(std::ptr::null_mut(), size, prot, flags, fd, 0) };
    if mapped == MAP_FAILED {
        let err = io::Error::last_os_error();
        return Err(io::Error::new(
            err.kind(),
            format!("mmap failed for '{name}': {err} ({size} bytes)"),
        ));
    }
    Ok(mapped)
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub(crate) fn map(
    _fd: i32,
    _size: usize,
    _prot: i32,
    _lock: bool,
    _name: &str,
) -> io::Result<*mut c_void> {
    Ok(std::ptr::null_mut())
}
