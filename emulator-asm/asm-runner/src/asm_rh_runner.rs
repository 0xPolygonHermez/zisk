use tracing::error;
use zisk_common::ExecutorStatsHandle;

use crate::{AsmRHData, AsmRHHeader, AsmRunError, AsmService, AsmServices, AsmSharedMemory};
use anyhow::{Context, Result};
use named_sem::NamedSemaphore;
use std::sync::atomic::{fence, Ordering};
use std::time::Duration;

pub struct PreloadedRH {
    pub output_shmem: AsmSharedMemory<AsmRHHeader>,
}

impl PreloadedRH {
    pub fn new(
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        let port = if let Some(base_port) = base_port {
            AsmServices::port_for(&AsmService::RH, base_port, local_rank)
        } else {
            AsmServices::default_port(&AsmService::RH, local_rank)
        };

        let output_name =
            AsmSharedMemory::<AsmRHHeader>::shmem_output_name(port, AsmService::RH, local_rank);

        let output_shared_memory =
            AsmSharedMemory::<AsmRHHeader>::open_and_map(&output_name, unlock_mapped_memory)?;

        Ok(Self { output_shmem: output_shared_memory })
    }
}

#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;

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
        asm_shared_memory: &mut Option<PreloadedRH>,
        max_steps: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        _stats: ExecutorStatsHandle,
    ) -> Result<AsmRunnerRH> {
        let __stats = _stats.clone();

        #[cfg(feature = "stats")]
        let parent_stats_id = __stats.next_id();
        #[cfg(feature = "stats")]
        _stats.add_stat(0, parent_stats_id, "ASM_RH_RUNNER", 0, ExecutorStatsEvent::Begin);

        let port = if let Some(base_port) = base_port {
            AsmServices::port_for(&AsmService::RH, base_port, local_rank)
        } else {
            AsmServices::default_port(&AsmService::RH, local_rank)
        };

        let sem_chunk_done_name =
            AsmSharedMemory::<AsmRHHeader>::shmem_chunk_done_name(port, AsmService::RH, local_rank);

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

        if asm_shared_memory.is_none() {
            *asm_shared_memory =
                Some(PreloadedRH::new(local_rank, base_port, unlock_mapped_memory)?);
        }

        let asm_rowh_output =
            AsmRHData::from_shared_memory(&asm_shared_memory.as_ref().unwrap().output_shmem);

        // Add to executor stats
        #[cfg(feature = "stats")]
        _stats.add_stat(0, parent_stats_id, "ASM_RH_RUNNER", 0, ExecutorStatsEvent::End);

        Ok(AsmRunnerRH::new(asm_rowh_output))
    }
}
