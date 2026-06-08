use crate::{shmem_control_input_name, ShmemWriter};

use anyhow::Result;

/// This struct manages the shared memory for controlling the assembly runner from the Rust side.
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
        /// The total size of the control shared memory region, in bytes.
    pub const CONTROL_WRITER_SIZE: u64 = 0x1000; // 4KB

    /// Creates a new `ControlShmem` by opening and mapping the shared memory for control inputs.
    pub fn new(shm_prefix: &str, unlock_mapped_memory: bool) -> Result<Self> {
        let name = shmem_control_input_name(shm_prefix);
        let writer =
            ShmemWriter::new(&name, Self::CONTROL_WRITER_SIZE as usize, unlock_mapped_memory)
                .map_err(anyhow::Error::from)?;
        Ok(Self { writer })
    }

    /// Resets the control shared memory by clearing all flags and sizes to their default values.
    pub fn reset(&self) -> Result<()> {
        self.writer.write_u64_at(ControlShmemOffsets::PrecompilesSize as usize, 0)?;
        self.writer.write_u64_at(ControlShmemOffsets::ShutdownFlag as usize, 0)?;
        self.writer.write_u64_at(ControlShmemOffsets::InputsSize as usize, 0)?;
        self.writer.write_u64_at(ControlShmemOffsets::ResetFlag as usize, 0)?;
        Ok(())
    }

    /// Sets the precompiles size in the control shared memory.
    pub fn set_prec_hints_size(&self, size: u64) -> Result<()> {
        self.writer.write_u64_at(ControlShmemOffsets::PrecompilesSize as usize, size)?;
        Ok(())
    }

    /// Reads the precompiles size from the control shared memory.
    pub fn prec_hints_size(&self) -> u64 {
        self.writer.read_u64_at(ControlShmemOffsets::PrecompilesSize as usize)
    }

    /// Sets the reset flag in the control shared memory, which signals the C++ side to reset its state.
    pub fn set_reset_flag(&self) -> Result<()> {
        self.writer.write_u64_at(ControlShmemOffsets::ResetFlag as usize, 1)?;
        Ok(())
    }

    /// Increments the inputs size in the control shared memory by the given size, which signals the C++ side that new inputs have been added.
    pub fn inc_inputs_size(&self, size: usize) -> Result<()> {
        let current_size = self.writer.read_u64_at(ControlShmemOffsets::InputsSize as usize);
        let new_size = current_size + size as u64;
        self.writer.write_u64_at(ControlShmemOffsets::InputsSize as usize, new_size)?;
        Ok(())
    }
}
