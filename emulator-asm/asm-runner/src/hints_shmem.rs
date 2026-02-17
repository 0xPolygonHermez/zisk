//! HintsShmem is responsible for writing precompile processed hints to shared memory.
//!
//! It implements the HintsSink trait to receive processed hints and write them to shared memory
//! using SharedMemoryWriter instances.

use crate::{
    sem_available_name, sem_read_name, shmem_control_reader_name, shmem_control_writer_name,
    shmem_precompile_name, AsmService, AsmServices, SharedMemoryReader, SharedMemoryWriter,
};
use anyhow::Result;
use named_sem::NamedSemaphore;
use std::{cell::RefCell, sync::atomic::AtomicBool};
use tracing::debug;
use zisk_common::io::StreamSink;

/// Names for separate resources (per-service)
struct SeparateResourceNames {
    control_reader: String,
    sem_available_name: String,
    sem_read_name: String,
}

impl SeparateResourceNames {
    fn new(service: &AsmService, port: u16, local_rank: i32) -> Self {
        Self {
            control_reader: shmem_control_reader_name(port, *service, local_rank),
            sem_available_name: sem_available_name(port, *service, local_rank),
            sem_read_name: sem_read_name(port, *service, local_rank),
        }
    }
}

/// Separate resources, one per asm service
struct SeparateResources {
    /// Control shared memory reader (consumer's read position)
    control_reader: SharedMemoryReader,
    /// Semaphore to signal data availability to this consumer
    sem_available: NamedSemaphore,
    /// Semaphore to wait for this consumer's data consumption
    sem_read: NamedSemaphore,
}

/// Unified resources shared across all asm services
struct UnifiedResources {
    /// Control shared memory writer (single write_pos)
    control_writer: SharedMemoryWriter,
    /// Data shared memory writer (single data buffer)
    data_writer: SharedMemoryWriter,
}

/// HintsShmem struct manages the writing of processed precompile hints to shared memory.
pub struct HintsShmem {
    has_rom_sm: AtomicBool,
    /// Unified resources (single data buffer and control writer)
    unified: RefCell<UnifiedResources>,
    /// Separate resources (control_reader and semaphores for each service)
    separate: RefCell<Vec<SeparateResources>>,
}

unsafe impl Send for HintsShmem {}
unsafe impl Sync for HintsShmem {}

impl HintsShmem {
    const CONTROL_PRECOMPILE_SIZE: u64 = 0x1000; // 4KB
    const MAX_PRECOMPILE_SIZE: u64 = 0x400000; // 4MB
    const BUFFER_CAPACITY_U64: u64 = Self::MAX_PRECOMPILE_SIZE >> 3; // Capacity in u64 elements

    /// Create a new HintsShmem with the given shared memory names and unlock option.
    ///
    /// # Arguments
    /// * `base_port` - Optional base port for generating shared memory names.
    /// * `local_rank` - Local rank for generating shared memory names.
    /// * `unlock_mapped_memory` - Whether to unlock mapped memory after writing.
    ///
    /// # Returns
    /// A new `HintsShmem` instance with uninitialized writers.
    pub fn new(
        base_port: Option<u16>,
        local_rank: i32,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        // Use the first service's port for shared resources naming
        let first_service = &AsmServices::SERVICES[0];
        let shared_port = if let Some(base_port) = base_port {
            AsmServices::port_for(first_service, base_port, local_rank)
        } else {
            AsmServices::default_port(first_service, local_rank)
        };

        // Create unified resources (single data buffer and control writer)
        let unified =
            Self::create_unified_resources(shared_port, local_rank, unlock_mapped_memory)?;
        unified.control_writer.write_u64_at(0, 0);

        // Create separate resources
        let separate_names: Vec<SeparateResourceNames> = AsmServices::SERVICES
            .iter()
            .map(|service| {
                let port = AsmServices::port_base_for(base_port, local_rank);

                SeparateResourceNames::new(service, port, local_rank)
            })
            .collect();

        let separate = Self::create_separate_resources(separate_names)?;

        Ok(Self {
            unified: RefCell::new(unified),
            separate: RefCell::new(separate),
            has_rom_sm: AtomicBool::new(true),
        })
    }

    /// Create the unified resources (single data buffer and control writer).
    fn create_unified_resources(
        port: u16,
        local_rank: i32,
        unlock_mapped_memory: bool,
    ) -> Result<UnifiedResources> {
        debug!("Initializing unified resources for precompile hints");
        let control_name = shmem_control_writer_name(port, local_rank);
        let data_name = shmem_precompile_name(port, local_rank);

        Ok(UnifiedResources {
            control_writer: SharedMemoryWriter::new(
                &control_name,
                Self::CONTROL_PRECOMPILE_SIZE as usize,
                unlock_mapped_memory,
            )?,
            data_writer: SharedMemoryWriter::new(
                &data_name,
                Self::MAX_PRECOMPILE_SIZE as usize,
                unlock_mapped_memory,
            )?,
        })
    }

    /// Create separate resources (control_reader and semaphores for each service).
    fn create_separate_resources(
        separate_names: Vec<SeparateResourceNames>,
    ) -> Result<Vec<SeparateResources>> {
        debug!("Initializing separate resources for precompile hints");
        separate_names
            .iter()
            .map(|names: &SeparateResourceNames| -> Result<SeparateResources> {
                Ok(SeparateResources {
                    control_reader: SharedMemoryReader::new(
                        &names.control_reader,
                        Self::CONTROL_PRECOMPILE_SIZE as usize,
                    )?,
                    sem_available: NamedSemaphore::create(&names.sem_available_name, 0).map_err(
                        |e| {
                            anyhow::anyhow!(
                                "Failed to create semaphore '{}': {}",
                                names.sem_available_name,
                                e
                            )
                        },
                    )?,
                    sem_read: NamedSemaphore::create(&names.sem_read_name, 0).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to create semaphore '{}': {}",
                            names.sem_read_name,
                            e
                        )
                    })?,
                })
            })
            .collect()
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
    fn submit(&self, processed: Vec<u64>) -> anyhow::Result<()> {
        let data_size = processed.len() as u64;

        // Early return for empty data
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
        let mut separate = self.separate.borrow_mut();

        let separate = if self.has_rom_sm.load(std::sync::atomic::Ordering::SeqCst) {
            &mut separate
        } else {
            &mut separate[0..2]
        };

        // Read current write position once
        let write_pos = unified.control_writer.read_u64_at(0);

        // Flow control: wait until all consumers have advanced enough
        // We need to wait for the slowest consumer (minimum read position)
        loop {
            // Find the slowest consumer (minimum read position) and its index
            let (slowest_idx, min_read_pos) = separate
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
            if separate[slowest_idx].sem_read.wait().is_err() {
                continue;
            }
        }

        // Write data ONCE to the unified shared memory buffer
        unified.data_writer.write_ring_buffer(&processed)?;

        // Update write position ONCE in control memory
        unified.control_writer.write_u64_at(0, write_pos + data_size);

        // Notify ALL consumers that new data is available
        for res in separate.iter_mut() {
            res.sem_available.post()?;
        }

        Ok(())
    }

    fn reset(&self) {
        // Reset control writer and data writer to initial state for next stream
        let mut unified = self.unified.borrow_mut();
        unified.control_writer.write_u64_at(0, 0);
        unified.data_writer.reset();

        // Drain stale semaphore signals from previous execution
        let mut separate = self.separate.borrow_mut();
        for res in separate.iter_mut() {
            while res.sem_available.try_wait().is_ok() {}
            while res.sem_read.try_wait().is_ok() {}
        }
    }

    fn set_has_rom_sm(&self, has_rom_sm: bool) {
        self.has_rom_sm.store(has_rom_sm, std::sync::atomic::Ordering::SeqCst);
    }
}
