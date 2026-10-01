use std::fmt::Debug;

use crate::{AsmShmem, AsmShmemHeader};

#[repr(C)]
#[derive(Debug, Default)]
pub(crate) struct AsmRHHeader {
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

/// This struct represents the ROM histogram data collected from the assembly code execution.
#[repr(C)]
#[derive(Debug, Default)]
pub struct AsmRHData {
    /// The number of steps executed.
    pub steps: u64,
    /// A vector containing the instruction count for each ROM.
    pub inst_count: Vec<u64>,
    /// Multiplicity of every row of the global FROPS table, i.e. how many times each frequent
    /// operation was executed (`zisk_core::frops::FROPS_TABLE_ROWS` counters).
    pub frops_count: Vec<u64>,
}

impl AsmRHData {
    /// Creates a new `AsmRHData` with the given number of steps and multiplicity vectors.
    pub fn new(steps: u64, inst_count: Vec<u64>, frops_count: Vec<u64>) -> Self {
        AsmRHData { steps, inst_count, frops_count }
    }
}

impl AsmRHData {
    /// Build an [`AsmRHData`] by reading the ROM histogram out of shared memory.
    ///
    /// # Invariant (load-bearing)
    /// `inst_count` and `frops_count` are constructed with [`Vec::from_raw_parts`]
    /// pointing DIRECTLY into the shared-memory mapping — they are NOT allocated by
    /// Rust's global allocator. Dropping those `Vec`s the normal way would make the
    /// allocator free pages it never owned (undefined behavior / heap corruption).
    ///
    /// The returned `AsmRHData` must therefore never be dropped normally:
    /// `AsmRunnerRH::drop` (in `asm_rh_runner.rs`) `mem::forget`s it before the
    /// mapping is torn down. These two sites are a matched pair — do not change
    /// the `from_raw_parts` construction here without updating that `Drop`, and
    /// vice versa.
    pub(crate) fn from_shared_memory(asm_shared_memory: &AsmShmem<AsmRHHeader>) -> AsmRHData {
        // SAFETY: `data_ptr` points into the live, read-only shared mapping owned by
        // `asm_shared_memory`, which the caller keeps alive across this read. The
        // header reads and `Vec::from_raw_parts` stay in bounds — the `assert!`s below
        // reject any length that would run past the mapped region, each one checked
        // before the pointer it validates is dereferenced. The returned `Vec`s alias
        // the mapping and must never be freed by Rust's allocator; see the
        // `# Invariant` above and `AsmRunnerRH::drop`.
        unsafe {
            let available = asm_shared_memory.mapped_size() - std::mem::size_of::<AsmRHHeader>();
            let data_ptr = asm_shared_memory.data_ptr() as *mut u64;

            // Instruction multiplicity: [len][counter; len]
            let len = std::ptr::read(data_ptr) as usize;
            assert!(
                (len + 2) * 8 <= available,
                "Data length {len} exceeds allocated shared memory size"
            );
            let inst_count = Vec::from_raw_parts(data_ptr.add(1), len, len);

            // FROPS multiplicity, which follows it: [frops_len][counter; frops_len]
            let frops_ptr = data_ptr.add(1 + len);
            let frops_len = std::ptr::read(frops_ptr) as usize;
            assert!(
                (len + frops_len + 2) * 8 <= available,
                "FROPS length {frops_len} exceeds allocated shared memory size"
            );
            let frops_count = Vec::from_raw_parts(frops_ptr.add(1), frops_len, frops_len);

            AsmRHData { steps: asm_shared_memory.map_header().steps, inst_count, frops_count }
        }
    }
}
