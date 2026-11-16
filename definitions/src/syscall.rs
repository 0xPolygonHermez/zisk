// Syscall 0x800 - 0x84F (80 syscalls)

// Existing contiguous syscall IDs match riscv2zisk_context.rs. New provisional
// IDs with reserved gaps are dispatched explicitly, outside that legacy table.

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
pub const SYSCALL_BABYJUBJUB_ADD_ID: u16 = 0x81D;
pub const SYSCALL_BLAKE3F_ID: u16 = 0x81E;
pub const SYSCALL_BLAKE2SF_ID: u16 = 0x81F;
pub const SYSCALL_ED25519_ADD_ID: u16 = 0x820;
pub const SYSCALL_POSEIDON2_KOALABEAR_ID: u16 = 0x821;
/// Experimental: emulator only, no state machine proves it yet.
pub const SYSCALL_DOT_I8_ID: u16 = 0x822;
/// Experimental `floatdot`: FP32-accumulating ordered dot with FP32, BF16 or FP16 inputs;
/// requires the matching proof setup.
/// Assembly and hints execution remain unsupported.
// Provisional: coordinate upstream allocation before integration.
pub const SYSCALL_FLOATDOT_ID: u16 = 0x823;
pub const FLOATDOT_MAX_LEN: usize = 4096;
/// Legacy FP32-dot names for existing callers; same ID and limit as `floatdot`.
pub const SYSCALL_FP32_DOT_ID: u16 = SYSCALL_FLOATDOT_ID;
pub const FP32_DOT_MAX_LEN: usize = FLOATDOT_MAX_LEN;

/// blake2b `extended_arg`: round index (bits 0..3), state and m from the previous round of the
/// call (`BLAKE2B_RD_CHAIN`), to the next one (`BLAKE2B_WR_CHAIN`), steps back to it (bits 6..).
pub const BLAKE2B_ROUND_MASK: u64 = 0xF;
pub const BLAKE2B_RD_CHAIN: u64 = 1 << 4;
pub const BLAKE2B_WR_CHAIN: u64 = 1 << 5;
pub const BLAKE2B_GAP_SHIFT: u64 = 6;
/// Steps back to the previous round of a chain whose rounds are consecutive instructions.
pub const BLAKE2B_GAP: u64 = 1 << BLAKE2B_GAP_SHIFT;
/// Flags of the first, middle and last round of such a chain.
pub const BLAKE2B_CHAIN_FIRST: u64 = BLAKE2B_WR_CHAIN;
pub const BLAKE2B_CHAIN_MID: u64 = BLAKE2B_RD_CHAIN | BLAKE2B_WR_CHAIN | BLAKE2B_GAP;
pub const BLAKE2B_CHAIN_LAST: u64 = BLAKE2B_RD_CHAIN | BLAKE2B_GAP;
