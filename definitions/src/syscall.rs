// Syscall 0x800 - 0x84F (80 syscalls)

// Important: Syscalls should be contiguous and in the same order as in riscv2zisk_context.rs

pub const SYSCALL_KECCAKF_ID: u16 = 0x800;
pub const SYSCALL_ARITH256_ID: u16 = 0x801;
pub const SYSCALL_ARITH256_MOD_ID: u16 = 0x802;
pub const SYSCALL_SECP256K1_ADD_ID: u16 = 0x803;
pub const SYSCALL_SECP256K1_DBL_ID: u16 = 0x804;
pub const SYSCALL_SHA256F_ID: u16 = 0x805;
pub const SYSCALL_BN254_CURVE_ADD_ID: u16 = 0x806;
pub const SYSCALL_BN254_CURVE_DBL_ID: u16 = 0x807;
pub const SYSCALL_BN254_COMPLEX_ADD_ID: u16 = 0x808;
pub const SYSCALL_BN254_COMPLEX_SUB_ID: u16 = 0x809;
pub const SYSCALL_BN254_COMPLEX_MUL_ID: u16 = 0x80A;
pub const SYSCALL_ARITH384_MOD_ID: u16 = 0x80B;
pub const SYSCALL_BLS12_381_CURVE_ADD_ID: u16 = 0x80C;
pub const SYSCALL_BLS12_381_CURVE_DBL_ID: u16 = 0x80D;
pub const SYSCALL_BLS12_381_COMPLEX_ADD_ID: u16 = 0x80E;
pub const SYSCALL_BLS12_381_COMPLEX_SUB_ID: u16 = 0x80F;
pub const SYSCALL_BLS12_381_COMPLEX_MUL_ID: u16 = 0x810;
pub const SYSCALL_ADD256_ID: u16 = 0x811;
pub const SYSCALL_POSEIDON2_ID: u16 = 0x812;
pub const SYSCALL_DMA_MEMCPY_ID: u16 = 0x813;
pub const SYSCALL_DMA_MEMCMP_ID: u16 = 0x814;
pub const SYSCALL_DMA_INPUTCPY_ID: u16 = 0x815;
pub const SYSCALL_DMA_MEMSET_ID: u16 = 0x816;
pub const SYSCALL_SECP256R1_ADD_ID: u16 = 0x817;
pub const SYSCALL_SECP256R1_DBL_ID: u16 = 0x818;
pub const SYSCALL_BLAKE2B_ROUND_ID: u16 = 0x819;
pub const SYSCALL_PROFILE_ID: u16 = 0x81A;
pub const SYSCALL_POSEIDON1_ID: u16 = 0x81B;
pub const SYSCALL_JUMP_DEST_ID: u16 = 0x81C;
pub const SYSCALL_DMA_MTCPY_ID: u16 = 0x81D;
pub const SYSCALL_DMA_MTCMP_ID: u16 = 0x81E;
pub const SYSCALL_TEMPORAL_REF_ID: u16 = 0x81F;

/// Requests a temporal reference *and* advises a region under it, in one operation.
///
/// The two-step form (a [`SYSCALL_TEMPORAL_REF_ID`] request followed by an `execute_advice`
/// pattern) stays available and is still the only way to put more than one region under the same
/// reference.  This is the common case folded into a single instruction: one region, its reference
/// handed straight back to the guest.
pub const SYSCALL_TEMPORAL_REF_ADVICE_ID: u16 = 0x820;

/// Immediate of the two `addi x0, x0, ID` markers that delimit the `execute_advice` pattern.
///
/// The middle instruction of the pattern (`addi x0, reg(address), count`) is, on its own,
/// indistinguishable from any other hint `addi`, so the transpiler only recognises it when it is
/// wrapped by these two markers.  Must fit in a signed 12-bit immediate.
pub const EXECUTE_ADVICE_MARKER_ID: i32 = 0x5AD;

/// Value the transpiler puts in `b` of the `flag` operation that requests a temporal reference,
/// so that the emulator can tell such a request apart from any other `flag` (nop, hint, jal).
/// Out of reach of a 12-bit `addi` immediate, which is the only other source of `b` for `flag`.
pub const TEMPORAL_REF_REQUEST_TAG: u64 = 0x5A17_C0DE;
