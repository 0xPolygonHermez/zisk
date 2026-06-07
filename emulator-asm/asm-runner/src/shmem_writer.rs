#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use libc::{msync, MS_SYNC};
use std::io::{self, Result};
use std::ptr;

use libc::{close, munmap, PROT_READ, PROT_WRITE};

use crate::shmem_sys;

pub(crate) struct ShmemWriter {
    ptr: *mut u8,
    current_ptr: *mut u8,
    size: usize,
    fd: i32,
    name: String,
}

// SAFETY: the only non-auto fields are `ptr`/`current_ptr`, raw pointers into an
// mmap'd shared-memory region whose address is fixed for the handle's lifetime.
// Moving or sharing the handle across threads is sound; data-race freedom on the
// mapped bytes is the caller's responsibility (as for any shared memory) — the
// higher-level wrappers serialize writes where needed.
unsafe impl Send for ShmemWriter {}
unsafe impl Sync for ShmemWriter {}

impl ShmemWriter {
    pub fn new(name: &str, size: usize, unlock_mapped_memory: bool) -> Result<Self> {
        // Open existing shared memory (read/write)
        let fd = shmem_sys::open(name, libc::O_RDWR)?;

        // Map the memory region for read/write
        let ptr = shmem_sys::map(fd, size, PROT_READ | PROT_WRITE, !unlock_mapped_memory, name)?;
        let ptr_u8 = ptr as *mut u8;

        Ok(Self { ptr: ptr_u8, current_ptr: ptr_u8, size, fd, name: name.to_string() })
    }

    unsafe fn unmap(&mut self) {
        if munmap(self.ptr as *mut _, self.size) != 0 {
            tracing::error!("munmap failed: {:?}", io::Error::last_os_error());
        } else {
            self.ptr = ptr::null_mut();
            self.size = 0;
            tracing::trace!("Unmapped shared memory '{}'", self.name);
        }
    }

    /// Writes data to the shared memory, starting at the specified offset
    ///
    /// # Type Parameters
    /// * `T` - The element type of the slice (e.g., u8, u64)
    ///
    /// # Arguments
    /// * `offset` - Byte offset from the start of shared memory where data should be written
    /// * `data` - A slice of data to write to shared memory
    ///
    /// # Returns
    /// * `Ok(())` - If data was successfully written
    /// * `Err` - If data size exceeds shared memory capacity or msync fails
    pub fn write_at<T>(&self, offset: usize, data: &[T]) -> Result<()> {
        let byte_size = std::mem::size_of_val(data);

        if byte_size > self.size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Data size ({} bytes) exceeds shared memory capacity ({}) for '{}'",
                    byte_size, self.size, self.name
                ),
            ));
        }

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr() as *const u8, self.ptr.add(offset), byte_size);
            // Force changes to be flushed to the shared memory
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            if msync(self.ptr as *mut _, self.size, MS_SYNC /*| MS_INVALIDATE*/) != 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(())
    }

    /// Writes data to the shared memory, always from the start
    ///
    /// # Type Parameters
    /// * `T` - The element type of the slice (e.g., u8, u64)
    ///
    /// # Arguments
    /// * `data` - A slice of data to write to shared memory
    ///
    /// # Returns
    /// * `Ok(())` - If data was successfully written
    /// * `Err` - If data size exceeds shared memory capacity or msync fails
    pub fn append_input<T>(&mut self, data: &[T]) -> Result<()> {
        let byte_size = std::mem::size_of_val(data);

        if byte_size > self.size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Data size ({} bytes) exceeds shared memory capacity ({}) for '{}'",
                    byte_size, self.size, self.name
                ),
            ));
        }

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr() as *const u8, self.current_ptr, byte_size);
            // Force changes to be flushed to the shared memory
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            if msync(self.ptr as *mut _, self.size, MS_SYNC) != 0 {
                return Err(io::Error::last_os_error());
            }

            self.current_ptr = self.current_ptr.add(byte_size);
        }

        Ok(())
    }

    /// Writes data to the shared memory as a ring buffer, handling wraparound automatically
    ///
    /// Uses internal pointer tracking with automatic wraparound.
    ///
    /// # Type Parameters
    /// * `T` - The element type of the slice (e.g., u8, u64)
    ///
    /// # Arguments
    /// * `data` - A slice of data to write to shared memory
    #[inline]
    pub fn write_ring_buffer<T>(&mut self, data: &[T]) -> Result<()> {
        let byte_size = std::mem::size_of_val(data);

        let data_ptr = data.as_ptr() as *const u8;

        unsafe {
            let current_offset = self.current_ptr.offset_from(self.ptr) as usize;

            // Check if data wraps around the buffer
            if current_offset + byte_size > self.size {
                // Split write: first part to end of buffer, second part from start
                let first_part_size = self.size - current_offset;
                let second_part_size = byte_size - first_part_size;

                // Write first part to end of buffer
                ptr::copy_nonoverlapping(data_ptr, self.current_ptr, first_part_size);

                // Write second part to start of buffer
                ptr::copy_nonoverlapping(data_ptr.add(first_part_size), self.ptr, second_part_size);

                // Update current_ptr to point after the second part
                self.current_ptr = self.ptr.add(second_part_size);
            } else {
                // Write contiguously
                ptr::copy_nonoverlapping(data_ptr, self.current_ptr, byte_size);

                // Update current_ptr, wrapping if at end
                self.current_ptr = self.current_ptr.add(byte_size);
                let new_offset = self.current_ptr.offset_from(self.ptr) as usize;
                if new_offset == self.size {
                    self.current_ptr = self.ptr;
                }
            }

            // Force changes to be flushed to the shared memory
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            if msync(self.ptr as *mut _, self.size, MS_SYNC) != 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(())
    }

    /// Reads a u64 from shared memory at a specific offset (in bytes)
    ///
    /// # Arguments
    /// * `offset` - Byte offset from the start of shared memory (must be 8-byte aligned)
    ///
    /// # Safety
    /// This method assumes that:
    /// - The shared memory contains at least `offset + 8` bytes of valid data
    /// - The offset should be aligned to 8 bytes
    ///
    /// # Returns
    /// * The u64 value read from the specified offset (in native endianness)
    #[inline]
    pub fn read_u64_at(&self, offset: usize) -> u64 {
        debug_assert_eq!(offset % 8, 0, "Offset must be 8-byte aligned");

        unsafe { (self.ptr.add(offset) as *const u64).read() }
    }

    /// Writes a u64 to shared memory at a specific offset (in bytes)
    ///
    /// # Arguments
    /// * `offset` - Byte offset from the start of shared memory (must be 8-byte aligned)
    /// * `value` - The u64 value to write
    ///
    /// # Safety
    /// This method assumes that:
    /// - The shared memory contains at least `offset + 8` bytes of valid data
    /// - The offset is 8-byte aligned for optimal performance
    ///
    /// # Returns
    /// * `Ok(())` - If the value was written and flushed
    /// * `Err` - If the `msync` flushing the write fails (Linux only)
    #[inline]
    pub fn write_u64_at(&self, offset: usize, value: u64) -> Result<()> {
        debug_assert_eq!(offset % 8, 0, "Offset must be 8-byte aligned");

        unsafe {
            (self.ptr.add(offset) as *mut u64).write(value);

            // Force changes to be flushed to the shared memory
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            if msync(self.ptr as *mut _, self.size, MS_SYNC) != 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(())
    }

    pub fn reset(&mut self) {
        self.current_ptr = self.ptr;
    }
}

impl Drop for ShmemWriter {
    fn drop(&mut self) {
        unsafe {
            self.unmap();
            close(self.fd);
        }
    }
}
