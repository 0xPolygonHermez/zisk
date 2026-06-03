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
use std::sync::{
    atomic::{fence, AtomicUsize, Ordering},
    Arc, Mutex,
};
use tracing::debug;
use zisk_common::io::StreamSink;

/// Per-service control-output shmem (the C side's
/// `precompile_read_address`). Read by the parent for flow control in
/// `submit` (slowest-consumer wait); the C side resets it to 0 itself
/// in `server_reset_fast()` after every emulation.
struct SeparateShm {
    control_reader: SharedMemoryReader,
}

// SAFETY: serialised by the enclosing `Mutex<Vec<SeparateShm>>`.
unsafe impl Send for SeparateShm {}
unsafe impl Sync for SeparateShm {}

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

// SAFETY: POSIX named semaphores are thread- and process-safe by spec.
unsafe impl Send for SeparateSem {}
unsafe impl Sync for SeparateSem {}

/// Unified resources shared across all asm services.
struct UnifiedResources {
    /// Control shared memory writer (single write_pos)
    control_writer: Arc<ControlShmem>,
    /// One data writer per service — each C service has its own precompile shmem segment,
    /// so Rust writes the same hint data to all of them to keep them in sync.
    data_writer: SharedMemoryWriter,
}

// SAFETY: writes are serialized by the enclosing `Mutex<UnifiedResources>`.
unsafe impl Send for UnifiedResources {}
unsafe impl Sync for UnifiedResources {}

/// HintsShmem struct manages the writing of processed precompile hints to shared memory.
pub struct HintsShmem {
    /// Number of active ASM services to notify on submit.
    active_count: AtomicUsize,
    /// Unified resources (single data buffer and control writer)
    unified: Mutex<UnifiedResources>,
    /// Per-service shmem.
    separate_shm: Mutex<Vec<SeparateShm>>,
    /// Per-program semaphores.
    separate_sem: Mutex<Option<Vec<SeparateSem>>>,
}

impl HintsShmem {
    const CONTROL_PRECOMPILE_SIZE: u64 = 0x1000; // 4KB
    const MAX_PRECOMPILE_SIZE: u64 = 0x8000000; // 128MB
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
            unified: Mutex::new(unified),
            separate_shm: Mutex::new(separate_shm),
            separate_sem: Mutex::new(None),
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
        *self.separate_sem.lock().expect("separate_sem mutex poisoned") = Some(sems);
        Ok(())
    }

    /// Drop the semaphore handles (does not unlink — the binary owns the names).
    pub fn unbind_semaphores(&self) {
        *self.separate_sem.lock().expect("separate_sem mutex poisoned") = None;
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

        let name = shmem_precompile_name(shm_prefix);
        let data_writer = SharedMemoryWriter::new(
            &name,
            Self::MAX_PRECOMPILE_SIZE as usize,
            unlock_mapped_memory,
        )?;

        Ok(UnifiedResources { control_writer, data_writer })
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

        let mut unified = self.unified.lock().expect("unified mutex poisoned");
        let separate_shm = self.separate_shm.lock().expect("separate_shm mutex poisoned");
        let mut separate_sem_guard = self.separate_sem.lock().expect("separate_sem mutex poisoned");
        debug_assert!(separate_sem_guard.is_some(), "submit called before bind_semaphores");

        let active = self.active_count.load(Ordering::SeqCst);

        let Some(separate_sem) = separate_sem_guard.as_mut() else {
            return Ok(());
        };

        // Read current write position once
        let write_pos = unified.control_writer.prec_hints_size();

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
        unified.data_writer.write_ring_buffer(processed)?;

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
        let mut unified = self.unified.lock().expect("unified mutex poisoned");
        unified.control_writer.reset();
        unified.data_writer.reset();

        // Drain any leftover semaphore counts from the previous run.
        if let Some(sems) = self.separate_sem.lock().expect("separate_sem mutex poisoned").as_mut()
        {
            for res in sems.iter_mut() {
                while res.sem_available.try_wait().is_ok() {}
                while res.sem_read.try_wait().is_ok() {}
            }
        }
    }
}
