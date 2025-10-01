use named_sem::NamedSemaphore;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use zisk_common::ExecutorStats;
use zisk_common::{ChunkId, EmuTrace};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::atomic::{fence, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::{error, info};

use crate::{AsmMTChunk, AsmMTHeader, AsmRunError, AsmService, AsmServices, AsmSharedMemory};

use anyhow::{Context, Result};

#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;

pub trait Task: Send + Sync + 'static {
    type Output: Send + 'static;
    fn execute(self, exit: bool) -> Self::Output;
}

pub type TaskFactory<'a, T> = Box<dyn Fn(ChunkId, EmuTrace) -> T + Send + Sync + 'a>;

pub const MAX_CHUNKS: usize = 1 << 16;

pub struct PreloadedMT {
    pub output_shmem: AsmSharedMemory<AsmMTHeader>,
}

impl PreloadedMT {
    pub fn new(
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        let port = if let Some(base_port) = base_port {
            AsmServices::port_for(&AsmService::MT, base_port, local_rank)
        } else {
            AsmServices::default_port(&AsmService::MT, local_rank)
        };

        let output_name =
            AsmSharedMemory::<AsmMTHeader>::shmem_output_name(port, AsmService::MT, local_rank);

        let output_shared_memory =
            AsmSharedMemory::<AsmMTHeader>::open_and_map(&output_name, unlock_mapped_memory)?;

        Ok(Self { output_shmem: output_shared_memory })
    }
}

// This struct is used to run the assembly code in a separate process and generate minimal traces.
pub struct AsmRunnerMT {
    pub vec_chunks: Vec<EmuTrace>,
}

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

impl AsmRunnerMT {
    pub fn new(vec_chunks: Vec<EmuTrace>) -> Self {
        Self { vec_chunks }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_and_count<T: Task>(
        preloaded: &mut PreloadedMT,
        max_steps: u64,
        chunk_size: u64,
        task_factory: TaskFactory<T>,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        _stats: Arc<Mutex<ExecutorStats>>,
    ) -> Result<Vec<T::Output>> {
        let __stats = Arc::clone(&_stats);

        #[cfg(feature = "stats")]
        let parent_stats_id = __stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        _stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "ASM_MT_RUNNER",
            0,
            ExecutorStatsEvent::Begin,
        );

        let port = if let Some(base_port) = base_port {
            AsmServices::port_for(&AsmService::MT, base_port, local_rank)
        } else {
            AsmServices::default_port(&AsmService::MT, local_rank)
        };

        let sem_chunk_done_name =
            AsmSharedMemory::<AsmMTHeader>::shmem_chunk_done_name(port, AsmService::MT, local_rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        let start_time = Instant::now();

        let handle = std::thread::spawn(move || {
            let asm_services = AsmServices::new(world_rank, local_rank, base_port);

            #[cfg(feature = "stats")]
            let stats_id = __stats.lock().unwrap().get_id();
            #[cfg(feature = "stats")]
            __stats.lock().unwrap().add_stat(
                parent_stats_id,
                stats_id,
                "ASM_MT",
                0,
                ExecutorStatsEvent::Begin,
            );

            let result = asm_services.send_minimal_trace_request(max_steps, chunk_size);

            #[cfg(feature = "stats")]
            __stats.lock().unwrap().add_stat(
                parent_stats_id,
                stats_id,
                "ASM_MT",
                0,
                ExecutorStatsEvent::End,
            );

            result
        });

        let mut chunk_id = ChunkId(0);

        // Get the pointer to the data in the shared memory.
        let mut data_ptr = preloaded.output_shmem.data_ptr() as *const AsmMTChunk;

        let mut total_steps = 0;
        let mut handles = Vec::new();

        let __stats = Arc::clone(&_stats);

        let exit_code = loop {
            match sem_chunk_done.timed_wait(Duration::from_secs(10)) {
                Ok(()) => {
                    #[cfg(feature = "stats")]
                    {
                        let mut stats_guard = __stats.lock().unwrap();
                        let stats_id = stats_guard.get_id();
                        stats_guard.add_stat(
                            parent_stats_id,
                            stats_id,
                            "MT_CHUNK_DONE",
                            0,
                            ExecutorStatsEvent::Mark,
                        );
                    }

                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);

                    let emu_trace = AsmMTChunk::to_emu_trace(&mut data_ptr);
                    let should_exit = emu_trace.end;
                    total_steps += emu_trace.steps;

                    let task = task_factory(chunk_id, emu_trace);

                    handles.push(std::thread::spawn(move || task.execute(should_exit)));

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

                    break preloaded.output_shmem.map_header().exit_code;
                }
            }
        };

        if exit_code != 0 {
            return Err(AsmRunError::ExitCode(exit_code as u32))
                .context("Child process returned error");
        }

        // Collect results
        let mut tasks = Vec::new();
        for handle in handles {
            tasks.push(handle.join().expect("Task panicked"));
        }

        let mhz = (total_steps as f64 / start_time.elapsed().as_secs_f64()) / 1_000_000.0;
        info!("··· Assembly execution speed: {:.2} MHz", mhz);

        // Wait for the assembly emulator to complete writing the trace
        let response = handle
            .join()
            .map_err(|_| AsmRunError::JoinPanic)?
            .map_err(AsmRunError::ServiceError)?;

        assert_eq!(response.result, 0);
        assert!(response.trace_len > 0);
        assert!(response.trace_len <= response.allocated_len);

        #[cfg(feature = "stats")]
        _stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "ASM_MT_RUNNER",
            0,
            ExecutorStatsEvent::End,
        );

        Ok(tasks)
    }
}
