use std::sync::{Arc, Mutex};

use named_sem::NamedSemaphore;
use zisk_common::{
    io::{StreamProcessor, StreamSink},
    reinterpret_vec,
};
use zisk_core::MAX_INPUT_SIZE;

use crate::{
    sem_input_avail_name, shmem_input_name, AsmServices, ControlShmem, SharedMemoryWriter,
};

use anyhow::Result;

pub struct InputsShmemWriter {
    writer: Mutex<SharedMemoryWriter>,
    control_writer: Arc<ControlShmem>,
    sem_avails: Mutex<Option<Vec<NamedSemaphore>>>,
}

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
        let mut writer =
            SharedMemoryWriter::new(&name, MAX_INPUT_SIZE as usize, unlock_mapped_memory)
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

    pub fn write_input(&self, inputs: &[u8]) -> Result<()> {
        if inputs.is_empty() {
            return Ok(());
        }

        self.writer.lock().unwrap().write_at(8, inputs)?;
        self.control_writer.inc_inputs_size(inputs.len())?;
        self.notify_all_services()?;
        Ok(())
    }

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
    fn submit(&self, hints: &[u64]) -> anyhow::Result<()> {
        self.append_input(&reinterpret_vec(hints.to_vec())?)
    }

    fn reset(&self) {
        self.reset();
    }
}

impl StreamProcessor for InputsShmemWriter {
    fn process_hints(&self, data: &[u64], _first_batch: bool) -> anyhow::Result<bool> {
        self.submit(data)?;
        Ok(false)
    }

    fn reset(&self) {
        InputsShmemWriter::reset(self);
    }
}
