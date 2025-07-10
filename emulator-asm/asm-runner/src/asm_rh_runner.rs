use tracing::error;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{AsmRHData, AsmRHHeader, AsmRunError, AsmService, AsmServices, AsmSharedMemory};
use anyhow::{Context, Result};
use named_sem::NamedSemaphore;
use std::sync::atomic::{fence, Ordering};

// This struct is used to run the assembly code in a separate process and generate the ROM histogram.
pub struct AsmRunnerRH {
    pub asm_rowh_output: AsmRHData,
}

impl Drop for AsmRunnerRH {
    fn drop(&mut self) {
        // Forget all mem_reads Vec<u64> before unmapping
        std::mem::forget(std::mem::take(&mut self.asm_rowh_output));
    }
}

impl AsmRunnerRH {
    pub fn new(asm_rowh_output: AsmRHData) -> Self {
        AsmRunnerRH { asm_rowh_output }
    }

    pub fn create_shmem(
        local_rank: i32,
        unlock_mapped_memory: bool,
    ) -> Result<AsmSharedMemory<AsmRHHeader>> {
        AsmSharedMemory::create_shmem(AsmService::RH, local_rank, unlock_mapped_memory)
    }

    pub fn run(
        asm_shared_memory: Arc<Mutex<Option<AsmSharedMemory<AsmRHHeader>>>>,
        max_steps: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<AsmRunnerRH> {
        let sem_chunk_done_name =
            AsmSharedMemory::<AsmRHHeader>::shmem_chunk_done_name(AsmService::RH, local_rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        let asm_services = AsmServices::new(world_rank, local_rank, base_port);
        asm_services.send_rom_histogram_request(max_steps)?;

        match sem_chunk_done.timed_wait(Duration::from_secs(30)) {
            Err(e) => {
                error!("Semaphore '{}' error: {:?}", sem_chunk_done_name, e);

                return Err(AsmRunError::SemaphoreError(sem_chunk_done_name, e))
                    .context("Child process returned error");
            }
            Ok(()) => {
                // Synchronize with memory changes from the C++ side
                fence(Ordering::Acquire);
            }
        }

        let mut asm_shared_memory = asm_shared_memory.lock().unwrap();
        if asm_shared_memory.is_none() {
            *asm_shared_memory = Some(
                AsmSharedMemory::create_shmem(AsmService::RH, local_rank, unlock_mapped_memory)
                    .expect("Error creating MO assembly shared memory"),
            );
        }

        let asm_rowh_output = AsmRHData::from_shared_memory(asm_shared_memory.as_ref().unwrap());

        Ok(AsmRunnerRH::new(asm_rowh_output))
    }
}
