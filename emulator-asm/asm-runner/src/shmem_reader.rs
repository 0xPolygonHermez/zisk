use std::io::{self, Result};
use std::ptr;

use libc::{close, munmap, PROT_READ};

use crate::shmem_sys;

pub struct SharedMemoryReader {
    ptr: *const u8,
    size: usize,
    fd: i32,
    name: String,
}

// SAFETY: the only non-auto field is `ptr`, a raw pointer into a read-only
// mmap'd shared-memory region whose address is fixed for the handle's lifetime.
// Sharing the read-only handle across threads is sound.
unsafe impl Send for SharedMemoryReader {}
unsafe impl Sync for SharedMemoryReader {}

impl SharedMemoryReader {
    pub fn new(name: &str, size: usize) -> Result<Self> {
        // Open existing shared memory (read-only)
        let fd = shmem_sys::open(name, libc::O_RDONLY)?;

        // Map the memory region for read-only (always locked)
        let ptr = shmem_sys::map(fd, size, PROT_READ, true, name)?;
        let ptr_u8 = ptr as *const u8;

        Ok(Self { ptr: ptr_u8, size, fd, name: name.to_string() })
    }

    unsafe fn unmap(&mut self) {
        if munmap(self.ptr as *mut _, self.size) != 0 {
            tracing::error!("munmap failed: {:?}", io::Error::last_os_error());
        } else {
            self.ptr = ptr::null();
            self.size = 0;
            tracing::trace!("Unmapped shared memory '{}'", self.name);
        }
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

}

impl Drop for SharedMemoryReader {
    fn drop(&mut self) {
        unsafe {
            self.unmap();
            close(self.fd);
        }
    }
}
