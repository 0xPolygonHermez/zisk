use libc::{close, shm_unlink, PROT_READ, PROT_WRITE, S_IRUSR, S_IWUSR, S_IXUSR};

use named_sem::NamedSemaphore;
use rayon::ThreadPoolBuilder;
use zisk_common::{ChunkId, EmuTrace};

use std::ffi::c_void;
use std::fmt::Debug;
use std::path::Path;
use std::sync::atomic::{fence, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use std::{fs, ptr};

use tracing::{error, info};

use crate::{shmem_utils, AsmInputC2, AsmMTChunk, AsmMTHeader, AsmRunError, AsmServices};

use anyhow::{Context, Result};

pub trait Task: Send + Sync + 'static {
    type Output: Send + 'static;
    fn execute(&self) -> Self::Output;
}

pub type TaskFactory<'a, T> = Box<dyn Fn(ChunkId, Arc<EmuTrace>) -> T + Send + Sync + 'a>;

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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Drop for AsmRunnerMT {
    fn drop(&mut self) {
        for chunk in &mut self.vec_chunks {
            std::mem::forget(std::mem::take(&mut chunk.mem_reads));
        }
        let total_size = self.total_size();
        unsafe {
            shmem_utils::unmap(self.mapped_ptr, total_size);
        }
        let c_name =
            std::ffi::CString::new(self.shmem_output_name.clone()).expect("CString::new failed");
        unsafe {
            if shm_unlink(c_name.as_ptr()) != 0 {
                error!("shm_unlink failed: {:?}", std::io::Error::last_os_error());
            }
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl AsmRunnerMT {
    pub fn new(
        shmem_output_name: String,
        mapped_ptr: *mut std::ffi::c_void,
        vec_chunks: Vec<EmuTrace>,
    ) -> Self {
        Self { shmem_output_name, mapped_ptr, vec_chunks }
    }

    pub fn total_size(&self) -> usize {
        self.vec_chunks.iter().map(|chunk| chunk.mem_reads.len() * size_of::<u64>()).sum::<usize>()
            + size_of::<AsmMTHeader>()
    }

    pub fn run_and_count<T: Task>(
        inputs_path: &Path,
        max_steps: u64,
        chunk_size: u64,
        task_factory: TaskFactory<T>,
        rank: i32,
    ) -> Result<(AsmRunnerMT, Vec<T::Output>)> {
        const MEM_READS_SIZE_DUMMY: u64 = 0xFFFFFFFFFFFFFFFF;

        let shmem_input_name = format!("ZISK_{}_MT_input", rank);
        let shmem_output_name = format!("ZISK_{}_MT_output", rank);
        let sem_chunk_done_name = format!("/ZISK_{}_MT_chunk_done", rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name, e))?;

        Self::write_input(inputs_path, &shmem_input_name);

        let start = Instant::now();

        let handle = std::thread::spawn(move || {
            AsmServices::send_minimal_trace_request(max_steps, chunk_size)
        });

        let pool = ThreadPoolBuilder::new().num_threads(24).build().map_err(AsmRunError::from)?;
        let (sender, receiver) = mpsc::channel();

        let mut chunk_id = ChunkId(0);

        // Read the header data
        let header_ptr = Self::get_output_ptr(&shmem_output_name) as *const AsmMTHeader;

        // Skips the header size to get the data pointer.
        let data_ptr = unsafe { header_ptr.add(1) } as *const u64;
        // Skips the first u64 which is used for mem_reads_size.
        let mut data_ptr = unsafe { data_ptr.add(1) as *const AsmMTChunk };

        let mut emu_traces = Vec::new();
        let exit_code = loop {
            match sem_chunk_done.timed_wait(Duration::from_secs(10)) {
                Ok(()) => {
                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);

                    let chunk = unsafe { std::ptr::read(data_ptr) };

                    // TODO! Remove this check in the near future
                    if chunk.mem_reads_size == MEM_READS_SIZE_DUMMY {
                        panic!("Unexpected state: invalid data received from C++");
                    }

                    let emu_trace = AsmMTChunk::to_emu_trace(&mut data_ptr);

                    let should_exit = emu_trace.end;
                    let emu_trace = Arc::new(emu_trace);
                    let task = task_factory(chunk_id, emu_trace.clone());
                    emu_traces.push(emu_trace);

                    let sender = sender.clone();
                    pool.spawn(move || {
                        sender.send(task.execute()).unwrap();
                    });

                    if should_exit {
                        break 0;
                    }
                    chunk_id.0 += 1;
                }
                Err(e) => {
                    error!("Semaphore sem_chunk_done error: {:?}", e);

                    if chunk_id.0 == 0 {
                        break 1;
                    }

                    let header = unsafe { std::ptr::read(header_ptr) };
                    break header.exit_code;
                }
            }
        };

        if exit_code != 0 {
            return Err(AsmRunError::ExitCode(exit_code as u32))
                .context("Child process returned error");
        }

        // Collect results
        drop(sender);
        let tasks: Vec<T::Output> = receiver.iter().collect();

        let total_steps = emu_traces.iter().map(|x| x.steps).sum::<u64>();
        let mhz = (total_steps as f64 / start.elapsed().as_secs_f64()) / 1_000_000.0;
        info!("··· Assembly execution speed: {:.2} MHz", mhz);

        // Wait for the assembly emulator to complete writing the trace
        let response = handle
            .join()
            .map_err(|_| AsmRunError::JoinPanic)?
            .map_err(AsmRunError::ServiceError)?;

        assert_eq!(response.result, 0);
        assert!(response.trace_len > 0);
        assert!(response.trace_len <= response.allocated_len);

        // Unwrap the Arc pointers
        let emu_traces: Vec<EmuTrace> = emu_traces
            .into_iter()
            .map(|arc| Arc::try_unwrap(arc).map_err(|_| AsmRunError::ArcUnwrap))
            .collect::<std::result::Result<_, _>>()?;

        Ok((
            AsmRunnerMT::new(shmem_output_name.to_string(), header_ptr as *mut c_void, emu_traces),
            tasks,
        ))
    }

    pub fn write_input(inputs_path: &Path, shmem_input_name: &str) {
        let inputs = fs::read(inputs_path).expect("Failed to read input file");
        let asm_input = AsmInputC2 { zero: 0, input_data_size: inputs.len() as u64 };
        let shmem_input_size = (inputs.len() + size_of::<AsmInputC2>() + 7) & !7;

        let mut full_input = Vec::with_capacity(shmem_input_size);
        full_input.extend_from_slice(&asm_input.to_bytes());
        full_input.extend_from_slice(&inputs);

        let fd =
            shmem_utils::open_shmem(shmem_input_name, libc::O_RDWR, S_IRUSR | S_IWUSR | S_IXUSR);
        let ptr = shmem_utils::map(fd, shmem_input_size, PROT_READ | PROT_WRITE, "input mmap");
        unsafe {
            ptr::copy_nonoverlapping(full_input.as_ptr(), ptr as *mut u8, shmem_input_size);
            shmem_utils::unmap(ptr, shmem_input_size);
            close(fd);
        }
    }

    pub fn get_output_ptr(shmem_output_name: &str) -> *mut std::ffi::c_void {
        let fd =
            shmem_utils::open_shmem(shmem_output_name, libc::O_RDONLY, S_IRUSR | S_IWUSR | S_IXUSR);
        let header_size = size_of::<AsmMTHeader>();
        let temp = shmem_utils::map(fd, header_size, PROT_READ, "header temp map");
        let header = unsafe { (temp as *const AsmMTHeader).read() };
        unsafe {
            shmem_utils::unmap(temp, header_size);
        }
        shmem_utils::map(fd, header.mt_allocated_size as usize, PROT_READ, "output full map")
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
impl AsmRunnerMT {
    pub fn new(_: String, _: *mut c_void, _: Vec<EmuTrace>) -> Self {
        panic!(
            "AsmRunnerMT::new() is not supported on this platform. Only Linux x86_64 is supported."
        )
    }

    pub fn run_and_count<T: Task>(
        _: &Path,
        _: &Path,
        _: u64,
        _: u64,
        _: AsmRunnerOptions,
        _: TaskFactory<T>,
    ) -> (AsmRunnerMT, Vec<T::Output>) {
        panic!("AsmRunnerMT::run_and_count() is not supported on this platform. Only Linux x86_64 is supported.")
    }
}
