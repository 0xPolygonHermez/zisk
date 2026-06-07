#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use libc::shm_unlink;
use libc::{close, munmap, MAP_FAILED, PROT_READ};
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
use libc::{mmap, MAP_SHARED};
use proofman_common::format_bytes;
use std::{
    fmt::Debug,
    io,
    os::raw::c_void,
    ptr,
    sync::atomic::{fence, Ordering},
};
use tracing::info;

use anyhow::Result;

use crate::shmem_sys;

pub struct AsmShmem<H: AsmShmemHeader> {
    _fd: i32,
    mapped_ptr: *mut c_void,
    mapped_size: usize,
    shmem_name: String,
    _phantom: std::marker::PhantomData<H>,
}

// SAFETY: the only non-auto field is `mapped_ptr`, a raw pointer into an mmap'd
// shared-memory region. The mapping is stable for the handle's lifetime (`remap`
// updates the pointer under `&mut self`), so sending/sharing the handle across
// threads is sound.
unsafe impl<H: AsmShmemHeader> Send for AsmShmem<H> {}
unsafe impl<H: AsmShmemHeader> Sync for AsmShmem<H> {}

pub trait AsmShmemHeader: Debug {
    fn allocated_size(&self) -> u64;
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl<H: AsmShmemHeader> Drop for AsmShmem<H> {
    fn drop(&mut self) {
        if let Ok(c_name) = std::ffi::CString::new(self.shmem_name.clone()) {
            unsafe { shm_unlink(c_name.as_ptr()) };
        }
        self.unmap().unwrap_or_else(|err| {
            tracing::error!("Failed to unmap shared memory '{}': {}", self.shmem_name, err)
        });
        unsafe { close(self._fd) };
    }
}

impl<H: AsmShmemHeader> AsmShmem<H> {
    pub fn open_and_map(name: &str, _unlock_mapped_memory: bool) -> Result<Self> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Shared memory name {name} cannot be empty"));
        }

        let fd = shmem_sys::open(name, libc::O_RDONLY)?;

        // `MAP_LOCKED` only exists on Linux; elsewhere the lock flag is ignored.
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let lock = !_unlock_mapped_memory;
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        let lock = false;

        // First map only the header to learn the producer's allocated size.
        let size_header = size_of::<H>();
        let header_ptr = match shmem_sys::map(fd, size_header, PROT_READ, lock, name) {
            Ok(ptr) => ptr,
            Err(e) => {
                unsafe { close(fd) };
                return Err(e.into());
            }
        };

        let allocated_size = unsafe { (header_ptr as *const H).read().allocated_size() as usize };

        // Done with the header-only mapping; release it before the full map.
        unsafe {
            if munmap(header_ptr, size_header) != 0 {
                let err = io::Error::last_os_error();
                close(fd);
                return Err(anyhow::anyhow!("munmap failed for '{name}': {err}"));
            }
        }

        if allocated_size == 0 {
            unsafe { close(fd) };
            return Err(anyhow::anyhow!("Shared memory '{}' has zero allocated size", name));
        }

        // Remap the full region now that the real size is known.
        let mapped_ptr = match shmem_sys::map(fd, allocated_size, PROT_READ, lock, name) {
            Ok(ptr) => ptr,
            Err(e) => {
                unsafe { close(fd) };
                return Err(e.into());
            }
        };

        Ok(Self {
            _fd: fd,
            mapped_ptr,
            mapped_size: allocated_size,
            shmem_name: name.to_string(),
            _phantom: std::marker::PhantomData::<H>,
        })
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

        info!(
            "Remapping shared memory {}: {} => {}",
            self.shmem_name,
            format_bytes(self.mapped_size as f64),
            format_bytes(read_mapped_size as f64)
        );

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

    /// Read and return a copy of the header from the start of the mapping.
    ///
    /// # Panics
    /// Panics if the region is not currently mapped. This is an invariant guard,
    /// not an I/O path: an unmapped handle has a null `mapped_ptr`, so reading the
    /// header would dereference null (UB). On a live handle the mapping is set up
    /// in the constructor and only torn down by `unmap`/`Drop`, so this never
    /// fires in correct use.
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

    pub fn mapped_size(&self) -> usize {
        self.mapped_size
    }
}
