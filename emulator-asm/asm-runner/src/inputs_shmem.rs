use std::sync::{Arc, Mutex};

use named_sem::NamedSemaphore;
use zisk_common::{io::StreamSink, reinterpret_vec};
use zisk_core::MAX_INPUT_SIZE;

use crate::{
    sem_input_avail_name, shmem_input_name, AsmServices, ControlShmem, SharedMemoryWriter,
};

use anyhow::Result;

pub struct InputsShmemWriter {
    writer: Mutex<SharedMemoryWriter>,
    control_writer: Arc<ControlShmem>,
    sem_avails: Mutex<Vec<NamedSemaphore>>,
}

unsafe impl Send for InputsShmemWriter {}
unsafe impl Sync for InputsShmemWriter {}

impl InputsShmemWriter {
    pub fn new(
        base_port: Option<u16>,
        local_rank: i32,
        unlock_mapped_memory: bool,
        control_writer: Arc<ControlShmem>,
    ) -> Result<Self> {
        let port = AsmServices::port_base_for(base_port, local_rank);

        let mut writer = SharedMemoryWriter::new(
            &shmem_input_name(port, local_rank),
            MAX_INPUT_SIZE as usize,
            unlock_mapped_memory,
        )?;

        writer.reset();
        writer.append_input(&0u64.to_le_bytes())?;

        // Create one semaphore per ASM service
        let sem_avails: Vec<NamedSemaphore> = AsmServices::SERVICES
            .iter()
            .map(|service| {
                let name = sem_input_avail_name(port, *service, local_rank);
                NamedSemaphore::create(&name, 0)
                    .map_err(|e| anyhow::anyhow!("Failed to create semaphore '{}': {}", name, e))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            writer: Mutex::new(writer),
            control_writer,
            sem_avails: Mutex::new(sem_avails),
        })
    }

    pub fn write_input(&self, inputs: &[u8]) -> Result<()> {
        self.writer.lock().unwrap().write_at(8, inputs)?;
        self.control_writer.inc_inputs_size(inputs.len());
        self.notify_all_services()?;

        Ok(())
    }

    pub fn append_input(&self, inputs: &[u8]) -> Result<()> {
        self.writer.lock().unwrap().append_input(inputs)?;
        self.control_writer.inc_inputs_size(inputs.len());
        self.notify_all_services()?;

        Ok(())
    }

    /// Notify all ASM services that new input data is available
    fn notify_all_services(&self) -> Result<()> {
        for sem in self.sem_avails.lock().unwrap().iter_mut() {
            sem.post()?;
        }
        Ok(())
    }

    pub fn reset(&self) {
        let mut writer = self.writer.lock().unwrap();
        writer.reset();
        writer
            .append_input(&0u64.to_le_bytes())
            .expect("Failed to write initial header after reset");

        self.control_writer.reset();
        // Drain all the semaphore signals from all services
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
