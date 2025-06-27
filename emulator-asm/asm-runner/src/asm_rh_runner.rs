use libc::{
    close, ftruncate, mmap, munmap, shm_open, shm_unlink, MAP_SHARED, O_CREAT, PROT_READ,
    PROT_WRITE, S_IRUSR, S_IWUSR,
};

use crate::AsmInputC;
use named_sem::NamedSemaphore;

use std::ffi::{c_uint, c_void, CString};
use std::path::Path;
use std::process::{self, Command};
use std::{fs, ptr};

use crate::{AsmRHData, AsmRHHeader, AsmRunnerOptions, AsmRunnerTraceLevel};

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
        rom_asm_path: &Path,
        inputs_path: Option<&Path>,
        shm_size: u64,
        options: AsmRunnerOptions,
    ) -> AsmRunnerRH {
        let pid = unsafe { libc::getpid() };

        let shmem_prefix = format!("ZISKRH{}", pid);
        let shmem_input_name = format!("/{}_input", shmem_prefix);
        let shmem_output_name = format!("/{}_output", shmem_prefix);

        // Build semaphores names, and create them (if they don not already exist)
        let sem_output_name = format!("/{}_semout", shmem_prefix);
        let sem_input_name = format!("/{}_semin", shmem_prefix);
        let mut semin = NamedSemaphore::create(sem_input_name.clone(), 0).unwrap_or_else(|e| {
            panic!(
                "AsmRunnerRomH::run() failed calling NamedSemaphore::create({}), error: {}",
                sem_input_name, e
            )
        });
        let mut semout = NamedSemaphore::create(sem_output_name.clone(), 0).unwrap_or_else(|e| {
            panic!(
                "AsmRunnerRomH::run() failed calling NamedSemaphore::create({}), error: {}",
                sem_output_name, e
            )
        });

        Self::write_input(inputs_path, &shmem_input_name, shm_size, 0);

        // Prepare command
        let mut command = Command::new(rom_asm_path);

        command.arg("--generate_rom_histogram");

        if !options.log_output {
            command.arg("-o");
            command.stdout(process::Stdio::null());
            command.stderr(process::Stdio::null());
        }
        if options.metrics {
            command.arg("-m");
        }
        if options.verbose {
            command.arg("-v");
        }
        match options.trace_level {
            AsmRunnerTraceLevel::None => {}
            AsmRunnerTraceLevel::Trace => {
                command.arg("-t");
            }
            AsmRunnerTraceLevel::ExtendedTrace => {
                command.arg("-tt");
            }
        }
        if options.keccak_trace {
            command.arg("-k");
        }

        // Spawn child process
        if let Err(e) = command.arg(&shmem_prefix).spawn() {
            tracing::error!("Child process failed: {:?}", e);
        } else if options.verbose || options.log_output {
            tracing::info!("Child process launched successfully");
        }

        // Wait for the assembly emulator to complete writing the trace
        if let Err(e) = semin.wait() {
            panic!(
                "AsmRunnerRomH::run() failed calling semin.wait({}), error: {}",
                sem_input_name, e
            );
        }

        let (mapped_ptr, asm_rowh_output) = Self::map_output(shmem_output_name.clone());

        // Tell the assembly that we are done reading the trace
        if let Err(e) = semout.post() {
            panic!(
                "AsmRunnerRomH::run() failed calling semout.post({}), error: {}",
                sem_output_name, e
            );
        }

        AsmRunnerRH::new(shmem_output_name, mapped_ptr, asm_rowh_output)
    }

    fn write_input(
        inputs_path: Option<&Path>,
        shmem_input_name: &str,
        max_steps: u64,
        chunk_size: u64,
    ) {
        let shmem_input_name = CString::new(shmem_input_name).expect("CString::new failed");
        let shmem_input_name_ptr = shmem_input_name.as_ptr();

        let inputs = if let Some(inputs_path) = inputs_path {
            fs::read(inputs_path).expect("Could not read inputs file")
        } else {
            vec![]
        };

        let asm_input = AsmInputC {
            chunk_size,
            max_steps,
            initial_trace_size: 1u64 << 32, // 4GB
            input_data_size: inputs.len() as u64,
        };

        // Shared memory size (aligned to 8 bytes)
        let shmem_input_size = (inputs.len() + std::mem::size_of::<AsmInputC>() + 7) & !7;

        let mut shmem_input_data = Vec::with_capacity(shmem_input_size);
        shmem_input_data.extend_from_slice(&asm_input.to_bytes());
        shmem_input_data.extend_from_slice(&inputs);
        while shmem_input_data.len() < shmem_input_size {
            shmem_input_data.push(0);
        }

        // Remove old shared memory if it exists
        unsafe { shm_unlink(shmem_input_name_ptr) };

        let shm_fd = unsafe {
            shm_open(shmem_input_name_ptr, libc::O_RDWR | O_CREAT, (S_IRUSR | S_IWUSR) as c_uint)
        };
        Self::check_shm_open(shm_fd, shmem_input_name_ptr);

        if unsafe { ftruncate(shm_fd, shmem_input_size as i64) } < 0 {
            panic!("ftruncate failed");
        }

        let mapped_ptr = unsafe {
            mmap(
                std::ptr::null_mut(),
                shmem_input_size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                shm_fd,
                0,
            )
        };
        Self::check_mmap(mapped_ptr, shmem_input_size, file!(), line!());

        unsafe {
            std::ptr::copy_nonoverlapping(
                shmem_input_data.as_ptr(),
                mapped_ptr as *mut u8,
                shmem_input_size,
            );

            munmap(mapped_ptr, shmem_input_size);
            close(shm_fd);
        }
    }

    fn map_output(shmem_output_name: String) -> (*mut c_void, AsmRHData) {
        let shmem_output_name = CString::new(shmem_output_name).expect("CString::new failed");
        let shmem_output_name_ptr = shmem_output_name.as_ptr();

        let shm_fd = unsafe {
            shm_open(shmem_output_name_ptr, libc::O_RDONLY, (S_IRUSR | S_IWUSR) as c_uint)
        };

        Self::check_shm_open(shm_fd, shmem_output_name_ptr);

        // Read Output Header
        let output_header_size = size_of::<AsmRHHeader>();
        let mapped_ptr =
            unsafe { mmap(ptr::null_mut(), output_header_size, PROT_READ, MAP_SHARED, shm_fd, 0) };
        Self::check_mmap(mapped_ptr, output_header_size, file!(), line!());

        let output_header = AsmRHHeader::from_ptr(mapped_ptr);

        // Read Output
        let output_size = output_header_size + output_header.mt_allocated_size as usize;

        let mut mapped_ptr =
            unsafe { mmap(ptr::null_mut(), output_size, PROT_READ, MAP_SHARED, shm_fd, 0) };
        Self::check_mmap(mapped_ptr, output_size, file!(), line!());

        // println!("Output Header: {:?}", output_header);

        unsafe {
            mapped_ptr = mapped_ptr.add(output_header_size);

            (mapped_ptr, AsmRHData::from_ptr(&mut mapped_ptr, output_header))
        }
    }

    fn check_shm_open(shm_fd: i32, name: *const i8) {
        if shm_fd == -1 {
            let err = std::io::Error::last_os_error();
            panic!("shm_open({:?}) failed: {:?}", name, err);
        }
    }

    fn check_mmap(ptr: *mut libc::c_void, size: usize, file: &str, line: u32) {
        if ptr == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            panic!("mmap failed: {:?} (size: {} bytes) at {}:{}", err, size, file, line);
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
