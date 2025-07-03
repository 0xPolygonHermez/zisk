use libc::{close, PROT_READ, PROT_WRITE, S_IRUSR, S_IWUSR};

use named_sem::NamedSemaphore;
use rayon::ThreadPoolBuilder;
use zisk_common::{ChunkId, EmuTrace};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::path::Path;
use std::sync::atomic::{fence, Ordering};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::Mutex;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use std::{fs, ptr};

use tracing::{error, info};

use crate::{
    shmem_utils, AsmInputC2, AsmMTChunk, AsmMTHeader, AsmRunError, AsmService, AsmServices,
    AsmSharedMemory,
};

use anyhow::{Context, Result};

pub trait Task: Send + Sync + 'static {
    type Output: Send + 'static;
    fn execute(&self) -> Self::Output;
}

pub type TaskFactory<'a, T> = Box<dyn Fn(ChunkId, Arc<EmuTrace>) -> T + Send + Sync + 'a>;

pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    AsmEmuTrace(AsmRunnerMT),
}

// This struct is used to run the assembly code in a separate process and generate minimal traces.
pub struct AsmRunnerMT {
    pub vec_chunks: Vec<EmuTrace>,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Drop for AsmRunnerMT {
    fn drop(&mut self) {
        for chunk in &mut self.vec_chunks {
            // Ensure that the memory reads are not dropped when the chunk is dropped
            // This is necessary because the memory reads are stored in a Vec<u64> which is
            // allocated in the shared memory and we need to avoid double freeing it.
            std::mem::forget(std::mem::take(&mut chunk.mem_reads));
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl AsmRunnerMT {
    pub fn new(vec_chunks: Vec<EmuTrace>) -> Self {
        Self { vec_chunks }
    }

    pub fn create_shmem(
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<AsmSharedMemory<AsmMTHeader>> {
        AsmSharedMemory::create_shmem(AsmService::MT, local_rank, base_port, unlock_mapped_memory)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_and_count<T: Task>(
        asm_shared_memory: Arc<Mutex<Option<AsmSharedMemory<AsmMTHeader>>>>,
        inputs_path: &Path,
        max_steps: u64,
        chunk_size: u64,
        task_factory: TaskFactory<T>,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<(AsmRunnerMT, Vec<T::Output>)> {
        const MEM_READS_SIZE_DUMMY: u64 = 0xFFFFFFFFFFFFFFFF;

        let (shmem_input_name, _, sem_chunk_done_name) =
            AsmSharedMemory::<AsmMTHeader>::shmem_names(AsmService::MT, base_port, local_rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        Self::write_input(inputs_path, &shmem_input_name, unlock_mapped_memory);

        let start = Instant::now();

        let handle = std::thread::spawn(move || {
            let asm_services = AsmServices::new(world_rank, local_rank, base_port);
            asm_services.send_minimal_trace_request(max_steps, chunk_size)
        });

        // Initialize the assembly shared memory if necessary
        let mut asm_shared_memory = asm_shared_memory.lock().unwrap();

        if asm_shared_memory.is_none() {
            *asm_shared_memory = Some(
                AsmRunnerMT::create_shmem(local_rank, base_port, unlock_mapped_memory)
                    .expect("Error creating assembly shared memory"),
            );
        }

        let pool = ThreadPoolBuilder::new().num_threads(24).build().map_err(AsmRunError::from)?;
        let (sender, receiver) = mpsc::channel();

        let mut chunk_id = ChunkId(0);

        // Get the pointer to the data in the shared memory.
        let mut data_ptr = asm_shared_memory.as_ref().unwrap().data_ptr() as *const AsmMTChunk;

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
                    error!("Semaphore '{}' error: {:?}", sem_chunk_done_name, e);

                    if chunk_id.0 == 0 {
                        break 1;
                    }

                    break asm_shared_memory.as_ref().unwrap().header().exit_code;
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

        Ok((AsmRunnerMT::new(emu_traces), tasks))
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
            "MT input mmap",
        );
        unsafe {
            ptr::copy_nonoverlapping(full_input.as_ptr(), ptr as *mut u8, shmem_input_size);
            shmem_utils::unmap(ptr, shmem_input_size);
            close(fd);
        }
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
