use libc::{close, PROT_READ, PROT_WRITE, S_IRUSR, S_IWUSR};
use tracing::error;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::{Arc, Mutex};
use std::{fs, path::Path, ptr, time::Duration};

use crate::{
    shmem_utils, AsmInputC2, AsmRHData, AsmRHHeader, AsmRunError, AsmService, AsmServices,
    AsmSharedMemory,
};
use anyhow::{Context, Result};
use named_sem::NamedSemaphore;
use std::sync::atomic::{fence, Ordering};

// This struct is used to run the assembly code in a separate process and generate the ROM histogram.
pub struct AsmRunnerRH {
    pub asm_rowh_output: AsmRHData,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Drop for AsmRunnerRH {
    fn drop(&mut self) {
        // Forget all mem_reads Vec<u64> before unmapping
        std::mem::forget(std::mem::take(&mut self.asm_rowh_output));
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl AsmRunnerRH {
    pub fn new(asm_rowh_output: AsmRHData) -> Self {
        AsmRunnerRH { asm_rowh_output }
    }

    pub fn create_shmem(
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<AsmSharedMemory<AsmRHHeader>> {
        AsmSharedMemory::create_shmem(AsmService::RH, local_rank, base_port, unlock_mapped_memory)
    }

    pub fn run(
        asm_shared_memory: Arc<Mutex<Option<AsmSharedMemory<AsmRHHeader>>>>,
        inputs_path: &Path,
        max_steps: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<AsmRunnerRH> {
        let (shmem_input_name, _, sem_chunk_done_name) =
            AsmSharedMemory::<AsmRHHeader>::shmem_names(AsmService::RH, base_port, local_rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        Self::write_input(inputs_path, &shmem_input_name, unlock_mapped_memory);

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
                AsmRunnerRH::create_shmem(local_rank, base_port, unlock_mapped_memory)
                    .expect("Error creating assembly shared memory"),
            );
        }

        let asm_rowh_output = AsmRHData::from_shared_memory(asm_shared_memory.as_ref().unwrap());

        Ok(AsmRunnerRH::new(asm_rowh_output))
    }

    fn write_input(inputs_path: &Path, shmem_input_name: &str, unlock_mapped_memory: bool) {
        let inputs = fs::read(inputs_path).expect("Failed to read input file");
        let asm_input = AsmInputC2 { zero: 0, input_data_size: inputs.len() as u64 };
        let shmem_input_size = (inputs.len() + size_of::<AsmInputC2>() + 7) & !7;

        let mut full_input = Vec::with_capacity(shmem_input_size);
        full_input.extend_from_slice(&asm_input.to_bytes());
        full_input.extend_from_slice(&inputs);
        while full_input.len() < shmem_input_size {
            full_input.push(0);
        }

        let fd = shmem_utils::open_shmem(shmem_input_name, libc::O_RDWR, S_IRUSR | S_IWUSR);
        let ptr = shmem_utils::map(
            fd,
            shmem_input_size,
            PROT_READ | PROT_WRITE,
            unlock_mapped_memory,
            "RH input mmap",
        );
        unsafe {
            ptr::copy_nonoverlapping(full_input.as_ptr(), ptr as *mut u8, shmem_input_size);
            shmem_utils::unmap(ptr, shmem_input_size);
            close(fd);
        }
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
impl AsmRunnerRH {
    pub fn new(
        _shmem_output_name: String,
        _mapped_ptr: *mut c_void,
        _asm_rowh_output: AsmRHData,
    ) -> Self {
        panic!("AsmRunnerRomH::new() is not supported on this platform. Only Linux x86_64 is supported.");
    }

    pub fn run(
        _rom_asm_path: &Path,
        _inputs_path: Option<&Path>,
        _shm_size: u64,
        _options: AsmRunnerOptions,
    ) -> AsmRunnerRH {
        panic!("AsmRunnerRomH::run() is not supported on this platform. Only Linux x86_64 is supported.");
    }
}
