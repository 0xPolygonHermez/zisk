//! Zisk ROM to code: target-neutral definitions
//!
//! Definitions shared by every Zisk ROM code generator, regardless of the language or the target
//! architecture the code is generated for (e.g. i86-64 assembly in `zisk_rom_2_asm`).
//!
//! What belongs here is everything that describes *what* the generated program must do, i.e. the
//! contracts it shares with the rest of the system:
//! - the generation methods (which trace, if any, the generated program produces),
//! - the memory layout of the traces it writes (ROM histogram, mem op trace),
//! - the layout of the fcall context it exchanges with the C runtime,
//! - the mem op flags it encodes,
//! - which Zisk operations are precompiled, and which precompiles provide results.
//!
//! What does NOT belong here is anything about *how* the code is written: register allocation,
//! instruction mnemonics, comment syntax, calling conventions.  Those are backend-specific.
//!
//! Keeping these definitions in one place is what allows several backends to coexist: a divergence
//! in mnemonics between backends is a compilation error, but a divergence in trace layout or mem op
//! encoding would silently produce a wrong trace.

// NOTE: `::zisk_definitions` is the zisk-definitions crate, not the crate::zisk_definitions module
// of the same name.  The leading `::` keeps the two apart.
use crate::zisk_ops::ZiskOp;

/// ZisK Emulator can be executed in assembly to get the maximum performance
/// in the first sequential emulation.
///
/// ROM histogram contains a counter per program counter that is incremented every time that
/// instruction is executed.  It is generated in one single, sequential emulation.
///
/// Mem reads contain all the memory reads done during a chunk of the emulation.  Mem reads chunks
/// are generated sequentially, and consumed in parallel after the first chunk is ready to generate
/// the main AIR traces.
///
/// Mem trace contains a record of all the memory operations: step, r/w, address, width, write
/// value, etc.  Mem trace is generated sequentially in chunks, which are consumed in parallel in C
/// to generate the memory AIR plan and AIR traces.
///
/// ```text
///                 /-> [ASM seq] -> ROM Histogram
///                /
/// RISC-V -> ZisK ---> [ASM seq] -> Mem Reads chunks -> [ASM par chunk player] -> Main Trace
///                \
///                 \-> [ASM seq] -> Mem Trace chunks -> [  C par chunk player] -> Mem Plan & Trace
/// ```
///
/// Other meaningful assembly emulation methods used for performance investigation include:
/// - Fast: Does not generate any trace, but simply emulates the program.  It is the fastest method.
/// - Chunks: Stops every chunk-size steps, without generating traces.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AsmGenerationMethod {
    /// Generate assembly code to not even stop at chunks, nor generate trace, i.e. fast
    #[default]
    AsmFast,
    /// Generate assembly code to compute the minimal trace
    AsmMinimalTraces,
    /// Generate assembly code to compute the ROM histogram
    AsmRomHistogram,
    /// Generate assembly code to compute the memory operations [w/r, width, address] trace
    AsmMemOp,
}

impl AsmGenerationMethod {
    pub fn is_fast(&self) -> bool {
        *self == AsmGenerationMethod::AsmFast
    }
    pub fn is_minimal_trace(&self) -> bool {
        *self == AsmGenerationMethod::AsmMinimalTraces
    }
    pub fn is_rom_histogram(&self) -> bool {
        *self == AsmGenerationMethod::AsmRomHistogram
    }
    pub fn is_mem_op(&self) -> bool {
        *self == AsmGenerationMethod::AsmMemOp
    }
}

/// ROM histogram trace base address.  Only used to calculate the histogram position for every rom
/// pc, via `rom_histogram_trace_address()`.
pub(crate) const TRACE_ADDR_NUMBER: u64 = 0xd0000000 + 0x20;

/// Address of the ROM histogram counter of the instruction at the given ROM index.
///
/// ROM histogram structure:
///
/// ROM trace control:
///     [8B] version
///     [8B] exit_code (0=success, 1=not completed)
///     [8B] allocated_size = xxx (bytes)
///     [8B] executed steps
/// Instruction histogram: (TRACE_ADDR_NUMBER)
///     [8B] multiplicity_size = S
///     [8B] multiplicity[0]
///     [8B] multiplicity[1]
///     …
///     [8B] multiplicity[S-1]
pub(crate) fn rom_histogram_trace_address(index: u64) -> u64 {
    TRACE_ADDR_NUMBER + (1 + index) * 8
}

// Fcall params and result lengths
// NOTE: if these parameters are update, review dma_constants.inc
pub(crate) const FCALL_PARAMS_LENGTH: u64 = 386;
pub(crate) const FCALL_RESULT_LENGTH: u64 = 8193;

// Fcall context offsets of the different fields
pub(crate) const FCALL_FUNCTION_ID: u64 = 0;
pub(crate) const FCALL_PARAMS_CAPACITY: u64 = FCALL_FUNCTION_ID + 1;
pub(crate) const FCALL_PARAMS_SIZE: u64 = FCALL_PARAMS_CAPACITY + 1;
pub(crate) const FCALL_PARAMS: u64 = FCALL_PARAMS_SIZE + 1;
pub(crate) const FCALL_RESULT_CAPACITY: u64 = FCALL_PARAMS + FCALL_PARAMS_LENGTH;
pub(crate) const FCALL_RESULT_SIZE: u64 = FCALL_RESULT_CAPACITY + 1;
pub(crate) const FCALL_RESULT: u64 = FCALL_RESULT_SIZE + 1;
pub(crate) const FCALL_RESULT_GOT: u64 = FCALL_RESULT + FCALL_RESULT_LENGTH;
pub(crate) const FCALL_LENGTH: u64 = FCALL_RESULT_GOT + 1;

// One-pass (single emulation) memory trace, used to count, plan and collect.
// If ZisK instruction contains at least one memory operation:
//   [32b] header (from higher bits to lower bits)
//     [1b] read_a
//       0 = no reg a mem op
//       1 = one reg a mem op
//     [3b] read_b
//       0 = no reg b mem op
//       1 = one reg b mem op of width 1
//       2 = one reg b mem op of width 2
//       3 = one reg b mem op of width 4
//       4 = one reg b mem op of width 8
//     [3b] write
//       0 = no write op
//       1 = one write c mem op of width 1
//       2 = one write c mem op of width 2
//       3 = one write c mem op of width 4
//       4 = one write c mem op of width 8
//       5 = one precompiled mem op of contiguous addresses
//       6 = one precompiled mem op of non-contiguous addresses
//     [25b] relative step: lower bits of step
// If header.read_a == 1:
//   [32b] a mem address
// If header.read_b == 1, 2, 3 or 4:
//   [32b] b mem address
// If header.write == 1, 2, 3 or 4
//   [32b] c mem address
//   [64b] c write value
// If header.write == 5
//   [32b] prec_cont_count = prec_read_count + prec_write_count<<16
//   [32b] prec_const_address
//   [64b x prec_write_count] prec_cont_write_data
// If header.write == 6
//   [32b] prec_non_cont_count = prec_read_count + prec_write_count<<16
//   [32b x prec_read_count] prec_non_cont_read_address = precompiled read addresses
//   [32b x prec_write_count] prec_non_const_write_address = precompiled write addresses
//   [64b x prec_write_count] prec_non_const_write_data = precompiled write data
// If not aligned to 64b
//   [32b] padding zeros
pub(crate) const F_MOPS_CLEAR_WRITE_BYTE: u64 = 1 << 37;

pub(crate) const F_MOPS_BLOCK_READ: u64 = 0x0000_000A_0000_0000;
pub(crate) const F_MOPS_BLOCK_WRITE: u64 = 0x0000_000B_0000_0000;

pub(crate) const F_MOPS_READ_8: u64 = 0x0000_0008_0000_0000;
pub(crate) const F_MOPS_READ_4: u64 = 0x0000_0004_0000_0000;
pub(crate) const F_MOPS_READ_2: u64 = 0x0000_0002_0000_0000;
pub(crate) const F_MOPS_READ_1: u64 = 0x0000_0001_0000_0000;

pub(crate) const F_MOPS_WRITE_8: u64 = 0x0000_0018_0000_0000;
pub(crate) const F_MOPS_WRITE_4: u64 = 0x0000_0014_0000_0000;
pub(crate) const F_MOPS_WRITE_2: u64 = 0x0000_0012_0000_0000;
pub(crate) const F_MOPS_WRITE_1: u64 = 0x0000_0011_0000_0000;

pub(crate) const F_MOPS_ALIGNED_READ: u64 = 0x0000_000C_0000_0000;
pub(crate) const F_MOPS_ALIGNED_WRITE: u64 = 0x0000_000D_0000_0000;
// pub(crate) const F_MOPS_ALIGNED_BLOCK_READ: u64 = 0x0000_000E_0000_0000;
// pub(crate) const F_MOPS_ALIGNED_BLOCK_WRITE: u64 = 0x0000_000F_0000_0000;
pub(crate) const F_MOPS_BLOCK_LENGTH_SHIFT: u64 = 36;

// const PRECOMPILE_BUFFER_SIZE_IN_BYTES: u64 = 0x100000; // 1MB
pub(crate) const PRECOMPILE_BUFFER_SIZE_IN_BYTES: u64 = 0x8000000; // 128MB
pub(crate) const PRECOMPILE_BUFFER_SIZE_IN_U64: u64 = PRECOMPILE_BUFFER_SIZE_IN_BYTES / 8;
pub(crate) const PRECOMPILE_BUFFER_SIZE_U64_MASK: u64 = PRECOMPILE_BUFFER_SIZE_IN_U64 - 1;

/// True if the given Zisk operation is implemented by a precompile, i.e. by a call to an external
/// function instead of by generated code.
pub fn op_is_precompiled(zisk_op: &ZiskOp) -> bool {
    matches!(
        zisk_op,
        ZiskOp::Keccak
            | ZiskOp::Sha256
            | ZiskOp::Poseidon2
            | ZiskOp::Poseidon1
            | ZiskOp::Arith256
            | ZiskOp::Arith256Mod
            | ZiskOp::Secp256k1Add
            | ZiskOp::Secp256k1Dbl
            | ZiskOp::Bn254CurveAdd
            | ZiskOp::Bn254CurveDbl
            | ZiskOp::Bn254ComplexAdd
            | ZiskOp::Bn254ComplexSub
            | ZiskOp::Bn254ComplexMul
            | ZiskOp::Arith384Mod
            | ZiskOp::Bls12_381CurveAdd
            | ZiskOp::Bls12_381CurveDbl
            | ZiskOp::Bls12_381ComplexAdd
            | ZiskOp::Bls12_381ComplexSub
            | ZiskOp::Bls12_381ComplexMul
            | ZiskOp::Add256
            | ZiskOp::Secp256r1Add
            | ZiskOp::Secp256r1Dbl
            | ZiskOp::Blake2
    )
}

/// Which precompiles provide their results to the generated code, instead of the generated code
/// having to compute them.
///
/// Wraps the "consuming precompile results" flag: when it is false every query is false, and when
/// it is true each precompile answers according to its own `zisk_definitions` switch.
#[derive(Default, Debug, Clone, Copy)]
pub struct PrecompileResults {
    /// Set to true if we are consuming precompile results
    enabled: bool,
}

impl PrecompileResults {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn keccak(&self) -> bool {
        ::zisk_definitions::KECCAK_RESULTS && self.enabled
    }
    pub fn sha256(&self) -> bool {
        ::zisk_definitions::SHA256_RESULTS && self.enabled
    }
    pub fn arith256(&self) -> bool {
        self.enabled
    }
    pub fn arith256mod(&self) -> bool {
        ::zisk_definitions::ARITH256MOD_RESULTS && self.enabled
    }
    pub fn secp256k1add(&self) -> bool {
        self.enabled
    }
    pub fn secp256k1dbl(&self) -> bool {
        self.enabled
    }
    pub fn secp256r1add(&self) -> bool {
        self.enabled
    }
    pub fn secp256r1dbl(&self) -> bool {
        self.enabled
    }
    pub fn fcall(&self) -> bool {
        self.enabled
    }
    pub fn bn254curveadd(&self) -> bool {
        self.enabled
    }
    pub fn bn254curvedbl(&self) -> bool {
        self.enabled
    }
    pub fn bn254complexadd(&self) -> bool {
        self.enabled
    }
    pub fn bn254complexsub(&self) -> bool {
        self.enabled
    }
    pub fn bn254complexmul(&self) -> bool {
        self.enabled
    }
    pub fn arith384mod(&self) -> bool {
        self.enabled
    }
    pub fn bls12_381curveadd(&self) -> bool {
        self.enabled
    }
    pub fn bls12_381curvedbl(&self) -> bool {
        self.enabled
    }
    pub fn bls12_381complexadd(&self) -> bool {
        self.enabled
    }
    pub fn bls12_381complexsub(&self) -> bool {
        self.enabled
    }
    pub fn bls12_381complexmul(&self) -> bool {
        self.enabled
    }
    pub fn add256(&self) -> bool {
        self.enabled
    }
    pub fn blake2(&self) -> bool {
        //self.enabled
        false
    }
    pub fn call_wait_for_prec_avail(&self) -> bool {
        self.enabled
    }
}
