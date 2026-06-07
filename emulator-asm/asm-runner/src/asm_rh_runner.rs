use crate::{
    sem_chunk_done_name, shmem_output_name, AsmRHData, AsmRHHeader, AsmRunError, AsmService,
    AsmServices, AsmShmem, SEM_CHUNK_DONE_WAIT_DURATION,
};
use named_sem::NamedSemaphore;
use std::sync::atomic::{fence, Ordering};
use tracing::{error, warn};
use zisk_common::{stats_begin, stats_end, ExecutorStatsHandle};

use anyhow::{Context, Result};

pub struct RHShmemReader {
    pub(crate) output_shmem: AsmShmem<AsmRHHeader>,
}

impl RHShmemReader {
    pub fn new(shm_prefix: &str, unlock_mapped_memory: bool) -> Result<Self> {
        let output_name = shmem_output_name(shm_prefix, AsmService::RH, Some(0));

        let output_shared_memory =
            AsmShmem::<AsmRHHeader>::open_and_map(&output_name, unlock_mapped_memory)?;

        Ok(Self { output_shmem: output_shared_memory })
    }
}

// This struct is used to run the assembly code in a separate process and generate the ROM histogram.
pub struct AsmRunnerRH {
    pub asm_rowh_output: AsmRHData,
}

impl Drop for AsmRunnerRH {
    fn drop(&mut self) {
        // Load-bearing invariant — pairs with `AsmRHData::from_shared_memory`
        // (asm_rh.rs): `asm_rowh_output.inst_count` is a `Vec` built via
        // `Vec::from_raw_parts` over the shared-memory mapping, NOT allocated by
        // Rust's allocator. Letting it drop normally would free foreign pages
        // (UB), so `forget` it here before the mapping is torn down. Do not
        // remove this without also changing the `from_raw_parts` construction.
        std::mem::forget(std::mem::take(&mut self.asm_rowh_output));
    }
}

impl AsmRunnerRH {
    pub fn new(asm_rowh_output: AsmRHData) -> Self {
        AsmRunnerRH { asm_rowh_output }
    }

    pub fn run(
        asm_shared_memory: &mut Option<RHShmemReader>,
        max_steps: u64,
        asm_services: AsmServices,
        unlock_mapped_memory: bool,
        _stats: ExecutorStatsHandle,
    ) -> Result<AsmRunnerRH> {
        stats_begin!(_stats, 0, _runner_scope, "ASM_RH_RUNNER", 0);

        let sem_chunk_done_name = sem_chunk_done_name(asm_services.sem_prefix(), AsmService::RH);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        let stale = crate::drain_chunk_done(&mut sem_chunk_done);
        if stale > 0 {
            warn!(
                "RH semaphore '{sem_chunk_done_name}' had {stale} stale chunk_done post(s) at run start; a prior run skipped its end-side cleanup"
            );
        }

        let result: Result<AsmRunnerRH> = (|| -> Result<AsmRunnerRH> {
            tracing::debug!("[RH] Sending histogram request...");
            asm_services.send_rom_histogram_request(max_steps)?;
            tracing::debug!("[RH] Histogram request sent, waiting for completion...");
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

                        return Err(AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))
                            .context("Child process returned error");
                    }
                }
            }
            tracing::debug!(
                "[RH] Histogram computation completed, reading results from shared memory..."
            );
            if asm_shared_memory.is_none() {
                *asm_shared_memory =
                    Some(RHShmemReader::new(asm_services.shm_prefix(), unlock_mapped_memory)?);
            }
            tracing::debug!("[RH] Shared memory mapped, processing results...");
            let reader = asm_shared_memory.as_ref().ok_or_else(|| {
                anyhow::anyhow!("ASM_RH_RUNNER: asm_shared_memory is None after initialization")
            })?;
            let asm_rowh_output = AsmRHData::from_shared_memory(&reader.output_shmem);
            tracing::debug!("[RH] Results processed successfully.");
            stats_end!(_stats, &_runner_scope);
            Ok(AsmRunnerRH::new(asm_rowh_output))
        })();

        crate::drain_chunk_done(&mut sem_chunk_done);

        result
    }
}
