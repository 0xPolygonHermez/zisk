use libc::{
    close, ftruncate, mmap, munmap, shm_open, shm_unlink, MAP_SHARED, O_CREAT, PROT_READ,
    PROT_WRITE, S_IRUSR, S_IWUSR, S_IXUSR,
};

use named_sem::NamedSemaphore;
use rayon::ThreadPoolBuilder;
use zisk_common::{ChunkId, EmuTrace};

use std::ffi::{c_uint, c_void, CString};
use std::fmt::Debug;
use std::path::Path;
use std::process::{self, Command};
use std::sync::mpsc;
use std::time::Duration;
use std::{fs, ptr};

use log::{error, info};

use crate::{AsmInputC, AsmMTChunk, AsmMTHeader, AsmRunnerOptions, AsmRunnerTraceLevel};

pub trait Task: Send + Sync + 'static {
    type Output: Send + 'static;
    fn execute(&self) -> Self::Output;
}

pub type TaskFactory<'a, T> = Box<dyn Fn(ChunkId, EmuTrace) -> T + Send + Sync + 'a>;

#[derive(Debug)]
pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    AsmEmuTrace(AsmRunnerMT),
}

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

        let shmem_prefix = format!("ZISKMT{}", pid);
        let shmem_input_name = format!("/{}_input", shmem_prefix);
        let shmem_output_name = format!("/{}_output", shmem_prefix);

        // Build semaphores names, and create them (if they do not already exist)
        let sem_output_name = format!("/{}_semout", shmem_prefix);
        let sem_input_name = format!("/{}_semin", shmem_prefix);

        let mut sem_in = NamedSemaphore::create(sem_input_name.clone(), 0).unwrap_or_else(|e| {
            panic!(
                "AsmRunnerMT::run() failed calling NamedSemaphore::create({}), error: {}",
                sem_input_name, e
            )
        });

        let mut sem_out = NamedSemaphore::create(sem_output_name.clone(), 0).unwrap_or_else(|e| {
            panic!(
                "AsmRunnerMT::run() failed calling NamedSemaphore::create({}), error: {}",
                sem_output_name, e
            )
        });

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
        if let Err(e) = command.arg(&shmem_prefix).spawn() {
            error!("Child process failed: {:?}", e);
        } else if options.verbose || options.log_output {
            info!("Child process launched successfully");
        }

        // Wait for the assembly emulator to complete writing the trace
        if let Err(e) = sem_in.wait() {
            panic!(
                "AsmRunnerMT::run() failed calling semin.wait({}), error: {}",
                sem_input_name, e
            );
        }

        let stop = start.elapsed();

        let (mapped_ptr, vec_chunks) = Self::map_output(shmem_output_name.clone());

        let total_steps = vec_chunks.iter().map(|x| x.steps).sum::<u64>();
        let mhz = (total_steps as f64 / stop.as_secs_f64()) / 1_000_000.0;
        info!("AsmRnner: ··· Assembly execution speed: {:.2} MHz", mhz);

        // Tell the assembly that we are done reading the trace
        if let Err(e) = sem_out.post() {
            panic!(
                "AsmRunnerMT::run() failed calling semout.post({}), error: {}",
                sem_output_name, e
            );
        }

        AsmRunnerMT::new(shmem_output_name, mapped_ptr, vec_chunks)
    }

    pub fn run_and_count<T: Task>(
        ziskemuasm_path: &Path,
        inputs_path: &Path,
        shm_size: u64,
        chunk_size: u64,
        options: AsmRunnerOptions,
        task_factory: TaskFactory<T>,
    ) -> (AsmRunnerMT, Vec<T::Output>) {
        let pid = unsafe { libc::getpid() };

        let shmem_prefix = format!("ZISKMT{}", pid);
        let shmem_input_name = format!("/{}_input", shmem_prefix);
        let shmem_output_name = format!("/{}_output", shmem_prefix);

        // Build semaphores names, and create them (if they do not already exist)
        let sem_output_name = format!("/{}_semout", shmem_prefix);
        let sem_input_name = format!("/{}_semin", shmem_prefix);
        let sem_chunk_done_name = format!("/{}_semckd", shmem_prefix);

        let mut sem_in = NamedSemaphore::create(sem_input_name.clone(), 0).unwrap_or_else(|e| {
            panic!(
                "AsmRunnerMT::run() failed calling NamedSemaphore::create({}), error: {}",
                sem_input_name, e
            )
        });

        let mut sem_out = NamedSemaphore::create(sem_output_name.clone(), 0).unwrap_or_else(|e| {
            panic!(
                "AsmRunnerMT::run() failed calling NamedSemaphore::create({}), error: {}",
                sem_input_name, e
            )
        });

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .unwrap_or_else(|e| {
                panic!(
                    "AsmRunnerMT::run() failed calling NamedSemaphore::create({}), error: {}",
                    sem_input_name, e
                )
            });

        Self::write_input(inputs_path, &shmem_input_name, shm_size, chunk_size);

        // Prepare command
        let mut command = Command::new(ziskemuasm_path);

        command.arg("--generate_minimal_trace");

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

        let start = std::time::Instant::now();
        if let Err(e) = command.arg(&shmem_prefix).spawn() {
            error!("Child process failed: {:?}", e);
        } else if options.verbose || options.log_output {
            info!("Child process launched successfully");
        }

        let pool = ThreadPoolBuilder::new().num_threads(16).build().unwrap();

        let mut chunk_id = ChunkId(0);
        let mut header_ptr: Option<*mut c_void> = None;
        let mut data_ptr: Option<*mut c_void> = None;

        let mut should_exit = false;

        let (sender, receiver) = mpsc::channel();

        let exit_code = loop {
            match sem_chunk_done.timed_wait(Duration::from_millis(10000)) {
                Ok(()) => {
                    // Read the header data
                    if data_ptr.is_none() {
                        header_ptr.get_or_insert_with(|| Self::get_output_ptr(&shmem_output_name));

                        let output_header_size = std::mem::size_of::<AsmMTHeader>();

                        data_ptr = Some(unsafe {
                            (header_ptr.unwrap() as *mut u8).add(output_header_size + 8)
                                as *mut c_void
                        });
                    }

                    let emu_trace = AsmMTChunk::to_emu_trace(data_ptr.as_mut().unwrap());

                    let task = task_factory(chunk_id, emu_trace);
                    let sender = sender.clone();
                    pool.spawn(move || {
                        sender.send(task.execute()).unwrap();
                    });

                    chunk_id.0 += 1;

                    // Check exit_code after processing the chunk
                    if !should_exit {
                        let header = Self::read_output_header(&mut header_ptr, &shmem_output_name);
                        should_exit = header.exit_code == 0;
                    }
                }
                Err(e) => {
                    error!("Semaphore sem_chunk_done error: {:?}", e);

                    if chunk_id.0 == 0 {
                        break 1;
                    }

                    let output_header =
                        Self::read_output_header(&mut header_ptr, &shmem_output_name);
                    break output_header.exit_code;
                }
            }

            if should_exit {
                while let Ok(()) = sem_chunk_done.try_wait() {
                    let emu_trace = AsmMTChunk::to_emu_trace(data_ptr.as_mut().unwrap());
                    let task = task_factory(chunk_id, emu_trace);
                    let sender = sender.clone();
                    pool.spawn(move || {
                        sender.send(task.execute()).unwrap();
                    });

                    chunk_id.0 += 1;
                }

                break 0;
            }
        };

        if exit_code != 0 {
            panic!("Child process terminated with error code: {}", exit_code,);
        }

        // Collect results
        drop(sender);
        let tasks: Vec<T::Output> = receiver.iter().collect();

        // Wait for the assembly emulator to complete writing the trace
        let result = sem_in.wait();
        if result.is_err() {
            panic!("AsmRunnerMT::run() failed calling semin.wait({})", sem_input_name);
        }

        let stop = start.elapsed();

        let (mapped_ptr, vec_chunks) = Self::map_output(shmem_output_name.clone());

        let total_steps = vec_chunks.iter().map(|x| x.steps).sum::<u64>();
        let mhz = (total_steps as f64 / stop.as_secs_f64()) / 1_000_000.0;
        info!("AsmRnner: ··· Assembly execution speed: {:.2} MHz", mhz);

        // Tell the assembly that we are done reading the trace
        let result = sem_out.post();
        if result.is_err() {
            panic!("AsmRunnerMT::run() failed calling semout.post({})", sem_output_name);
        }

        (AsmRunnerMT::new(shmem_output_name, mapped_ptr, vec_chunks), tasks)
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
            shm_open(
                shmem_input_name_ptr,
                libc::O_RDWR | O_CREAT,
                (S_IRUSR | S_IWUSR | S_IXUSR) as c_uint,
            )
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

        let shm_fd = unsafe {
            shm_open(shmem_output_name_ptr, libc::O_RDONLY, (S_IRUSR | S_IWUSR | S_IXUSR) as c_uint)
        };

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

    fn read_output_header(
        header_ptr: &mut Option<*mut c_void>,
        shmem_output_name: &str,
    ) -> AsmMTHeader {
        header_ptr.get_or_insert_with(|| {
            let cstr = CString::new(shmem_output_name).expect("CString::new failed");
            let ptr = cstr.as_ptr();

            // Open shared memory read-only
            let shm_fd = unsafe { shm_open(ptr, libc::O_RDONLY, S_IRUSR | S_IWUSR | S_IXUSR) };
            Self::check_shm_open(shm_fd, ptr);

            let header_size = size_of::<AsmMTHeader>();

            // Map the header from the shared memory
            let mapped =
                unsafe { mmap(ptr::null_mut(), header_size, PROT_READ, MAP_SHARED, shm_fd, 0) };
            Self::check_mmap(mapped, header_size, file!(), line!());

            mapped
        });
        unsafe { std::ptr::read(header_ptr.unwrap() as *const AsmMTHeader) }
    }

    fn get_output_ptr(shmem_output_name: &str) -> *mut c_void {
        let cstr = CString::new(shmem_output_name).expect("CString::new failed");
        let ptr = cstr.as_ptr();

        // Open shared memory read-only
        let shm_fd = unsafe { shm_open(ptr, libc::O_RDONLY, S_IRUSR | S_IWUSR | S_IXUSR) };
        Self::check_shm_open(shm_fd, ptr);

        let header_size = size_of::<AsmMTHeader>();

        // Map the header from the shared memory
        let mapped =
            unsafe { mmap(ptr::null_mut(), header_size, PROT_READ, MAP_SHARED, shm_fd, 0) };
        Self::check_mmap(mapped, header_size, file!(), line!());

        // Read the header
        let header = unsafe { ptr::read(mapped as *const AsmMTHeader) };

        // Unmap the small mapping
        unsafe {
            munmap(mapped, header_size);
        }

        // Step 4: Map the full allocated size
        let mapped = unsafe {
            mmap(
                ptr::null_mut(),
                header.mt_allocated_size as usize,
                PROT_READ,
                MAP_SHARED,
                shm_fd,
                0,
            )
        };
        Self::check_mmap(mapped, header.mt_allocated_size as usize, file!(), line!());

        mapped
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
