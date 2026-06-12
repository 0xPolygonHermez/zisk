use std::sync::{Arc, Mutex};

use named_sem::NamedSemaphore;
use zisk_common::{
    io::{StreamError, StreamProcessor, StreamSink},
    reinterpret_vec,
};
use zisk_core::MAX_INPUT_SIZE;

use crate::{sem_input_avail_name, shmem_input_name, AsmServices, ControlShmem, ShmemWriter};

use anyhow::Result;

/// This struct manages the shared memory for writing inputs to the C++ side.
pub struct InputsShmemWriter {
    writer: Mutex<ShmemWriter>,
    control_writer: Arc<ControlShmem>,
    sem_avails: Mutex<Option<Vec<NamedSemaphore>>>,
}

// SAFETY: needed because `sem_avails` holds `NamedSemaphore`s, which are !Send +
// !Sync (they wrap a raw `*mut sem_t`). POSIX named semaphores are themselves
// thread- and process-safe by spec, and all access here goes through the inner
// `Mutex`es, so sharing the writer across threads is sound.
unsafe impl Send for InputsShmemWriter {}
unsafe impl Sync for InputsShmemWriter {}

impl InputsShmemWriter {
    /// Create writers mapping the per-service input shmem segments.
    /// Semaphores are not opened here — call `bind_semaphores` before first use.
    pub fn new(
        shm_prefix: &str,
        unlock_mapped_memory: bool,
        control_writer: Arc<ControlShmem>,
    ) -> Result<Self> {
        let name = shmem_input_name(shm_prefix);
        let mut writer = ShmemWriter::new(&name, MAX_INPUT_SIZE as usize, unlock_mapped_memory)
            .map_err(anyhow::Error::from)?;
        writer.reset();
        writer.append_input(&0u64.to_le_bytes())?;

        Ok(Self { writer: Mutex::new(writer), control_writer, sem_avails: Mutex::new(None) })
    }

    /// Open the per-service input-availability semaphores for a given program.
    /// Replaces any previously bound semaphores.
    pub fn bind_semaphores(&self, sem_prefix: &str) -> Result<()> {
        let sems = AsmServices::SERVICES
            .iter()
            .map(|service| {
                let name = sem_input_avail_name(sem_prefix, *service);
                NamedSemaphore::create(&name, 0)
                    .map_err(|e| anyhow::anyhow!("Failed to create semaphore '{}': {}", name, e))
            })
            .collect::<Result<Vec<_>>>()?;
        *self.sem_avails.lock().unwrap() = Some(sems);
        Ok(())
    }

    /// Drop the semaphore handles (does not unlink — the binary owns the names).
    pub fn unbind_semaphores(&self) {
        *self.sem_avails.lock().unwrap() = None;
    }

    /// Writes inputs to the shared memory and updates the control shared memory.
    pub fn write_input(&self, inputs: &[u8]) -> Result<()> {
        if inputs.is_empty() {
            return Ok(());
        }

        self.writer.lock().unwrap().write_at(8, inputs)?;
        self.control_writer.inc_inputs_size(inputs.len())?;
        self.notify_all_services()?;
        Ok(())
    }

    /// Appends inputs to the shared memory and updates the control shared memory.
    pub fn append_input(&self, inputs: &[u8]) -> Result<()> {
        self.writer.lock().unwrap().append_input(inputs)?;
        self.control_writer.inc_inputs_size(inputs.len())?;
        self.notify_all_services()?;
        Ok(())
    }

    fn notify_all_services(&self) -> Result<()> {
        if let Some(sems) = self.sem_avails.lock().unwrap().as_mut() {
            for sem in sems.iter_mut() {
                sem.post()?;
            }
        }
        Ok(())
    }

    /// Set the C-side `ResetFlag` and wake all `sem_input_avail` waiters in
    /// the correct order: flag first, then post. A child that wakes from a
    /// post with `flag == 0` goes back to sleep and would never see a later
    /// `set_reset_flag()`, so the two steps must always run together.
    /// Cleared by the next job's `reset()`.
    pub fn signal_reset(&self) -> Result<()> {
        self.control_writer.set_reset_flag()?;
        self.notify_all_services()
    }

    /// Resets the shared memory and control flags, and drains any stale semaphore posts.
    pub fn reset(&self) {
        let mut writer = self.writer.lock().unwrap();
        writer.reset();
        writer
            .append_input(&0u64.to_le_bytes())
            .expect("Failed to write initial header after reset");

        if let Err(e) = self.control_writer.reset() {
            tracing::error!("InputsShmemWriter::reset: control flush failed: {e}");
        }
        if let Some(sems) = self.sem_avails.lock().unwrap().as_mut() {
            for sem in sems.iter_mut() {
                while sem.try_wait().is_ok() {}
            }
        }
    }
}

impl StreamSink for InputsShmemWriter {
    fn submit(&self, hints: &[u64]) -> Result<(), StreamError> {
        let bytes = reinterpret_vec(hints.to_vec()).map_err(StreamError::other)?;
        self.append_input(&bytes).map_err(StreamError::other)
    }

    fn reset(&self) {
        self.reset();
    }
}

impl StreamProcessor for InputsShmemWriter {
    fn process_hints(&self, data: &[u64], _first_batch: bool) -> Result<bool, StreamError> {
        self.submit(data)?;
        Ok(false)
    }

    fn reset(&self) {
        InputsShmemWriter::reset(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{shmem_control_input_name, shmem_input_name, ControlShmem, ShmemReader};
    use std::ffi::CString;
    use zisk_core::MAX_INPUT_SIZE;

    fn create_segment(name: &str, size: usize) {
        let c = CString::new(name).unwrap();
        unsafe {
            libc::shm_unlink(c.as_ptr());
            let fd = libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
            assert!(fd >= 0);
            assert_eq!(libc::ftruncate(fd, size as libc::off_t), 0); // sparse — no real allocation
            libc::close(fd);
        }
    }
    fn unlink_segment(name: &str) {
        let c = CString::new(name).unwrap();
        unsafe { libc::shm_unlink(c.as_ptr()) };
    }

    #[test]
    fn write_input_places_bytes_after_header_and_tracks_size() {
        let prefix = format!("ZISK_unittest_inputs_{}", std::process::id());
        let input_seg = shmem_input_name(&prefix);
        let control_seg = shmem_control_input_name(&prefix);
        create_segment(&input_seg, MAX_INPUT_SIZE as usize);
        create_segment(&control_seg, ControlShmem::CONTROL_WRITER_SIZE as usize);

        let control = std::sync::Arc::new(ControlShmem::new(&prefix, true).unwrap());
        let writer = InputsShmemWriter::new(&prefix, true, control).unwrap();

        // No semaphores bound → notify is a no-op; we only exercise the write path.
        writer.write_input(&[1u8, 2, 3, 4, 5, 6, 7, 8]).unwrap();

        // Input bytes land at offset 8 (after the 0u64 length header). Map only a
        // small window of the (1 GiB) segment: `ShmemReader` always uses
        // `MAP_LOCKED`, and locking the full size blows past `RLIMIT_MEMLOCK` on
        // CI runners. The test only ever reads the first 16 bytes.
        let r = ShmemReader::new(&input_seg, 4096).unwrap();
        assert_eq!(r.read_u64_at(8), u64::from_le_bytes([1, 2, 3, 4, 5, 6, 7, 8]));

        // The control plane's InputsSize slot (offset 16) tracks bytes written.
        let cr =
            ShmemReader::new(&control_seg, ControlShmem::CONTROL_WRITER_SIZE as usize).unwrap();
        assert_eq!(cr.read_u64_at(16), 8);

        // reset() clears the control plane.
        writer.reset();
        assert_eq!(cr.read_u64_at(16), 0);

        drop(r);
        drop(cr);
        unlink_segment(&input_seg);
        unlink_segment(&control_seg);
    }
}
