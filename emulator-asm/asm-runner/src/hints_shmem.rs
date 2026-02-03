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
use std::cell::RefCell;
use tracing::debug;
use zisk_common::io::StreamSink;

/// Names for a service's shared memory and semaphore resources
struct ServiceResourceNames {
    control_writer: String,
    control_reader: String,
    data_name: String,
    sem_available_name: String,
    sem_read_name: String,
}

impl ServiceResourceNames {
    fn new(service: &AsmService, port: u16, local_rank: i32) -> Self {
        Self {
            control_writer: shmem_control_writer_name(port, *service, local_rank),
            control_reader: shmem_control_reader_name(port, *service, local_rank),
            data_name: shmem_precompile_name(port, *service, local_rank),
            sem_available_name: sem_available_name(port, *service, local_rank),
            sem_read_name: sem_read_name(port, *service, local_rank),
        }
    }
}

/// Represents a service's shared memory and synchronization resources
struct ServiceResources {
    /// Control shared memory writer
    control_writer: SharedMemoryWriter,
    /// Control shared memory reader
    control_reader: SharedMemoryReader,
    /// Data shared memory writer
    data_writer: SharedMemoryWriter,
    /// Semaphore to signal data availability
    sem_available: NamedSemaphore,
    /// Semaphore to wait for data consumption
    sem_read: NamedSemaphore,
}

/// HintsShmem struct manages the writing of processed precompile hints to shared memory.
pub struct HintsShmem {
    /// Service resources combining shared memory writers and semaphores
    resources: RefCell<Vec<ServiceResources>>,
}

unsafe impl Send for HintsShmem {}
unsafe impl Sync for HintsShmem {}

impl HintsShmem {
    const CONTROL_PRECOMPILE_SIZE: u64 = 0x1000; // 4KB
    const MAX_PRECOMPILE_SIZE: u64 = 0x10000000; // 256MB
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
        let resources_names = AsmServices::SERVICES
            .iter()
            .map(|service| {
                let port = if let Some(base_port) = base_port {
                    AsmServices::port_for(service, base_port, local_rank)
                } else {
                    AsmServices::default_port(service, local_rank)
                };
                ServiceResourceNames::new(service, port, local_rank)
            })
            .collect();

        let mut resources = Self::create_resources(resources_names, unlock_mapped_memory)?;

        for resource in resources.iter_mut() {
            resource.control_writer.write_u64_at(0, 0);
        }

        Ok(Self { resources: RefCell::new(resources) })
    }

    /// Initialize the shared memory writers for the pipeline.
    ///
    /// This method creates SharedMemoryWriter instances for each shared memory name.
    /// If writers are already initialized it logs a warning and does nothing.
    fn create_resources(
        resources_names: Vec<ServiceResourceNames>,
        unlock_mapped_memory: bool,
    ) -> Result<Vec<ServiceResources>> {
        debug!("Initializing resources for precompile hints");
        resources_names
            .iter()
            .map(|names: &ServiceResourceNames| -> Result<ServiceResources> {
                Ok(ServiceResources {
                    control_writer: SharedMemoryWriter::new(
                        &names.control_writer,
                        Self::CONTROL_PRECOMPILE_SIZE as usize,
                        unlock_mapped_memory,
                    )?,
                    control_reader: SharedMemoryReader::new(
                        &names.control_reader,
                        Self::CONTROL_PRECOMPILE_SIZE as usize,
                    )?,
                    data_writer: SharedMemoryWriter::new(
                        &names.data_name,
                        Self::MAX_PRECOMPILE_SIZE as usize,
                        unlock_mapped_memory,
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
    /// Writes processed precompile hints to all shared memory writers.
    ///
    /// # Arguments
    /// * `processed` - A vector of processed precompile hints as u64 values.
    ///
    /// # Returns
    /// * `Ok(())` - If hints were successfully written to all shared memories
    /// * `Err` - If writing to any shared memory fails
    #[inline]
    fn submit(&self, processed: Vec<u64>) -> anyhow::Result<()> {
        let data_size = processed.len() as u64;

        debug_assert!(
            data_size <= Self::BUFFER_CAPACITY_U64,
            "Processed data size ({} u64 elements) exceeds maximum precompile shared memory capacity ({} u64 elements)",
            data_size,
            Self::BUFFER_CAPACITY_U64
        );

        let mut resources = self.resources.borrow_mut();

        for resource in resources.iter_mut() {
            // Read current write position once (we're the only writer)
            let write_pos = resource.control_writer.read_u64_at(0);

            loop {
                // Read current read position (updated by reader)
                let read_pos = resource.control_reader.read_u64_at(0);

                // Calculate occupied space in ring buffer (positions are absolute values in u64 elements)
                let occupied_space = write_pos - read_pos;
                let available_space = Self::BUFFER_CAPACITY_U64 - occupied_space;

                // Flow control based on buffer occupancy
                if available_space >= data_size {
                    break;
                }

                // Not enough space - wait for consumption
                resource.sem_read.wait()?;
            }

            // Write data to shared memory with automatic wraparound
            resource.data_writer.write_ring_buffer(&processed);

            // Update write position in control memory (absolute position, always increases)
            resource.control_writer.write_u64_at(0, write_pos + data_size);

            resource.sem_available.post()?;
        }

        Ok(())
    }
}
