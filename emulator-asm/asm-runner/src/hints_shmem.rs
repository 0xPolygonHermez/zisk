//! HintsShmem is responsible for writing precompile processed hints to shared memory.
//!
//! It implements the HintsSink trait to receive processed hints and write them to shared memory
//! using SharedMemoryWriter instances.

use crate::{
    sem_available_name, sem_read_name, shmem_control_reader_name, shmem_precompile_name,
    AsmService, AsmServices, ControlShmem, SharedMemoryReader, SharedMemoryWriter,
};
use anyhow::Result;
use named_sem::NamedSemaphore;
use std::{
    cell::RefCell,
    sync::{
        atomic::{fence, AtomicUsize, Ordering},
        Arc,
    },
};
use tracing::debug;
use zisk_common::io::StreamSink;

/// Per-service shmem resources
struct SeparateShm {
    /// Consumer's read-position control shmem.
    control_reader: SharedMemoryReader,
}

impl SeparateShm {
    pub fn new(shm_prefix: &str, service: AsmService) -> Result<Self> {
        let name = shmem_control_reader_name(shm_prefix, service);
        Ok(Self {
            control_reader: SharedMemoryReader::new(
                &name,
                HintsShmem::CONTROL_PRECOMPILE_SIZE as usize,
            )?,
        })
    }
}

/// Per-service semaphore resources.
struct SeparateSem {
    /// Semaphore to signal data availability to this consumer
    sem_available: NamedSemaphore,
    /// Semaphore to wait for this consumer's data consumption
    sem_read: NamedSemaphore,
}

/// Unified resources shared across all asm services.
struct UnifiedResources {
    /// Control shared memory writer (single write_pos)
    control_writer: Arc<ControlShmem>,
    /// One data writer per service — each C service has its own precompile shmem segment,
    /// so Rust writes the same hint data to all of them to keep them in sync.
    data_writers: Vec<SharedMemoryWriter>,
}

/// HintsShmem struct manages the writing of processed precompile hints to shared memory.
pub struct HintsShmem {
    /// Number of active ASM services to notify on submit.
    active_count: AtomicUsize,
    /// Unified resources (single data buffer and control writer)
    unified: RefCell<UnifiedResources>,
    /// Per-service shmem.
    separate_shm: RefCell<Vec<SeparateShm>>,
    /// Per-program semaphores.
    separate_sem: RefCell<Option<Vec<SeparateSem>>>,
}

unsafe impl Send for HintsShmem {}
unsafe impl Sync for HintsShmem {}

impl HintsShmem {
    const CONTROL_PRECOMPILE_SIZE: u64 = 0x1000; // 4KB
    const MAX_PRECOMPILE_SIZE: u64 = 0x400000; // 4MB
    const BUFFER_CAPACITY_U64: u64 = Self::MAX_PRECOMPILE_SIZE >> 3;

    /// Map shmem segments. Semaphores are NOT opened here; call `bind_semaphores` before use.
    pub fn new(
        shm_prefix: &str,
        unlock_mapped_memory: bool,
        control_writer: Arc<ControlShmem>,
        active_services: &[AsmService],
    ) -> Result<Self> {
        // Create unified resources (single data buffer and control writer)
        let unified = Self::create_unified(shm_prefix, unlock_mapped_memory, control_writer)?;
        unified.control_writer.reset();

        // Create separate resources
        let separate_shm = AsmServices::SERVICES
            .iter()
            .map(|service| SeparateShm::new(shm_prefix, *service))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            unified: RefCell::new(unified),
            separate_shm: RefCell::new(separate_shm),
            separate_sem: RefCell::new(None),
            active_count: AtomicUsize::new(active_services.len()),
        })
    }

    /// Open per-service semaphores for the given program's `sem_prefix`.
    /// Replaces any previously bound semaphores.
    pub fn bind_semaphores(&self, sem_prefix: &str) -> Result<()> {
        let sems = AsmServices::SERVICES
            .iter()
            .map(|service| {
                let avail_name = sem_available_name(sem_prefix, *service);
                let read_name = sem_read_name(sem_prefix, *service);
                Ok(SeparateSem {
                    sem_available: NamedSemaphore::create(&avail_name, 0).map_err(|e| {
                        anyhow::anyhow!("Failed to create semaphore '{}': {}", avail_name, e)
                    })?,
                    sem_read: NamedSemaphore::create(&read_name, 0).map_err(|e| {
                        anyhow::anyhow!("Failed to create semaphore '{}': {}", read_name, e)
                    })?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        *self.separate_sem.borrow_mut() = Some(sems);
        Ok(())
    }

    /// Drop the semaphore handles (does not unlink — the binary owns the names).
    pub fn unbind_semaphores(&self) {
        *self.separate_sem.borrow_mut() = None;
    }

    /// Update the number of active ASM services notified on each submit.
    pub fn set_active_services(&self, services: &[AsmService]) -> Result<()> {
        if services.len() > AsmServices::SERVICES.len() {
            return Err(anyhow::anyhow!(
                "active_services count {} exceeds allocated separate resources {}",
                services.len(),
                AsmServices::SERVICES.len()
            ));
        }
        self.active_count.store(services.len(), Ordering::SeqCst);
        Ok(())
    }

    fn create_unified(
        shm_prefix: &str,
        unlock_mapped_memory: bool,
        control_writer: Arc<ControlShmem>,
    ) -> Result<UnifiedResources> {
        debug!("Initializing unified resources for precompile hints");
        let data_writers = AsmServices::SERVICES
            .iter()
            .map(|service| {
                let name = shmem_precompile_name(shm_prefix, *service);
                SharedMemoryWriter::new(
                    &name,
                    Self::MAX_PRECOMPILE_SIZE as usize,
                    unlock_mapped_memory,
                )
                .map_err(anyhow::Error::from)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(UnifiedResources { control_writer, data_writers })
    }
}

impl StreamSink for HintsShmem {
    /// Writes processed precompile hints to the shared memory.
    ///
    /// Data is written ONCE to the shared buffer, then all consumers are notified.
    /// Flow control waits for the slowest consumer.
    ///
    /// # Arguments
    /// * `processed` - A vector of processed precompile hints as u64 values.
    ///
    /// # Returns
    /// * `Ok(())` - If hints were successfully written to shared memory
    /// * `Err` - If writing to shared memory fails
    #[inline]
    fn submit(&self, processed: &[u64]) -> anyhow::Result<()> {
        let data_size = processed.len() as u64;

        if data_size == 0 {
            return Ok(());
        }

        // Validate data size fits in buffer
        if data_size > Self::BUFFER_CAPACITY_U64 {
            return Err(anyhow::anyhow!(
                "Processed data size ({} u64 elements) exceeds buffer capacity ({} u64 elements)",
                data_size,
                Self::BUFFER_CAPACITY_U64
            ));
        }

        let mut unified = self.unified.borrow_mut();
        let separate_shm = self.separate_shm.borrow();
        let mut separate_sem_guard = self.separate_sem.borrow_mut();
        debug_assert!(separate_sem_guard.is_some(), "submit called before bind_semaphores");

        let active = self.active_count.load(Ordering::SeqCst);

        let Some(separate_sem) = separate_sem_guard.as_mut() else {
            return Ok(());
        };

        // Read current write position once
        let write_pos = unified.control_writer.prec_hints_size();

        // (1) Capture per-consumer read positions BEFORE flow-control wait. If a job starts
        // with non-zero read positions (carried over from previous run), the math
        // `occupied_space = write_pos - min_read_pos` underflows or returns a wrong value,
        // and write_ring_buffer below scribbles in the wrong place → SIGSEGV in the C reader.
        let pre_submit_read_positions: Vec<u64> = separate_shm[0..active]
            .iter()
            .map(|res| res.control_reader.read_u64_at(0))
            .collect();
        tracing::debug!(
            "HintsShmem::submit: data_size={} write_pos={} read_positions={:?} active={}",
            data_size,
            write_pos,
            pre_submit_read_positions,
            active
        );
        if write_pos == 0 && pre_submit_read_positions.iter().any(|&p| p != 0) {
            tracing::error!(
                "HintsShmem::submit: STALE STATE write_pos=0 but read_positions={:?} \
                 (carried over from previous job — flow-control math will be wrong)",
                pre_submit_read_positions
            );
        }

        // Flow control: wait until all consumers have advanced enough
        // We need to wait for the slowest consumer (minimum read position)
        loop {
            // Ensure we observe the latest read positions
            fence(Ordering::Acquire);

            // Find the slowest consumer (minimum read position) and its index
            let (slowest_idx, min_read_pos) = separate_shm[0..active]
                .iter()
                .enumerate()
                .map(|(i, res)| (i, res.control_reader.read_u64_at(0)))
                .min_by_key(|(_, pos)| *pos)
                .unwrap();

            // Calculate occupied space based on slowest consumer (saturating to avoid underflow)
            debug_assert!(
                write_pos >= min_read_pos,
                "Write position ({}) is behind minimum read position ({})",
                write_pos,
                min_read_pos
            );
            let occupied_space = write_pos - min_read_pos;
            debug_assert!(
                occupied_space <= Self::BUFFER_CAPACITY_U64,
                "Occupied space ({}) exceeds buffer capacity ({})",
                occupied_space,
                Self::BUFFER_CAPACITY_U64
            );
            let available_space = Self::BUFFER_CAPACITY_U64 - occupied_space;

            // Flow control based on buffer occupancy
            if available_space >= data_size {
                break;
            }

            // Not enough space - wait for the SLOWEST consumer to signal progress
            // Retry on interrupt (EINTR)
            if separate_sem[slowest_idx].sem_read.wait().is_err() {
                continue;
            }
        }

        // Write data to each service's precompile buffer (same data, keeps all in sync)
        for writer in &mut unified.data_writers {
            writer.write_ring_buffer(processed)?;
        }

        fence(Ordering::Release);

        // Update write position ONCE in control memory
        unified.control_writer.set_prec_hints_size(write_pos + data_size);

        fence(Ordering::Release);

        // Notify ALL consumers that new data is available
        for res in &mut separate_sem[0..active] {
            res.sem_available.post()?;
        }

        Ok(())
    }

    fn reset(&self) {
        let mut unified = self.unified.borrow_mut();

        // (1) Pre-reset snapshot: control writer's write position and per-service C-side read positions.
        // If any read_pos != 0 here, the C side is starting the next job with stale state — likely cause
        // of the SIGSEGV when ring-buffer indexing relies on writer/reader positions both being 0.
        let pre_write_pos = unified.control_writer.prec_hints_size();
        let pre_read_positions: Vec<u64> = self
            .separate_shm
            .borrow()
            .iter()
            .map(|res| res.control_reader.read_u64_at(0))
            .collect();
        let pre_active = self.active_count.load(Ordering::SeqCst);
        tracing::info!(
            "HintsShmem::reset: pre  write_pos={} read_positions={:?} active_count={}",
            pre_write_pos,
            pre_read_positions,
            pre_active
        );

        unified.control_writer.reset();
        for (idx, writer) in unified.data_writers.iter_mut().enumerate() {
            // (2) The data_writers are SharedMemoryWriter ring buffers. `reset()` only rewinds
            // current_ptr to ptr — it does NOT zero memory or re-sync any C-side read position.
            // Log so we can correlate with crashes triggered after wraparound on the next stream.
            tracing::info!(
                "HintsShmem::reset: data_writer[{}] rewinding ring-buffer current_ptr -> ptr",
                idx
            );
            writer.reset();
        }

        // Drain stale semaphore signals from previous execution
        if let Some(separate_sem) = self.separate_sem.borrow_mut().as_mut() {
            for (idx, res) in separate_sem.iter_mut().enumerate() {
                let mut drained_avail = 0u32;
                while res.sem_available.try_wait().is_ok() {
                    drained_avail += 1;
                }
                let mut drained_read = 0u32;
                while res.sem_read.try_wait().is_ok() {
                    drained_read += 1;
                }
                if drained_avail > 0 || drained_read > 0 {
                    // (3) Stale semaphore posts are a smoking gun for incomplete teardown of the
                    // previous job. The C side may have posted sem_read after we already moved on,
                    // or sem_available may be lingering from a partially-consumed batch.
                    tracing::warn!(
                        "HintsShmem::reset: drained stale semaphores for service[{}] sem_available={} sem_read={}",
                        idx,
                        drained_avail,
                        drained_read
                    );
                }
            }
        }

        for (idx, res) in self.separate_shm.borrow().iter().enumerate() {
            let read_pos = res.control_reader.read_u64_at(0);
            if read_pos != 0 {
                // (3) Originally just a warn — promote to error so it's loud and add the writer's
                // post-reset write position so we can see whether reader is ahead of writer
                // (which would mean the next submit's flow-control math underflows).
                tracing::error!(
                    "HintsShmem::reset: control_reader[{}] read_pos={} != 0 after reset \
                     (post_write_pos={}). Previous emulation didn't complete cleanly — \
                     C-side will read garbage from offset {} on the next stream.",
                    idx,
                    read_pos,
                    unified.control_writer.prec_hints_size(),
                    read_pos
                );
            }
        }

        // (1) Post-reset snapshot.
        let post_write_pos = unified.control_writer.prec_hints_size();
        let post_read_positions: Vec<u64> = self
            .separate_shm
            .borrow()
            .iter()
            .map(|res| res.control_reader.read_u64_at(0))
            .collect();
        tracing::info!(
            "HintsShmem::reset: post write_pos={} read_positions={:?}",
            post_write_pos,
            post_read_positions
        );
    }
}
