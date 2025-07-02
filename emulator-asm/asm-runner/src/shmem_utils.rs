use libc::{mmap, munmap, shm_open, MAP_FAILED, MAP_SHARED};
use std::{ffi::CString, io, os::raw::c_void, ptr};

pub fn open_shmem(name: &str, flags: i32, mode: u32) -> i32 {
    let c_name = CString::new(name).expect("CString::new failed");
    let fd = unsafe { shm_open(c_name.as_ptr(), flags, mode) };
    if fd == -1 {
        let err = io::Error::last_os_error();
        panic!("shm_open('{name}') failed: {err}");
    }
    fd
}

pub fn map(fd: i32, size: usize, prot: i32, unlock_mapped_memory: bool, desc: &str) -> *mut c_void {
    let mut flags = MAP_SHARED;
    if !unlock_mapped_memory {
        flags |= libc::MAP_LOCKED;
    }
    let mapped = unsafe { mmap(ptr::null_mut(), size, prot, flags, fd, 0) };
    if mapped == MAP_FAILED {
        let err = io::Error::last_os_error();
        panic!("mmap failed for '{desc}': {err:?} ({size} bytes)");
    }
    mapped
}

/// Unmaps memory at the given raw pointer.
///
/// # Safety
/// The caller must ensure that:
/// - `ptr` was returned by a successful call to `mmap`,
/// - `size` matches the original mapped size,
/// - the region pointed to by `ptr` is not already unmapped.
pub unsafe fn unmap(ptr: *mut c_void, size: usize) {
    if munmap(ptr, size) != 0 {
        tracing::error!("munmap failed: {:?}", io::Error::last_os_error());
    }
}
