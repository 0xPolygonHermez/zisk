#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use libc::{mmap, msync, shm_open, MAP_FAILED, MAP_SHARED, MS_SYNC};
use std::io::{self, Result};
use std::ptr;

use libc::{c_void, close, munmap, PROT_READ, PROT_WRITE, S_IRUSR, S_IWUSR};

pub struct SharedMemoryWriter {
    ptr: *mut u8,
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

        Ok(Self { ptr: ptr as *mut u8, size, fd, name: name.to_string() })
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
    pub fn write_input(&self, data: &[u8]) -> Result<()> {
        if data.len() > self.size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Data size ({}) exceeds shared memory capacity ({}) for '{}'",
                    data.len(),
                    self.size,
                    self.name
                ),
            ));
        }

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), self.ptr, data.len());
            // Force changes to be flushed to the shared memory
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            if msync(self.ptr as *mut _, self.size, MS_SYNC /*| MS_INVALIDATE*/) != 0 {
                panic!("msync failed: {}", std::io::Error::last_os_error());
            }
        }

        Ok(())
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
