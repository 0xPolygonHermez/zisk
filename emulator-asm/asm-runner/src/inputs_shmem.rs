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
    writers: Vec<Mutex<SharedMemoryWriter>>,
    control_writer: Arc<ControlShmem>,
    sem_avails: Mutex<Vec<NamedSemaphore>>,
}

unsafe impl Send for InputsShmemWriter {}
unsafe impl Sync for InputsShmemWriter {}

impl InputsShmemWriter {
    pub fn new(
        shm_prefix: &str,
        sem_prefix: &str,
        unlock_mapped_memory: bool,
        control_writer: Arc<ControlShmem>,
    ) -> Result<Self> {
        let writers = AsmServices::SERVICES
            .iter()
            .map(|service| {
                let name = shmem_input_name(shm_prefix, *service);
                let mut writer =
                    SharedMemoryWriter::new(&name, MAX_INPUT_SIZE as usize, unlock_mapped_memory)?;
                writer.reset();
                writer.append_input(&0u64.to_le_bytes())?;
                Ok(Mutex::new(writer))
            })
            .collect::<Result<Vec<_>>>()?;

        let sem_avails: Vec<NamedSemaphore> = AsmServices::SERVICES
            .iter()
            .map(|service| {
                let name = sem_input_avail_name(sem_prefix, *service);
                NamedSemaphore::create(&name, 0)
                    .map_err(|e| anyhow::anyhow!("Failed to create semaphore '{}': {}", name, e))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { writers, control_writer, sem_avails: Mutex::new(sem_avails) })
    }

    pub fn write_input(&self, inputs: &[u8]) -> Result<()> {
        if inputs.is_empty() {
            return Ok(());
        }

        for writer in &self.writers {
            writer.lock().unwrap().write_at(8, inputs)?;
        }
        self.control_writer.inc_inputs_size(inputs.len());
        self.notify_all_services()?;
        Ok(())
    }

    pub fn append_input(&self, inputs: &[u8]) -> Result<()> {
        for writer in &self.writers {
            writer.lock().unwrap().append_input(inputs)?;
        }
        self.control_writer.inc_inputs_size(inputs.len());
        self.notify_all_services()?;
        Ok(())
    }

    fn notify_all_services(&self) -> Result<()> {
        for sem in self.sem_avails.lock().unwrap().iter_mut() {
            sem.post()?;
        }
        Ok(())
    }

    pub fn reset(&self) {
        for writer in &self.writers {
            let mut w = writer.lock().unwrap();
            w.reset();
            w.append_input(&0u64.to_le_bytes())
                .expect("Failed to write initial header after reset");
        }
        self.control_writer.reset();
        for sem in self.sem_avails.lock().unwrap().iter_mut() {
            while sem.try_wait().is_ok() {}
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
