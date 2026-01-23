#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use libc::{mmap, shm_open, MAP_FAILED, MAP_SHARED};
use std::io::{self, Result};
use std::ptr;
use std::sync::atomic::{compiler_fence, Ordering};

use libc::{c_void, close, munmap, PROT_READ, S_IRUSR};

pub struct SharedMemoryReader {
    ptr: *const u8,
    size: usize,
    fd: i32,
    name: String,
}

unsafe impl Send for SharedMemoryReader {}
unsafe impl Sync for SharedMemoryReader {}

impl SharedMemoryReader {
    pub fn new(name: &str, size: usize) -> Result<Self> {
        // Open existing shared memory (read-only)
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let fd = Self::open_shmem(name, libc::O_RDONLY, S_IRUSR);

        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        let fd = Self::open_shmem(name, libc::O_RDONLY, S_IRUSR as u32);

        // Map the memory region for read-only
        let ptr = Self::map(fd, size, PROT_READ, false, name);
        let ptr_u8 = ptr as *const u8;

        Ok(Self { ptr: ptr_u8, size, fd, name: name.to_string() })
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn open_shmem(name: &str, flags: i32, mode: u32) -> i32 {
        let c_name = std::ffi::CString::new(name).expect("CString::new failed");
        let fd = unsafe { shm_open(c_name.as_ptr(), flags, mode) };
        if fd == -1 {
            let errno_value = unsafe { *libc::__errno_location() };
            let err = io::Error::from_raw_os_error(errno_value);
            let err2 = io::Error::last_os_error();
            panic!("shm_open('{name}') failed: libc::errno:{err} #### last_os_error:{err2}");
        }
        fd
    }

    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    fn open_shmem(_name: &str, _flags: i32, _mode: u32) -> i32 {
        0
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn map(fd: i32, size: usize, prot: i32, unlock_mapped_memory: bool, desc: &str) -> *mut c_void {
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
    fn map(_: i32, _: usize, _: i32, _: bool, _: &str) -> *mut c_void {
        ptr::null_mut()
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

    /// Reads a slice of data from shared memory at a specific offset
    ///
    /// # Type Parameters
    /// * `T` - The element type to read
    ///
    /// # Arguments
    /// * `offset` - Byte offset from the start of shared memory
    /// * `len` - Number of elements of type T to read
    ///
    /// # Returns
    /// * `Ok(Vec<T>)` - A vector containing the read data
    /// * `Err` - If the read would exceed shared memory bounds
    pub fn read_slice<T: Copy>(&self, offset: usize, len: usize) -> Result<Vec<T>> {
        let byte_size = len * std::mem::size_of::<T>();

        if offset + byte_size > self.size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Read of {} bytes at offset {} exceeds shared memory capacity ({}) for '{}'",
                    byte_size, offset, self.size, self.name
                ),
            ));
        }

        compiler_fence(Ordering::Acquire);

        let mut result = Vec::with_capacity(len);
        unsafe {
            ptr::copy_nonoverlapping(self.ptr.add(offset) as *const T, result.as_mut_ptr(), len);
            result.set_len(len);
        }

        Ok(result)
    }

    /// Returns the size of the shared memory region in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the name of the shared memory region
    pub fn name(&self) -> &str {
        &self.name
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
