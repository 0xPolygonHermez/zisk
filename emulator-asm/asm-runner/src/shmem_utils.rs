#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use libc::shm_unlink;
use libc::{close, munmap, PROT_READ};
use std::{fmt::Debug, io, os::raw::c_void, ptr};

use anyhow::Result;

use crate::shmem_sys;

pub(crate) struct AsmShmem<H: AsmShmemHeader> {
    _fd: i32,
    mapped_ptr: *mut c_void,
    mapped_size: usize,
    shmem_name: String,
    _phantom: std::marker::PhantomData<H>,
}

// SAFETY: the only non-auto field is `mapped_ptr`, a raw pointer into an mmap'd
// shared-memory region. The mapping address is stable for the handle's lifetime
// (set once in `open_and_map`, only torn down by `unmap`/`Drop`), so sending or
// sharing the handle across threads is sound. This does not by itself provide
// any data-race guarantees for the mapped bytes.
unsafe impl<H: AsmShmemHeader> Send for AsmShmem<H> {}
unsafe impl<H: AsmShmemHeader> Sync for AsmShmem<H> {}

pub(crate) trait AsmShmemHeader: Debug {
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
        let size_header = std::mem::size_of::<H>();
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

    pub fn data_ptr(&self) -> *mut c_void {
        // Skip the header size to get the data pointer
        unsafe { self.mapped_ptr.add(std::mem::size_of::<H>()) }
    }

    pub fn mapped_size(&self) -> usize {
        self.mapped_size
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ShmemWriter;
    use std::ffi::CString;

    /// A minimal header for exercising `AsmShmem`: `allocated_size` lives at offset 0.
    #[repr(C)]
    #[derive(Debug)]
    struct TestHeader {
        allocated_size: u64,
        _other: u64,
    }
    impl AsmShmemHeader for TestHeader {
        fn allocated_size(&self) -> u64 {
            self.allocated_size
        }
    }

    fn seg_name(tag: &str) -> String {
        format!("ZISK_unittest_shmem_{}_{tag}", std::process::id())
    }
    fn create_segment(name: &str, size: usize) {
        let c = CString::new(name).unwrap();
        unsafe {
            libc::shm_unlink(c.as_ptr());
            let fd = libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
            assert!(fd >= 0);
            assert_eq!(libc::ftruncate(fd, size as libc::off_t), 0);
            libc::close(fd);
        }
    }
    fn unlink_segment(name: &str) {
        let c = CString::new(name).unwrap();
        unsafe { libc::shm_unlink(c.as_ptr()) };
    }

    #[test]
    fn open_and_map_reads_header_and_maps_allocated_size() {
        let name = seg_name("hdr");
        create_segment(&name, 4096);
        // Populate the header's allocated_size field (offset 0).
        {
            let w = ShmemWriter::new(&name, 4096, true).unwrap();
            w.write_u64_at(0, 4096).unwrap();
        }
        let shm = AsmShmem::<TestHeader>::open_and_map(&name, true).unwrap();
        assert_eq!(shm.map_header().allocated_size(), 4096);
        assert_eq!(shm.mapped_size(), 4096);
        assert!(shm.is_mapped());
        // `AsmShmem`'s Drop shm_unlinks the segment, so no manual cleanup here.
    }

    #[test]
    fn open_and_map_rejects_zero_allocated_size() {
        let name = seg_name("zero");
        create_segment(&name, 4096); // ftruncate zero-fills → allocated_size == 0
        assert!(AsmShmem::<TestHeader>::open_and_map(&name, true).is_err());
        unlink_segment(&name); // open failed → AsmShmem didn't take ownership
    }

    #[test]
    fn open_and_map_rejects_empty_name() {
        assert!(AsmShmem::<TestHeader>::open_and_map("", true).is_err());
    }

    #[test]
    fn open_and_map_rejects_missing_segment() {
        let name = seg_name("absent");
        unlink_segment(&name);
        assert!(AsmShmem::<TestHeader>::open_and_map(&name, true).is_err());
    }
}
