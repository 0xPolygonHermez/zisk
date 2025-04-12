use libc::{
    close, ftruncate, mmap, munmap, shm_open, shm_unlink, MAP_SHARED, O_CREAT, PROT_READ,
    PROT_WRITE, S_IRUSR, S_IWUSR, S_IXUSR,
};

use zisk_common::EmuTrace;

use std::ffi::{c_void, CString};
use std::path::Path;
use std::process::{self, Command};
use std::{fs, ptr};

use log::info;

use crate::{AsmInputC, AsmMTChunk, AsmMTHeader, AsmRunnerOptions, AsmRunnerTraceLevel};

// This struct is used to run the assembly code in a separate process and generate minimal traces.
#[derive(Debug)]
pub struct AsmRunnerMT {
    shmem_output_name: String,
    mapped_ptr: *mut c_void,
    pub vec_chunks: Vec<EmuTrace>,
}

unsafe impl Send for AsmRunnerMT {}
unsafe impl Sync for AsmRunnerMT {}

impl Drop for AsmRunnerMT {
    fn drop(&mut self) {
        unsafe {
            // Forget all mem_reads Vec<u64> before unmapping
            for chunk in &mut self.vec_chunks {
                std::mem::forget(std::mem::take(&mut chunk.mem_reads));
            }

            // Unmap shared memory
            libc::munmap(self.mapped_ptr, self.total_size());

            let shmem_output_name =
                CString::new(self.shmem_output_name.clone()).expect("CString::new failed");
            let shmem_output_name_ptr = shmem_output_name.as_ptr();

            shm_unlink(shmem_output_name_ptr);
        }
    }
}

impl AsmRunnerMT {
    pub fn new(
        shmem_output_name: String,
        mapped_ptr: *mut c_void,
        vec_chunks: Vec<EmuTrace>,
    ) -> Self {
        AsmRunnerMT { shmem_output_name, mapped_ptr, vec_chunks }
    }

    fn total_size(&self) -> usize {
        self.vec_chunks.iter().map(|chunk| std::mem::size_of_val(&chunk.mem_reads)).sum::<usize>()
            + std::mem::size_of::<AsmMTHeader>()
    }

    pub fn run(
        ziskemuasm_path: &Path,
        inputs_path: &Path,
        shm_size: u64,
        chunk_size: u64,
        options: AsmRunnerOptions,
    ) -> AsmRunnerMT {
        let pid = unsafe { libc::getpid() };

        let shmem_prefix = format!("SHM{}", pid);
        let shmem_input_name = format!("/{}_input", shmem_prefix);
        let shmem_output_name = format!("/{}_output", shmem_prefix);

        Self::write_input(inputs_path, &shmem_input_name, shm_size, chunk_size);

        // Prepare command
        let mut command = Command::new(ziskemuasm_path);

        command.arg("--generate_minimal_trace");

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
        let start = std::time::Instant::now();
        if let Err(e) = command.arg(&shmem_prefix).spawn().and_then(|mut child| child.wait()) {
            eprintln!("Child process failed: {:?}", e);
        } else if options.verbose || options.log_output {
            println!("Child exited successfully");
        }
        let stop = start.elapsed();

        let (mapped_ptr, vec_chunks) = Self::map_output(shmem_output_name.clone());

        let total_steps = vec_chunks.iter().map(|x| x.steps).sum::<u64>();
        let mhz = (total_steps as f64 / stop.as_secs_f64()) / 1_000_000.0;
        info!("AsmRnner: ··· Assembly execution speed: {:.2} MHz", mhz);

        AsmRunnerMT::new(shmem_output_name, mapped_ptr, vec_chunks)
    }

    fn write_input(inputs_path: &Path, shmem_input_name: &str, max_steps: u64, chunk_size: u64) {
        let shmem_input_name = CString::new(shmem_input_name).expect("CString::new failed");
        let shmem_input_name_ptr = shmem_input_name.as_ptr();

        let inputs = fs::read(inputs_path).expect("Could not read inputs file");

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
        let output_header_size = size_of::<AsmMTHeader>();
        let mapped_ptr =
            unsafe { mmap(ptr::null_mut(), output_header_size, PROT_READ, MAP_SHARED, shm_fd, 0) };
        Self::check_mmap(mapped_ptr, output_header_size, file!(), line!());

        let output_header = AsmMTHeader::from_ptr(mapped_ptr);

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
                let data = AsmMTChunk::to_emu_trace(&mut mapped_ptr);
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
