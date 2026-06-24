use std::io::{self, Result};
use std::ptr;

use libc::{close, munmap, PROT_READ};

use crate::shmem_sys;

pub(crate) struct ShmemReader {
    ptr: *const u8,
    size: usize,
    fd: i32,
    name: String,
}

// SAFETY: the only non-auto field is `ptr`, a raw pointer into a read-only
// mmap'd shared-memory region whose address is fixed for the handle's lifetime.
// Sharing the read-only handle across threads is sound.
unsafe impl Send for ShmemReader {}
unsafe impl Sync for ShmemReader {}

impl ShmemReader {
    /// Opens and maps a shared memory region for read-only access (pages locked resident).
    pub fn new(name: &str, size: usize) -> Result<Self> {
        Self::open(name, size, true)
    }

    /// Like [`new`](Self::new) but without `MAP_LOCKED`, so a large sparse segment does not force
    /// the whole region resident. Used for the public-output channel, whose generous cap is only
    /// ever touched up to the actual output length.
    pub fn new_unlocked(name: &str, size: usize) -> Result<Self> {
        Self::open(name, size, false)
    }

    fn open(name: &str, size: usize, lock: bool) -> Result<Self> {
        // Open existing shared memory (read-only)
        let fd = shmem_sys::open(name, libc::O_RDONLY)?;

        // Map the memory region for read-only.
        let ptr = shmem_sys::map(fd, size, PROT_READ, lock, name)?;
        let ptr_u8 = ptr as *const u8;

        Ok(Self { ptr: ptr_u8, size, fd, name: name.to_string() })
    }

    /// Reads `len` bytes starting at `offset` into a fresh `Vec<u8>`.
    ///
    /// # Safety contract
    /// The caller must ensure the segment contains at least `offset + len` valid bytes; callers
    /// derive `len` from a length header the producer wrote before signalling completion.
    pub fn read_bytes(&self, offset: usize, len: usize) -> Vec<u8> {
        debug_assert!(offset + len <= self.size, "read_bytes out of bounds");
        let mut out = vec![0u8; len];
        if len != 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(self.ptr.add(offset), out.as_mut_ptr(), len);
            }
        }
        out
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

impl Drop for ShmemReader {
    fn drop(&mut self) {
        unsafe {
            self.unmap();
            close(self.fd);
        }
    }
}
