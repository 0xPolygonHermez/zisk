use crate::{
    sem_chunk_done_name, shmem_output_name, AsmRHHeader, AsmRunError, AsmService, AsmServices,
    AsmSharedMemory, SEM_CHUNK_DONE_WAIT_DURATION,
};
use named_sem::NamedSemaphore;
use std::sync::atomic::{fence, Ordering};
use tracing::error;
use zisk_common::{stats_begin, stats_end, ExecutorStatsHandle, RomHistogramData};

use anyhow::{Context, Result};

pub struct RHShMemReader {
    pub output_shmem: AsmSharedMemory<AsmRHHeader>,
}

impl RHShMemReader {
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
    pub rom_histogram: RomHistogramData,
}

impl Drop for AsmRunnerRH {
    fn drop(&mut self) {
        // Forget all mem_reads Vec<u64> before unmapping
        std::mem::forget(std::mem::take(&mut self.rom_histogram));
    }
}

impl AsmRunnerRH {
    pub fn new(rom_histogram: RomHistogramData) -> Self {
        AsmRunnerRH { rom_histogram }
    }

    pub fn run(
        asm_shared_memory: &mut Option<RHShMemReader>,
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
                Some(RHShMemReader::new(local_rank, base_port, unlock_mapped_memory)?);
        }

        let rom_histogram =
            rom_histogram_from_shared_memory(&asm_shared_memory.as_ref().unwrap().output_shmem);

        stats_end!(_stats, &_runner_scope);
        Ok(AsmRunnerRH::new(rom_histogram))
    }
}

/// Reads ROM histogram data from shared memory produced by the assembly runner.
fn rom_histogram_from_shared_memory(
    asm_shared_memory: &AsmSharedMemory<AsmRHHeader>,
) -> RomHistogramData {
    unsafe {
        let data_ptr = asm_shared_memory.data_ptr() as *mut u64;
        // BIOS chunk data
        let bios_data_ptr = data_ptr;
        let bios_len = std::ptr::read(bios_data_ptr) as usize;
        let bios_data_ptr = bios_data_ptr.add(1);
        let bios_inst_count = Vec::from_raw_parts(bios_data_ptr, bios_len, bios_len);

        // Advance pointer after BIOS
        let prog_data_ptr = bios_data_ptr.add(bios_len);

        // Program chunk data
        let prog_len = std::ptr::read(prog_data_ptr) as usize;
        let prog_data_ptr = prog_data_ptr.add(1);
        let prog_inst_count = Vec::from_raw_parts(prog_data_ptr, prog_len, prog_len);

        RomHistogramData {
            steps: asm_shared_memory.map_header().steps,
            bios_inst_count,
            prog_inst_count,
        }
    }
}
