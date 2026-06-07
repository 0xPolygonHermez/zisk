use crate::{shmem_control_input_name, ShmemWriter};

use anyhow::Result;

pub struct ControlShmem {
    writer: ShmemWriter,
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
        let name = shmem_control_input_name(shm_prefix);
        let writer =
            ShmemWriter::new(&name, Self::CONTROL_WRITER_SIZE as usize, unlock_mapped_memory)
                .map_err(anyhow::Error::from)?;
        Ok(Self { writer })
    }

    pub fn reset(&self) -> Result<()> {
        self.writer.write_u64_at(ControlShmemOffsets::PrecompilesSize as usize, 0)?;
        self.writer.write_u64_at(ControlShmemOffsets::ShutdownFlag as usize, 0)?;
        self.writer.write_u64_at(ControlShmemOffsets::InputsSize as usize, 0)?;
        self.writer.write_u64_at(ControlShmemOffsets::ResetFlag as usize, 0)?;
        Ok(())
    }

    pub fn set_prec_hints_size(&self, size: u64) -> Result<()> {
        self.writer.write_u64_at(ControlShmemOffsets::PrecompilesSize as usize, size)?;
        Ok(())
    }

    pub fn prec_hints_size(&self) -> u64 {
        self.writer.read_u64_at(ControlShmemOffsets::PrecompilesSize as usize)
    }

    pub fn set_reset_flag(&self) -> Result<()> {
        self.writer.write_u64_at(ControlShmemOffsets::ResetFlag as usize, 1)?;
        Ok(())
    }

    pub fn inc_inputs_size(&self, size: usize) -> Result<()> {
        let current_size = self.writer.read_u64_at(ControlShmemOffsets::InputsSize as usize);
        let new_size = current_size + size as u64;
        self.writer.write_u64_at(ControlShmemOffsets::InputsSize as usize, new_size)?;
        Ok(())
    }
}
