use crate::AsmShmemHeader;
use libc::{
    c_uint, close, mmap, munmap, shm_open, shm_unlink, MAP_FAILED, MAP_SHARED, PROT_READ, S_IRUSR,
    S_IWUSR,
};
use std::{
    ffi::CString,
    io,
    os::raw::c_void,
    ptr,
    sync::atomic::{fence, Ordering},
};
use tracing::debug;

use anyhow::anyhow;
use anyhow::Result;

/// Represents a single mapped shared memory file within the multi-file structure.
struct MappedFile {
    fd: i32,
    #[allow(dead_code)] // May be useful for debugging/validation
    size: usize,
}

/// A shared memory manager that supports multiple contiguous shared memory files.
///
/// This struct reserves a large virtual address range upfront and maps multiple
/// shared memory files (`_0`, `_1`, etc.) into contiguous portions of that range.
///
/// File layout:
/// - `{base_name}_0`: Initial file with size `initial_size`, contains the header
/// - `{base_name}_1`, `_2`, ...: Incremental files with size `incremental_size`
pub struct AsmMultiSharedMemory<H: AsmShmemHeader> {
    base_name: String,
    reserved_ptr: *mut c_void,
    reserved_size: usize,
    initial_size: usize,
    incremental_size: usize,
    mapped_files: Vec<MappedFile>,
    total_mapped_size: usize,
    unlock_mapped_memory: bool,
    _phantom: std::marker::PhantomData<H>,
}

unsafe impl<H: AsmShmemHeader> Send for AsmMultiSharedMemory<H> {}
unsafe impl<H: AsmShmemHeader> Sync for AsmMultiSharedMemory<H> {}

impl<H: AsmShmemHeader> Drop for AsmMultiSharedMemory<H> {
    fn drop(&mut self) {
        // Close all file descriptors
        for mapped_file in &self.mapped_files {
            unsafe { close(mapped_file.fd) };
        }

        // Unmap the entire reserved region (this handles all the MAP_FIXED mappings too)
        if !self.reserved_ptr.is_null() && self.reserved_size > 0 {
            unsafe {
                if munmap(self.reserved_ptr, self.reserved_size) != 0 {
                    tracing::error!(
                        "munmap failed for multi-shmem '{}': {:?}",
                        self.base_name,
                        io::Error::last_os_error()
                    );
                }
            }
        }
    }
}

impl<H: AsmShmemHeader> AsmMultiSharedMemory<H> {
    /// Opens and maps the initial shared memory file, reserving address space for growth.
    ///
    /// # Arguments
    /// * `base_name` - Base name for shared memory files (files will be `{base_name}_0`, `_1`, etc.)
    /// * `initial_size` - Size of the first file (`_0`)
    /// * `incremental_size` - Size of subsequent files (`_1`, `_2`, ...)
    /// * `max_size` - Total virtual address space to reserve
    /// * `unlock_mapped_memory` - If true, don't use MAP_LOCKED
    pub fn open_and_map(
        base_name: &str,
        initial_size: usize,
        incremental_size: usize,
        max_size: usize,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        if base_name.is_empty() {
            return Err(anyhow!("Shared memory base name cannot be empty"));
        }

        if max_size < initial_size {
            return Err(anyhow!(
                "max_size ({}) must be >= initial_size ({})",
                max_size,
                initial_size
            ));
        }

        if incremental_size == 0 {
            return Err(anyhow!("incremental_size must be > 0"));
        }

        // Reserve the entire address range with an anonymous mapping
        // MAP_NORESERVE prevents reserving swap space for the entire range
        let reserved_ptr = unsafe {
            mmap(
                ptr::null_mut(),
                max_size,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };

        if reserved_ptr == MAP_FAILED {
            let err = io::Error::last_os_error();
            return Err(anyhow!(
                "Failed to reserve {} bytes of address space for '{}': {}",
                max_size,
                base_name,
                err
            ));
        }

        debug!("Reserved {} bytes at {:p} for multi-shmem '{}'", max_size, reserved_ptr, base_name);

        let mut this = Self {
            base_name: base_name.to_string(),
            reserved_ptr,
            reserved_size: max_size,
            initial_size,
            incremental_size,
            mapped_files: Vec::with_capacity(8),
            total_mapped_size: 0,
            unlock_mapped_memory,
            _phantom: std::marker::PhantomData,
        };

        // Map the initial file (_0)
        if let Err(e) = this.map_file(0) {
            unsafe { munmap(reserved_ptr, max_size) };
            return Err(e);
        }

        this.total_mapped_size = initial_size;

        Ok(this)
    }

    /// Checks if the producer has allocated more space and maps any new files.
    ///
    /// This reads `allocated_size` from the header (always in file `_0`) and maps
    /// any new files that have been created by the producer.
    ///
    /// This does NOT move existing mappings, so pointers and slices to already-mapped data remain valid.
    pub fn check_size_changed(&mut self) -> Result<bool> {
        let allocated_size = self.map_header().allocated_size() as usize;

        if allocated_size <= self.total_mapped_size {
            return Ok(false);
        }

        // Calculate how many files should exist
        let files_needed = if allocated_size <= self.initial_size {
            1
        } else {
            1 + (allocated_size - self.initial_size).div_ceil(self.incremental_size)
        };

        let current_files = self.mapped_files.len();

        if files_needed <= current_files {
            // Size increased but within current file - just update total
            self.total_mapped_size = allocated_size;
            return Ok(true);
        }

        debug!(
            "Multi-shmem '{}': allocated_size={}, need {} files, have {}",
            self.base_name, allocated_size, files_needed, current_files
        );

        // Map all new files
        for file_idx in current_files..files_needed {
            self.map_file(file_idx)?;
        }

        self.total_mapped_size = allocated_size;

        fence(Ordering::Acquire);

        Ok(true)
    }

    /// Maps a specific file index into the reserved address space.
    fn map_file(&mut self, file_idx: usize) -> Result<()> {
        let file_name = format!("{}_{}", self.base_name, file_idx);

        unsafe {
            let c_name = CString::new(file_name.clone())
                .map_err(|_| anyhow!("Shared memory name contains null byte"))?;

            let fd =
                shm_open(c_name.as_ptr(), libc::O_RDONLY, S_IRUSR as c_uint | S_IWUSR as c_uint);
            if fd == -1 {
                let err = io::Error::last_os_error();
                return Err(anyhow!("shm_open('{}') failed: {}", file_name, err));
            }

            // Unlink to ensure cleanup
            if shm_unlink(c_name.as_ptr()) != 0 {
                let err = io::Error::last_os_error();
                close(fd);
                return Err(anyhow!("shm_unlink('{}') failed: {}", file_name, err));
            }

            // For _0, validate that the header has a non-zero allocated size
            if file_idx == 0 {
                let temp_map = mmap(ptr::null_mut(), size_of::<H>(), PROT_READ, MAP_SHARED, fd, 0);
                if temp_map == MAP_FAILED {
                    let err = io::Error::last_os_error();
                    close(fd);
                    return Err(anyhow!("mmap failed for header of '{}': {}", file_name, err));
                }

                let header = (temp_map as *const H).read();
                let allocated_size = header.allocated_size();
                munmap(temp_map, size_of::<H>());

                if allocated_size == 0 {
                    close(fd);
                    return Err(anyhow!("Shared memory '{}' has zero allocated size", file_name));
                }
            }

            // Calculate the offset where this file should be mapped
            let offset = if file_idx == 0 {
                0
            } else {
                self.initial_size + (file_idx - 1) * self.incremental_size
            };

            let file_size = if file_idx == 0 { self.initial_size } else { self.incremental_size };

            let target_addr = self.reserved_ptr.add(offset);

            let mut flags = MAP_SHARED | libc::MAP_FIXED;
            if !self.unlock_mapped_memory {
                flags |= libc::MAP_LOCKED;
            }

            let mapped_ptr = mmap(target_addr, file_size, PROT_READ, flags, fd, 0);
            if mapped_ptr == MAP_FAILED {
                let err = io::Error::last_os_error();
                close(fd);
                return Err(anyhow!(
                    "mmap(MAP_FIXED) failed for '{}': {} ({} bytes at {:p})",
                    file_name,
                    err,
                    file_size,
                    target_addr
                ));
            }

            debug!(
                "Mapped '{}' ({} bytes) at {:p} (offset {})",
                file_name, file_size, mapped_ptr, offset
            );

            self.mapped_files.push(MappedFile { fd, size: file_size });
        }

        Ok(())
    }

    /// Reads the header from the shared memory (always from file `_0`).
    pub fn map_header(&self) -> H {
        if self.mapped_files.is_empty() {
            panic!("Multi-shmem '{}' has no mapped files, cannot read header", self.base_name);
        }

        unsafe { (self.reserved_ptr as *const H).read() }
    }

    /// Returns the base pointer of the mapped region.
    pub fn mapped_ptr(&self) -> *mut c_void {
        self.reserved_ptr
    }

    /// Returns a pointer to the data area (after the header).
    pub fn data_ptr(&self) -> *mut c_void {
        unsafe { self.reserved_ptr.add(size_of::<H>()) }
    }

    /// Returns the total currently mapped size.
    pub fn total_mapped_size(&self) -> usize {
        self.total_mapped_size
    }

    /// Returns the number of currently mapped files.
    pub fn num_mapped_files(&self) -> usize {
        self.mapped_files.len()
    }

    /// Releases incremental shared memory files for a new execution.
    ///
    /// This closes file descriptors for incremental files (`_1`, `_2`, ...) while
    /// keeping `_0` mapped. The reserved address space is preserved.
    ///
    /// Call this before starting a new execution when reusing the same instance
    /// in a distributed context where `_0` remains valid across executions.
    pub fn release_incremental(&mut self) {
        let files_to_close = self.mapped_files.len().saturating_sub(1);

        // Close file descriptors for incremental files (_1, _2, ...), keep _0
        while self.mapped_files.len() > 1 {
            let mapped_file = self.mapped_files.pop().unwrap();
            unsafe { close(mapped_file.fd) };
        }

        // Reset state to initial
        self.total_mapped_size = self.initial_size;

        debug!(
            "Reset multi-shmem '{}': kept _0, closed {} incremental files, total_mapped_size={}",
            self.base_name, files_to_close, self.total_mapped_size
        );
    }
}
