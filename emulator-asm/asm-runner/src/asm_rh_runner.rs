use crate::{
    sem_chunk_done_name, shmem_output_name, AsmRHData, AsmRHHeader, AsmRunError, AsmService,
    AsmServices, AsmSharedMemory, SEM_CHUNK_DONE_WAIT_DURATION,
};
use named_sem::NamedSemaphore;
use std::sync::atomic::{fence, Ordering};
use tracing::error;
use zisk_common::{stats_begin, stats_end, ExecutorStatsHandle};

use anyhow::{Context, Result};

pub struct RHOutputShmem {
    pub output_shmem: AsmSharedMemory<AsmRHHeader>,
}

impl RHOutputShmem {
    pub fn new(
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        let port = AsmServices::port_base_for(base_port, local_rank);

        let output_name = shmem_output_name(port, AsmService::RH, local_rank, Some(0));

        let output_shared_memory =
            AsmSharedMemory::<AsmRHHeader>::open_and_map(&output_name, unlock_mapped_memory)?;

        Ok(Self { output_shmem: output_shared_memory })
    }
}

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

    pub fn run(
        asm_shared_memory: &mut Option<RHOutputShmem>,
        max_steps: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        _stats: ExecutorStatsHandle,
    ) -> Result<AsmRunnerRH> {
        stats_begin!(_stats, 0, _runner_scope, "ASM_RH_RUNNER", 0);

        let port = AsmServices::port_base_for(base_port, local_rank);

        let sem_chunk_done_name = sem_chunk_done_name(port, AsmService::RH, local_rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        let asm_services = AsmServices::new(world_rank, local_rank, base_port);
        asm_services.send_rom_histogram_request(max_steps)?;

        loop {
            match sem_chunk_done.timed_wait(SEM_CHUNK_DONE_WAIT_DURATION) {
                Ok(()) => {
                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);
                    break;
                }
                Err(named_sem::Error::WaitFailed(e))
                    if e.kind() == std::io::ErrorKind::Interrupted =>
                {
                    continue
                }
                Err(e) => {
                    error!("Semaphore '{}' error: {:?}", sem_chunk_done_name, e);

                    return Err(AsmRunError::SemaphoreError(sem_chunk_done_name, e))
                        .context("Child process returned error");
                }
            }
        }

        if asm_shared_memory.is_none() {
            *asm_shared_memory =
                Some(RHOutputShmem::new(local_rank, base_port, unlock_mapped_memory)?);
        }

        let asm_rowh_output =
            AsmRHData::from_shared_memory(&asm_shared_memory.as_ref().unwrap().output_shmem);

        stats_end!(_stats, &_runner_scope);
        Ok(AsmRunnerRH::new(asm_rowh_output))
    }
}
