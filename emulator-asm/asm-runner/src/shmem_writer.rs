#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use libc::{mmap, msync, shm_open, MAP_FAILED, MAP_SHARED, MS_SYNC};
use std::io::{self, Result};
use std::ptr;

use libc::{c_void, close, munmap, PROT_READ, PROT_WRITE, S_IRUSR, S_IWUSR};

pub struct SharedMemoryWriter {
    ptr: *mut u8,
    current_ptr: *mut u8,
    size: usize,
    fd: i32,
    name: String,
}

unsafe impl Send for SharedMemoryWriter {}
unsafe impl Sync for SharedMemoryWriter {}

impl SharedMemoryWriter {
    pub fn new(name: &str, size: usize, unlock_mapped_memory: bool) -> Result<Self> {
        // Open existing shared memory (read/write)
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let fd = Self::open_shmem(name, libc::O_RDWR, S_IRUSR | S_IWUSR);

        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        let fd = Self::open_shmem(name, libc::O_RDWR, S_IRUSR as u32 | S_IWUSR as u32);

        // Map the memory region for read/write
        let ptr = Self::map(fd, size, PROT_READ | PROT_WRITE, unlock_mapped_memory, name);
        let ptr_u8 = ptr as *mut u8;

        Ok(Self { ptr: ptr_u8, current_ptr: ptr_u8, size, fd, name: name.to_string() })
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
    pub fn map(_: i32, _: usize, _: i32, _: bool, _: &str) -> *mut c_void {
        ptr::null_mut()
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
    pub fn write_input<T>(&self, data: &[T]) -> Result<()> {
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
            ptr::copy_nonoverlapping(data.as_ptr() as *const u8, self.ptr, byte_size);
            // Force changes to be flushed to the shared memory
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            if msync(self.ptr as *mut _, self.size, MS_SYNC /*| MS_INVALIDATE*/) != 0 {
                return Err(io::Error::last_os_error());
            }
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
    pub fn write_ring_buffer<T>(&mut self, data: &[T]) {
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

            // // Force changes to be flushed to the shared memory
            // #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            // if msync(self.ptr as *mut _, self.size, MS_SYNC) != 0 {
            //     return Err(io::Error::last_os_error());
            // }
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
    #[inline]
    pub fn write_u64_at(&self, offset: usize, value: u64) {
        debug_assert_eq!(offset % 8, 0, "Offset must be 8-byte aligned");

        unsafe {
            (self.ptr.add(offset) as *mut u64).write(value);
        }
    }
}

impl Drop for SharedMemoryWriter {
    fn drop(&mut self) {
        unsafe {
            self.unmap();
            close(self.fd);
        }
    }
}
