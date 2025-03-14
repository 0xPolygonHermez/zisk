use libc::{
    close, ftruncate, mmap, munmap, shm_open, shm_unlink, MAP_SHARED, O_CREAT, PROT_READ,
    PROT_WRITE, S_IRUSR, S_IWUSR, S_IXUSR,
};
use zisk_common::AsmInputC;
use zisk_common::{AsmOutputChunkC, AsmOutputHeader};

use zisk_common::{AsmMinimalTraces, EmuTrace};

use std::ffi::{c_void, CString};
use std::path::Path;
use std::process::Command;
use std::{fs, ptr};

pub struct AsmRunner;

#[allow(dead_code)]
pub enum AsmTraceLevel {
    None,
    Trace,
    ExtendedTrace,
}

pub struct AsmRunnerOptions {
    pub log_output: bool,
    pub metrics: bool,
    pub verbose: bool,
    pub trace_level: AsmTraceLevel,
    pub keccak_trace: bool,
}

impl Default for AsmRunnerOptions {
    fn default() -> Self {
        Self {
            log_output: false,
            metrics: false,
            verbose: false,
            trace_level: AsmTraceLevel::None,
            keccak_trace: false,
        }
    }
}

impl AsmRunner {
    pub fn run(
        inputs_path: &Path,
        ziskemuasm_path: &Path,
        max_steps: u64,
        chunk_size: u64,
        options: AsmRunnerOptions,
    ) -> AsmMinimalTraces {
        let pid = unsafe { libc::getpid() };

        let shmem_prefix = format!("SHM{}", pid);
        let shmem_input_name = format!("/{}_input", shmem_prefix);
        let shmem_output_name = format!("/{}_output", shmem_prefix);

        Self::write_input(inputs_path, &shmem_input_name, max_steps, chunk_size);

        // Prepare command
        let mut command = Command::new(ziskemuasm_path);
        if !options.log_output {
            command.arg("-o");
        }
        if options.metrics {
            command.arg("-m");
        }
        if options.verbose {
            command.arg("-v");
        }
        match options.trace_level {
            AsmTraceLevel::None => {}
            AsmTraceLevel::Trace => {
                command.arg("-t");
            }
            AsmTraceLevel::ExtendedTrace => {
                command.arg("-tt");
            }
        }
        if options.keccak_trace {
            command.arg("-k");
        }

        // Spawn child process
        if let Err(e) = command.arg(&shmem_prefix).spawn().and_then(|mut child| child.wait()) {
            eprintln!("Child process failed: {:?}", e);
        } else if options.verbose || options.log_output {
            println!("Child exited successfully");
        }

        let (mapped_ptr, vec_chunks) = Self::map_output(shmem_output_name.clone());

        AsmMinimalTraces::new(shmem_output_name, mapped_ptr, vec_chunks)
    }

    fn write_input(inputs_path: &Path, shmem_input_name: &str, max_steps: u64, chunk_size: u64) {
        let shmem_input_name = CString::new(shmem_input_name).expect("CString::new failed");
        let shmem_input_name_ptr = shmem_input_name.as_ptr();

        let inputs = fs::read(inputs_path).expect("Could not read inputs file");

        let asm_input = AsmInputC {
            chunk_size,
            max_steps,
            initial_trace_size: 1 << 30, // 1GB
            input_data_size: inputs.len() as u64,
        };

        // Shared memory size (aligned to 8 bytes)
        let shmem_input_size =
            ((inputs.len() + std::mem::size_of::<AsmInputC>() + 7) & !7) as usize;

        let mut shmem_input_data = Vec::with_capacity(shmem_input_size);
        shmem_input_data.extend_from_slice(&asm_input.to_bytes());
        shmem_input_data.extend_from_slice(&inputs);

        // Remove old shared memory if it exists
        unsafe { shm_unlink(shmem_input_name_ptr) };

        let shm_fd = unsafe {
            shm_open(shmem_input_name_ptr, libc::O_RDWR | O_CREAT, S_IRUSR | S_IWUSR | S_IXUSR)
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

    fn map_output(shmem_output_name: String) -> (*mut c_void, Vec<EmuTrace>) {
        let shmem_output_name = CString::new(shmem_output_name).expect("CString::new failed");
        let shmem_output_name_ptr = shmem_output_name.as_ptr();

        let shm_fd =
            unsafe { shm_open(shmem_output_name_ptr, libc::O_RDONLY, S_IRUSR | S_IWUSR | S_IXUSR) };

        Self::check_shm_open(shm_fd, shmem_output_name_ptr);

        // Read Output Header
        let output_header_size = size_of::<AsmOutputHeader>();
        let mapped_ptr =
            unsafe { mmap(ptr::null_mut(), output_header_size, PROT_READ, MAP_SHARED, shm_fd, 0) };
        Self::check_mmap(mapped_ptr, output_header_size, file!(), line!());

        let output_header = AsmOutputHeader::from_ptr(mapped_ptr);

        // Read Output
        let output_size = output_header_size + output_header.mt_used_size as usize;

        let mut mapped_ptr =
            unsafe { mmap(ptr::null_mut(), output_size, PROT_READ, MAP_SHARED, shm_fd, 0) };
        Self::check_mmap(mapped_ptr, output_size, file!(), line!());

        // println!("Output Header: {:?}", output_header);

        let mut vec_chunks;
        unsafe {
            mapped_ptr = mapped_ptr.add(output_header_size);
            let num_chunks = std::ptr::read(mapped_ptr as *const u64);
            mapped_ptr = (mapped_ptr as *mut u8).add(8) as *mut c_void;

            vec_chunks = Vec::with_capacity(num_chunks as usize);
            for _ in 0..num_chunks {
                let data = AsmOutputChunkC::to_emu_trace(&mut mapped_ptr);
                vec_chunks.push(data);
            }
        }

        (mapped_ptr, vec_chunks)
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
