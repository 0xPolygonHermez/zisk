use crate::{shmem_control_writer_name, AsmServices, SharedMemoryWriter};

use anyhow::Result;

pub struct ControlShmem {
    writers: Vec<SharedMemoryWriter>,
}

/// Byte offsets into the C-side `shmem_control_input_address` array.
/// Each slot is a `u64`; the C wait functions read these on every iteration.
#[derive(Copy, Clone)]
enum ControlShmemOffsets {
    PrecompilesSize = 0,
    ShutdownFlag = 8,
    InputsSize = 16,
    ResetFlag = 24,
}

impl ControlShmem {
    pub const CONTROL_WRITER_SIZE: u64 = 0x1000; // 4KB

    pub fn new(shm_prefix: &str, unlock_mapped_memory: bool) -> Result<Self> {
        let writers = AsmServices::SERVICES
            .iter()
            .map(|service| {
                let name = shmem_control_writer_name(shm_prefix, *service);
                SharedMemoryWriter::new(
                    &name,
                    Self::CONTROL_WRITER_SIZE as usize,
                    unlock_mapped_memory,
                )
                .map_err(anyhow::Error::from)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { writers })
    }

    pub fn reset(&self) {
        for writer in &self.writers {
            writer.write_u64_at(ControlShmemOffsets::PrecompilesSize as usize, 0);
            writer.write_u64_at(ControlShmemOffsets::ShutdownFlag as usize, 0);
            writer.write_u64_at(ControlShmemOffsets::InputsSize as usize, 0);
            writer.write_u64_at(ControlShmemOffsets::ResetFlag as usize, 0);
        }
    }

    pub fn set_prec_hints_size(&self, size: u64) {
        for writer in &self.writers {
            writer.write_u64_at(ControlShmemOffsets::PrecompilesSize as usize, size);
        }
    }

    pub fn prec_hints_size(&self) -> u64 {
        self.writers[0].read_u64_at(ControlShmemOffsets::PrecompilesSize as usize)
    }

    pub(crate) fn set_reset_flag(&self) {
        for writer in &self.writers {
            writer.write_u64_at(ControlShmemOffsets::ResetFlag as usize, 1);
        }
    }

    pub fn inc_inputs_size(&self, size: usize) {
        let current_size = self.writers[0].read_u64_at(ControlShmemOffsets::InputsSize as usize);
        let new_size = current_size + size as u64;
        for writer in &self.writers {
            writer.write_u64_at(ControlShmemOffsets::InputsSize as usize, new_size);
        }
    }
}
