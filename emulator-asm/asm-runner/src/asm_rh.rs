use std::fmt::Debug;

use crate::{AsmSharedMemory, AsmShmemHeader};

#[repr(C)]
#[derive(Debug, Default)]
pub struct AsmRHHeader {
    pub version: u64,
    pub exit_code: u64,
    pub shmem_allocated_size: u64,
    pub steps: u64,
}

impl AsmShmemHeader for AsmRHHeader {
    fn allocated_size(&self) -> u64 {
        self.shmem_allocated_size
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct AsmRHData {
    pub steps: u64,
    pub inst_count: Vec<u64>,
}

impl AsmRHData {
    pub fn new(steps: u64, inst_count: Vec<u64>) -> Self {
        AsmRHData { steps, inst_count }
    }
}

impl AsmRHData {
    /// Build an [`AsmRHData`] by reading the ROM histogram out of shared memory.
    ///
    /// # Invariant (load-bearing)
    /// `inst_count` is constructed with [`Vec::from_raw_parts`] pointing DIRECTLY
    /// into the shared-memory mapping — it is NOT allocated by Rust's global
    /// allocator. Dropping that `Vec` the normal way would make the allocator
    /// free pages it never owned (undefined behavior / heap corruption).
    ///
    /// The returned `AsmRHData` must therefore never be dropped normally:
    /// `AsmRunnerRH::drop` (in `asm_rh_runner.rs`) `mem::forget`s it before the
    /// mapping is torn down. These two sites are a matched pair — do not change
    /// the `from_raw_parts` construction here without updating that `Drop`, and
    /// vice versa.
    pub fn from_shared_memory(asm_shared_memory: &AsmSharedMemory<AsmRHHeader>) -> AsmRHData {
        // SAFETY: `data_ptr` points into the live, read-only shared mapping owned by
        // `asm_shared_memory`, which the caller keeps alive across this read. The
        // header read and `Vec::from_raw_parts` stay in bounds — the `assert!` below
        // rejects any `len` that would run past the mapped region. The returned `Vec`
        // aliases the mapping and must never be freed by Rust's allocator; see the
        // `# Invariant` above and `AsmRunnerRH::drop`.
        unsafe {
            let data_ptr = asm_shared_memory.data_ptr() as *mut u64;
            // chunk data
            let len = std::ptr::read(data_ptr) as usize;
            assert!(
                (len + 1) * 8 <= (asm_shared_memory.mapped_size() - size_of::<AsmRHHeader>()),
                "Data length {} exceeds allocated shared memory size",
                len
            );
            let data_ptr = data_ptr.add(1);
            let inst_count = Vec::from_raw_parts(data_ptr, len, len);

            AsmRHData { steps: asm_shared_memory.map_header().steps, inst_count }
        }
    }
}
