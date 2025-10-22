use libc::{
    c_uint, close, mmap, munmap, shm_open, shm_unlink, MAP_FAILED, MAP_SHARED, PROT_READ,
    PROT_WRITE, S_IRUSR, S_IWUSR,
};
use std::{
    ffi::CString,
    fmt::Debug,
    fs, io,
    os::raw::c_void,
    path::Path,
    ptr,
    sync::atomic::{fence, Ordering},
};
use tracing::debug;

use anyhow::Result;

use crate::{AsmInputC2, AsmService, AsmServices};

pub enum AsmSharedMemoryMode {
    ReadOnly,
    ReadWrite,
}

pub struct AsmSharedMemory<H: AsmShmemHeader> {
    _fd: i32,
    mapped_ptr: *mut c_void,
    mapped_size: usize,
    shmem_name: String,
    _phantom: std::marker::PhantomData<H>,
}

unsafe impl<H: AsmShmemHeader> Send for AsmSharedMemory<H> {}
unsafe impl<H: AsmShmemHeader> Sync for AsmSharedMemory<H> {}

pub trait AsmShmemHeader: Debug {
    fn allocated_size(&self) -> u64;
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl<H: AsmShmemHeader> Drop for AsmSharedMemory<H> {
    fn drop(&mut self) {
        self.unmap().unwrap_or_else(|err| {
            tracing::error!("Failed to unmap shared memory '{}': {}", self.shmem_name, err)
        });
        unsafe { close(self._fd) };
    }
}

impl<H: AsmShmemHeader> AsmSharedMemory<H> {
    pub fn open_and_map(name: &str, _unlock_mapped_memory: bool) -> Result<Self> {
        unsafe {
            if name.is_empty() {
                return Err(anyhow::anyhow!("Shared memory name {name} cannot be empty"));
            }

            let c_name = CString::new(name)
                .map_err(|_| anyhow::anyhow!("Shared memory name contains null byte"))?;

            let fd =
                shm_open(c_name.as_ptr(), libc::O_RDONLY, S_IRUSR as c_uint | S_IWUSR as c_uint);
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

            #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
            let flags = MAP_SHARED;
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            let mut flags = MAP_SHARED;
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            if !_unlock_mapped_memory {
                flags |= libc::MAP_LOCKED;
            }

            let size_header = size_of::<H>();
            let mapped_ptr = mmap(ptr::null_mut(), size_header, PROT_READ, flags, fd, 0);
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

            let mapped_ptr = mmap(ptr::null_mut(), allocated_size, PROT_READ, flags, fd, 0);

            if mapped_ptr == MAP_FAILED {
                let err = io::Error::last_os_error();
                close(fd);
                return Err(anyhow::anyhow!(
                    "mmap failed for '{name}': {err:?} ({size_header} bytes)",
                ));
            }

            Ok(Self {
                _fd: fd,
                mapped_ptr,
                mapped_size: allocated_size,
                shmem_name: name.to_string(),
                _phantom: std::marker::PhantomData::<H>,
            })
        }
    }

    pub fn remap(&mut self, new_size: usize) -> Result<()> {
        if !self.is_mapped() {
            return Err(anyhow::anyhow!(
                "Shared memory '{}' is not currently mapped, cannot remap",
                self.shmem_name
            ));
        }

        if new_size == 0 {
            return Err(anyhow::anyhow!("New size must be greater than zero"));
        }

        // Use mremap to extend the existing mapping at the same address
        let new_ptr = self.remap_region(new_size)?;

        // Update the struct with new mapping info
        self.mapped_ptr = new_ptr;
        self.mapped_size = new_size;

        Ok(())
    }

    /// Remaps the shared memory region to a new size.
    /// # Safety
    /// The caller must ensure that:
    /// - `new_size` is the desired new size for the mapping.
    pub fn remap_region(&self, new_size: usize) -> Result<*mut c_void> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            let flags = libc::MREMAP_MAYMOVE;
            let new_ptr =
                unsafe { libc::mremap(self.mapped_ptr, self.mapped_size, new_size, flags) };
            if new_ptr == MAP_FAILED {
                Err(anyhow::anyhow!(
                    "Failed to remap shared memory '{}' from size {} to {}: {}",
                    self.shmem_name,
                    self.mapped_size,
                    new_size,
                    io::Error::last_os_error()
                ))
            } else {
                Ok(new_ptr)
            }
        }

        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        {
            // On macOS / other systems without mremap:
            // just unmap and remap a fresh region — no data copy.
            if unsafe { munmap(self.mapped_ptr, self.mapped_size) } != 0 {
                return Err(anyhow::anyhow!(
                    "munmap failed for '{}': {}",
                    self.shmem_name,
                    io::Error::last_os_error()
                ));
            }

            let new_ptr =
                unsafe { mmap(ptr::null_mut(), new_size, PROT_READ, MAP_SHARED, self._fd, 0) };

            if new_ptr == MAP_FAILED {
                Err(anyhow::anyhow!(
                    "mmap failed for '{}': {}",
                    self.shmem_name,
                    io::Error::last_os_error()
                ))
            } else {
                Ok(new_ptr)
            }
        }
    }

    pub fn check_size_changed<T>(&mut self, current_read_ptr: &mut *const T) -> Result<bool> {
        let read_mapped_size = self.map_header().allocated_size();

        if read_mapped_size == self.mapped_size as u64 {
            return Ok(false);
        }

        debug!("Remapping shared memory {} to new size: {}", self.shmem_name, read_mapped_size);

        let offset = (*current_read_ptr as usize).wrapping_sub(self.mapped_ptr as usize);

        self.remap(read_mapped_size as usize)?;

        *current_read_ptr = unsafe { self.mapped_ptr.add(offset) as *const T };

        fence(Ordering::Acquire);

        Ok(true)
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

    pub fn mapped_ptr(&self) -> *mut c_void {
        self.mapped_ptr
    }

    pub fn data_ptr(&self) -> *mut c_void {
        // Skip the header size to get the data pointer
        unsafe { self.mapped_ptr.add(size_of::<H>()) }
    }

    pub fn shmem_input_name(port: u16, asm_service: AsmService, local_rank: i32) -> String {
        format!("{}_{}_input", AsmServices::shmem_prefix(port, local_rank), asm_service.as_str())
    }

    pub fn shmem_output_name(port: u16, asm_service: AsmService, local_rank: i32) -> String {
        format!("{}_{}_output", AsmServices::shmem_prefix(port, local_rank), asm_service.as_str())
    }

    pub fn shmem_chunk_done_name(port: u16, asm_service: AsmService, local_rank: i32) -> String {
        format!(
            "/{}_{}_chunk_done",
            AsmServices::shmem_prefix(port, local_rank),
            asm_service.as_str()
        )
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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
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

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub fn map(_: i32, _: usize, _: i32, _: bool, _: &str) -> *mut c_void {
    ptr::null_mut()
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

pub fn write_input(inputs_path: &Path, shmem_input_name: &str, unlock_mapped_memory: bool) {
    let inputs = fs::read(inputs_path).expect("Failed to read input file");
    let asm_input = AsmInputC2 { zero: 0, input_data_size: inputs.len() as u64 };
    let shmem_input_size = (inputs.len() + size_of::<AsmInputC2>() + 7) & !7;

    let mut full_input = Vec::with_capacity(shmem_input_size);
    full_input.extend_from_slice(&asm_input.to_bytes());
    full_input.extend_from_slice(&inputs);
    while full_input.len() < shmem_input_size {
        full_input.push(0);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let fd = open_shmem(shmem_input_name, libc::O_RDWR, S_IRUSR | S_IWUSR);
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    let fd = open_shmem(shmem_input_name, libc::O_RDWR, S_IRUSR as u32 | S_IWUSR as u32);

    let ptr =
        map(fd, shmem_input_size, PROT_READ | PROT_WRITE, unlock_mapped_memory, "RH input mmap");
    unsafe {
        ptr::copy_nonoverlapping(full_input.as_ptr(), ptr as *mut u8, shmem_input_size);
        unmap(ptr, shmem_input_size);
        close(fd);
    }
}
