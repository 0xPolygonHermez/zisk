use libc::{close, shm_unlink, PROT_READ, PROT_WRITE, S_IRUSR, S_IWUSR};
use tracing::error;

use std::{
    ffi::{c_void, CString},
    fs,
    path::Path,
    ptr,
    time::Duration,
};

use crate::{
    shmem_utils, AsmInputC2, AsmMTHeader, AsmRHData, AsmRHHeader, AsmRunError, AsmServices,
};
use anyhow::{Context, Result};
use named_sem::NamedSemaphore;

// This struct is used to run the assembly code in a separate process and generate the ROM histogram.
pub struct AsmRunnerRH {
    shmem_output_name: String,
    mapped_ptr: *mut c_void,
    pub asm_rowh_output: AsmRHData,
}

unsafe impl Send for AsmRunnerRH {}
unsafe impl Sync for AsmRunnerRH {}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Drop for AsmRunnerRH {
    fn drop(&mut self) {
        unsafe {
            // Forget all mem_reads Vec<u64> before unmapping
            std::mem::forget(std::mem::take(&mut self.asm_rowh_output));

            // Unmap shared memory
            libc::munmap(self.mapped_ptr, self.total_size());

            let shmem_output_name =
                CString::new(self.shmem_output_name.clone()).expect("CString::new failed");
            let shmem_output_name_ptr = shmem_output_name.as_ptr();

            shm_unlink(shmem_output_name_ptr);
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl AsmRunnerRH {
    pub fn new(
        shmem_output_name: String,
        mapped_ptr: *mut c_void,
        asm_rowh_output: AsmRHData,
    ) -> Self {
        AsmRunnerRH { shmem_output_name, mapped_ptr, asm_rowh_output }
    }

    fn total_size(&self) -> usize {
        std::mem::size_of_val(&self.asm_rowh_output.bios_inst_count)
            + std::mem::size_of_val(&self.asm_rowh_output.prog_inst_count)
            + std::mem::size_of::<AsmRHHeader>()
    }

    pub fn run(
        inputs_path: &Path,
        max_steps: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
    ) -> Result<AsmRunnerRH> {
        let prefix = AsmServices::shmem_prefix(&crate::AsmService::RH, base_port, local_rank);

        let shmem_input_name = format!("{prefix}_RH_input");
        let shmem_output_name = format!("{prefix}_RH_output");
        let sem_chunk_done_name = format!("/{prefix}_RH_chunk_done");

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        Self::write_input(inputs_path, &shmem_input_name);

        let asm_services = AsmServices::new(world_rank, local_rank, base_port);
        asm_services.send_rom_histogram_request(max_steps)?;

        match sem_chunk_done.timed_wait(Duration::from_secs(30)) {
            Err(e) => {
                error!("Semaphore '{}' error: {:?}", sem_chunk_done_name, e);

                return Err(AsmRunError::SemaphoreError(sem_chunk_done_name, e))
                    .context("Child process returned error");
            }
            _ => { /* continue */ }
        }

        let (mapped_ptr, asm_rowh_output) = Self::map_output(shmem_output_name.clone());

        Ok(AsmRunnerRH::new(shmem_output_name, mapped_ptr, asm_rowh_output))
    }

    fn write_input(inputs_path: &Path, shmem_input_name: &str) {
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
        let ptr = shmem_utils::map(fd, shmem_input_size, PROT_READ | PROT_WRITE, "input mmap");
        unsafe {
            ptr::copy_nonoverlapping(full_input.as_ptr(), ptr as *mut u8, shmem_input_size);
            shmem_utils::unmap(ptr, shmem_input_size);
            close(fd);
        }
    }

    fn get_output_ptr(shmem_output_name: &str) -> *mut std::ffi::c_void {
        let fd = shmem_utils::open_shmem(shmem_output_name, libc::O_RDONLY, S_IRUSR | S_IWUSR);
        let header_size = size_of::<AsmMTHeader>();
        let temp = shmem_utils::map(fd, header_size, PROT_READ, "header temp map");
        let header = unsafe { (temp as *const AsmMTHeader).read() };
        unsafe {
            shmem_utils::unmap(temp, header_size);
        }
        shmem_utils::map(fd, header.mt_allocated_size as usize, PROT_READ, shmem_output_name)
    }

    fn map_output(shmem_output_name: String) -> (*mut c_void, AsmRHData) {
        // Read the header data
        let header_ptr = Self::get_output_ptr(&shmem_output_name) as *const AsmRHHeader;

        let header = AsmRHHeader::from_ptr(header_ptr as *mut c_void);

        // Skips the header size to get the data pointer.
        let mut data_ptr = unsafe { header_ptr.add(1) } as *mut c_void;

        (data_ptr, AsmRHData::from_ptr(&mut data_ptr, header))
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
