use libc::{
    close, mmap, munmap, shm_open, shm_unlink, MAP_FAILED, MAP_SHARED, PROT_READ, PROT_WRITE,
    S_IRUSR, S_IWUSR,
};
use std::{ffi::CString, io, mem::ManuallyDrop, os::raw::c_void, ptr};

use anyhow::Result;

pub enum AsmSharedMemoryMode {
    ReadOnly,
    ReadWrite,
}

pub struct AsmSharedMemory<H: AsmShmemHeader> {
    fd: i32,
    mapped_ptr: *mut c_void,
    mapped_size: usize,
    shmem_name: String,
    header: ManuallyDrop<H>,
}

unsafe impl<H: AsmShmemHeader> Send for AsmSharedMemory<H> {}
unsafe impl<H: AsmShmemHeader> Sync for AsmSharedMemory<H> {}

pub trait AsmShmemHeader {
    fn allocated_size(&self) -> u64;
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl<H: AsmShmemHeader> Drop for AsmSharedMemory<H> {
    fn drop(&mut self) {
        self.unmap().unwrap_or_else(|err| {
            tracing::error!("Failed to unmap shared memory '{}': {}", self.shmem_name, err)
        });
        unsafe { close(self.fd) };
    }
}

impl<H: AsmShmemHeader> AsmSharedMemory<H> {
    // pub fn open(name: &str, flags: i32, mode: u32) -> Result<Self> {
    //     if name.is_empty() {
    //         return Err(anyhow::anyhow!("Shared memory name {name} cannot be empty"));
    //     }

    //     let c_name = CString::new(name).expect("CString::new failed");
    //     let fd = unsafe { shm_open(c_name.as_ptr(), flags, mode) };
    //     if fd == -1 {
    //         let err = io::Error::last_os_error();
    //         return Err(anyhow::anyhow!("shm_open('{name}') failed: {err}"));
    //     }

    //     Ok(Self { fd, mapped_ptr: None, mapped_size: 0, shmem_name: name.to_string() })
    // }

    // pub fn map(&mut self, size: usize, prot: i32, unlock_mapped_memory: bool) -> Result<()> {
    //     // Ensure the size is valid
    //     assert!(size > 0, "Size must be greater than zero for shared memory mapping");

    //     if self.is_mapped() {
    //         return Err(anyhow::anyhow!(
    //             "Shared memory '{}' is already mapped, unwrapping first",
    //             self.shmem_name
    //         ));
    //     }

    //     let mut flags = MAP_SHARED;
    //     if !unlock_mapped_memory {
    //         flags |= libc::MAP_LOCKED;
    //     }

    //     let mapped = unsafe { mmap(ptr::null_mut(), size, prot, flags, self.fd, 0) };
    //     if mapped == MAP_FAILED {
    //         let err = io::Error::last_os_error();
    //         panic!("mmap failed for '{}': {err:?} ({size} bytes)", self.shmem_name);
    //     }

    //     self.mapped_ptr = Some(mapped);
    //     self.mapped_size = size;

    //     Ok(())
    // }

    pub fn open_and_map(
        name: &str,
        mode: AsmSharedMemoryMode,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        unsafe {
            if name.is_empty() {
                return Err(anyhow::anyhow!("Shared memory name {name} cannot be empty"));
            }

            let c_name = CString::new(name)
                .map_err(|_| anyhow::anyhow!("Shared memory name contains null byte"))?;

            let oflag = match mode {
                AsmSharedMemoryMode::ReadOnly => libc::O_RDONLY,
                AsmSharedMemoryMode::ReadWrite => libc::O_RDWR,
            };

            let fd = shm_open(c_name.as_ptr(), oflag, S_IRUSR | S_IWUSR);
            if fd == -1 {
                let err = io::Error::last_os_error();
                return Err(anyhow::anyhow!("shm_open('{name}') failed: {err}"));
            }

            // Unlink the shared memory object to ensure it is removed after use
            // This is necessary to avoid leaving stale shared memory objects
            // in the system, especially if the program crashes or exits unexpectedly.
            // Note: This does not affect the current mapping, it just ensures that
            // the shared memory object is removed from the filesystem namespace.
            if shm_unlink(c_name.as_ptr()) != 0 {
                let err = io::Error::last_os_error();
                close(fd);
                return Err(anyhow::anyhow!("shm_unlink('{name}') failed: {err}"));
            }

            let prot = match mode {
                AsmSharedMemoryMode::ReadOnly => PROT_READ,
                AsmSharedMemoryMode::ReadWrite => PROT_READ | PROT_WRITE,
            };

            let mut flags = MAP_SHARED;
            if !unlock_mapped_memory {
                flags |= libc::MAP_LOCKED;
            }

            let size_header = size_of::<H>();
            let mapped_ptr = mmap(ptr::null_mut(), size_header, prot, flags, fd, 0);
            if mapped_ptr == MAP_FAILED {
                let err = io::Error::last_os_error();
                close(fd);
                return Err(anyhow::anyhow!(
                    "mmap failed for '{name}': {err:?} ({size_header} bytes)",
                ));
            }

            // let header = Self::map_header(&self);
            let header = (mapped_ptr as *const H).read();
            let allocated_size = header.allocated_size() as usize;

            // Ensure the size is valid
            if allocated_size == 0 {
                if munmap(mapped_ptr, size_header) != 0 {
                    let err = io::Error::last_os_error();
                    return Err(anyhow::anyhow!("munmap failed for '{name}': {err}"));
                }

                close(fd);

                return Err(anyhow::anyhow!("Shared memory '{}' has zero allocated size", name));
            }

            if munmap(mapped_ptr, size_header) != 0 {
                let err = io::Error::last_os_error();
                close(fd);
                return Err(anyhow::anyhow!("munmap failed for '{name}': {err}"));
            }

            let mapped_ptr = mmap(ptr::null_mut(), allocated_size, prot, flags, fd, 0);

            if mapped_ptr == MAP_FAILED {
                let err = io::Error::last_os_error();
                close(fd);
                return Err(anyhow::anyhow!(
                    "mmap failed for '{name}': {err:?} ({size_header} bytes)",
                ));
            }

            Ok(Self {
                fd,
                mapped_ptr,
                mapped_size: allocated_size,
                shmem_name: name.to_string(),
                header: ManuallyDrop::new(header),
            })
        }
    }

    pub fn unmap(&mut self) -> Result<()> {
        unsafe {
            if munmap(self.mapped_ptr, self.mapped_size) != 0 {
                tracing::error!("munmap failed: {:?}", io::Error::last_os_error());
                return Err(anyhow::anyhow!(
                    "munmap failed for '{}': {}",
                    self.shmem_name,
                    io::Error::last_os_error()
                ));
            }
            tracing::trace!("Unmapped shared memory '{}'", self.shmem_name);
        }
        self.mapped_ptr = ptr::null_mut();
        self.mapped_size = 0;

        Ok(())
    }

    pub fn map_header(&self) -> H {
        if !self.is_mapped() {
            panic!("Shared memory '{}' is not mapped, cannot read header", self.shmem_name);
        }

        unsafe { (self.mapped_ptr as *const H).read() }
    }

    pub fn is_mapped(&self) -> bool {
        !self.mapped_ptr.is_null()
    }

    pub fn header_ptr(&self) -> *mut c_void {
        self.mapped_ptr
    }

    pub fn data_ptr(&self) -> *mut c_void {
        // Skip the header size to get the data pointer
        unsafe { self.mapped_ptr.add(size_of::<H>()) }
    }

    pub fn header(&self) -> &H {
        &self.header
    }
}

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
