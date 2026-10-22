#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use libc::{msync, MS_SYNC};
use std::io::{self, Result};
use std::ptr;

use libc::{close, munmap, PROT_READ, PROT_WRITE};
use rayon::prelude::*;

use crate::shmem_sys;

/// Below this many bytes one thread zeroes as fast as rayon's, or faster: on a 2-socket host,
/// spreading the work costs more than the `memset`. Above it, zeroing is bound by memory
/// bandwidth, which rayon's threads reach 2–5× sooner.
const PARALLEL_ZERO_BYTES: usize = 64 << 20;

/// The piece of a large range that each rayon task zeroes: a transparent huge page, as proofman
/// zeroes its pool buffers.
const ZERO_CHUNK_BYTES: usize = 2 << 20;

/// Zeroes `bytes`, from every rayon thread when they are many.
fn zero_bytes(bytes: &mut [u8]) {
    if bytes.len() < PARALLEL_ZERO_BYTES {
        bytes.fill(0);
    } else {
        bytes.par_chunks_mut(ZERO_CHUNK_BYTES).for_each(|chunk| chunk.fill(0));
    }
}

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
    /// Opens and maps a shared memory region for read/write access.
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

    /// Resets the internal pointer used for appending to the start of the buffer.
    pub fn reset(&mut self) {
        self.current_ptr = self.ptr;
    }

    /// The offset, in bytes, that `append_input` writes at next.
    pub fn offset(&self) -> usize {
        // SAFETY: `current_ptr` always points into the mapping that starts at `ptr`.
        unsafe { self.current_ptr.offset_from(self.ptr) as usize }
    }

    /// Zeroes the bytes `[start, end)` of the shared memory.
    pub fn zero_range(&mut self, start: usize, end: usize) -> Result<()> {
        if end <= start {
            return Ok(());
        }
        if end > self.size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Range end ({}) exceeds shared memory capacity ({}) for '{}'",
                    end, self.size, self.name
                ),
            ));
        }

        unsafe {
            // SAFETY: `[start, end)` lies inside the mapping, and `&mut self` keeps this writer
            // from writing it meanwhile.
            zero_bytes(std::slice::from_raw_parts_mut(self.ptr.add(start), end - start));
            // Force changes to be flushed to the shared memory
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            if msync(self.ptr as *mut _, self.size, MS_SYNC) != 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(())
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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ShmemReader;
    use std::ffi::CString;

    /// Unique per-test segment name (cargo runs tests as threads in one process).
    fn seg_name(tag: &str) -> String {
        format!("ZISK_unittest_{}_{tag}", std::process::id())
    }

    /// Create a fresh, zero-filled `/dev/shm` segment of `size` bytes.
    fn create_segment(name: &str, size: usize) {
        let c = CString::new(name).unwrap();
        unsafe {
            libc::shm_unlink(c.as_ptr()); // drop any stale leftover
            let fd = libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
            assert!(fd >= 0, "shm_open(create) failed for {name}");
            assert_eq!(libc::ftruncate(fd, size as libc::off_t), 0, "ftruncate failed");
            libc::close(fd);
        }
    }

    fn unlink_segment(name: &str) {
        let c = CString::new(name).unwrap();
        unsafe { libc::shm_unlink(c.as_ptr()) };
    }

    #[test]
    fn writer_reader_u64_round_trip() {
        let name = seg_name("u64rt");
        create_segment(&name, 4096);
        {
            let w = ShmemWriter::new(&name, 4096, true).unwrap();
            w.write_u64_at(0, 0xDEAD_BEEF).unwrap();
            w.write_u64_at(8, 42).unwrap();
        }
        let r = ShmemReader::new(&name, 4096).unwrap();
        assert_eq!(r.read_u64_at(0), 0xDEAD_BEEF);
        assert_eq!(r.read_u64_at(8), 42);
        unlink_segment(&name);
    }

    #[test]
    fn write_at_offset_is_visible_to_reader() {
        let name = seg_name("writeat");
        create_segment(&name, 4096);
        let w = ShmemWriter::new(&name, 4096, true).unwrap();
        w.write_at(16, &[1u64, 2, 3]).unwrap();
        let r = ShmemReader::new(&name, 4096).unwrap();
        assert_eq!([r.read_u64_at(16), r.read_u64_at(24), r.read_u64_at(32)], [1, 2, 3]);
        unlink_segment(&name);
    }

    #[test]
    fn write_at_rejects_payload_larger_than_segment() {
        let name = seg_name("cap");
        create_segment(&name, 64);
        let w = ShmemWriter::new(&name, 64, true).unwrap();
        assert!(w.write_at(0, &[0u8; 128]).is_err());
        unlink_segment(&name);
    }

    #[test]
    fn ring_buffer_wraps_around_the_end() {
        let name = seg_name("ring");
        create_segment(&name, 32); // 4 u64 slots
        let mut w = ShmemWriter::new(&name, 32, true).unwrap();
        w.write_ring_buffer(&[1u64, 2, 3]).unwrap(); // slots 0,8,16; cursor at 24
        w.write_ring_buffer(&[4u64, 5]).unwrap(); // 4 at 24, wraps, 5 at 0
        let r = ShmemReader::new(&name, 32).unwrap();
        assert_eq!(r.read_u64_at(24), 4, "last slot before wrap");
        assert_eq!(r.read_u64_at(0), 5, "wrapped to start");
        assert_eq!(r.read_u64_at(8), 2, "untouched from first write");
        unlink_segment(&name);
    }

    #[test]
    fn zero_bytes_clears_ranges_below_and_above_the_parallel_threshold() {
        for len in [1, 4097, PARALLEL_ZERO_BYTES + ZERO_CHUNK_BYTES / 2 + 3] {
            let mut bytes = vec![0xABu8; len];
            zero_bytes(&mut bytes);
            assert!(bytes.iter().all(|&b| b == 0), "{len} bytes");
        }
    }

    #[test]
    fn zero_range_clears_only_its_range() {
        let name = seg_name("zero");
        create_segment(&name, 4096);
        let mut w = ShmemWriter::new(&name, 4096, true).unwrap();
        w.write_at(0, &[1u64, 2, 3, 4]).unwrap();
        w.zero_range(8, 24).unwrap();
        let r = ShmemReader::new(&name, 4096).unwrap();
        assert_eq!(
            [r.read_u64_at(0), r.read_u64_at(8), r.read_u64_at(16), r.read_u64_at(24)],
            [1, 0, 0, 4]
        );
        assert!(w.zero_range(0, 4097).is_err(), "a range past the segment");
        unlink_segment(&name);
    }

    #[test]
    fn new_fails_for_missing_segment() {
        let name = seg_name("missing");
        unlink_segment(&name); // ensure it does not exist
        assert!(ShmemWriter::new(&name, 4096, true).is_err());
    }
}
