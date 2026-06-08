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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{shmem_control_input_name, ShmemReader};
    use std::ffi::CString;

    fn create_segment(name: &str, size: usize) {
        let c = CString::new(name).unwrap();
        unsafe {
            libc::shm_unlink(c.as_ptr());
            let fd = libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
            assert!(fd >= 0);
            assert_eq!(libc::ftruncate(fd, size as libc::off_t), 0);
            libc::close(fd);
        }
    }
    fn unlink_segment(name: &str) {
        let c = CString::new(name).unwrap();
        unsafe { libc::shm_unlink(c.as_ptr()) };
    }

    #[test]
    fn control_fields_round_trip_at_their_offsets() {
        let prefix = format!("ZISK_unittest_ctrl_{}", std::process::id());
        let seg = shmem_control_input_name(&prefix);
        let size = ControlShmem::CONTROL_WRITER_SIZE as usize;
        create_segment(&seg, size);

        let c = ControlShmem::new(&prefix, true).unwrap();
        let r = ShmemReader::new(&seg, size).unwrap();

        // PrecompilesSize @ offset 0
        c.set_prec_hints_size(99).unwrap();
        assert_eq!(c.prec_hints_size(), 99);
        assert_eq!(r.read_u64_at(0), 99);

        // InputsSize @ offset 16 accumulates
        c.inc_inputs_size(10).unwrap();
        c.inc_inputs_size(5).unwrap();
        assert_eq!(r.read_u64_at(16), 15);

        // ResetFlag @ offset 24
        c.set_reset_flag().unwrap();
        assert_eq!(r.read_u64_at(24), 1);

        // reset() clears every slot
        c.reset().unwrap();
        assert_eq!(c.prec_hints_size(), 0);
        assert_eq!(r.read_u64_at(16), 0);
        assert_eq!(r.read_u64_at(24), 0);

        drop(r);
        unlink_segment(&seg);
    }
}
