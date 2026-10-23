/// Version of the cost model used by ziskemu to evaluate executions.
///
/// IMPORTANT: increase this version whenever the result of a cost evaluation can change, so that
/// costs reported with different versions are never compared as equivalent. That is:
/// - any cost in this file, or in `emulator/src/emu_costs.rs` (memory, main, rom, tables, ...),
/// - the way costs are accumulated (what is charged for an operation, a memory access, ...),
/// - the FROPS (frequent operations) tables or the way they are evaluated.
pub const ZISK_COSTS_VERSION: u32 = 1;

// Cost definitions: Area x Op
//
// Area is the total number of columns of the air (SUMMARY Total in build/setup.log). When several
// instances can prove an opcode, the cheapest one is used: an instance packing k operations per row
// (lanes_x_row / op_x_row / LANES) costs ceil(Total / k) per operation.
pub const INTERNAL_COST: u64 = 0;
pub const BINARY_COST: u64 = 219_u64.div_ceil(4); // BinaryHuge (4 lanes); Binary 60, BinaryLarge 57
pub const BINARY_ADD_COST: u64 = 83_u64.div_ceil(4); // BinaryAddHuge (4 lanes); BinaryAdd 26, BinaryAddLarge 23
pub const BINARY_E_COST: u64 = 110_u64.div_ceil(2); // BinaryExtensionLarge (2 lanes); BinaryExtension 58
pub const ARITHA32_COST: u64 = 97;
pub const ARITHAM32_COST: u64 = 97;
pub const KECCAK_COST: u64 = (615 * 135_u64).div_ceil(2); // 135 clocks per slot of 2 Keccak-f
pub const SHA256_COST: u64 = 72 * 122;
pub const POSEIDON_COST: u64 = 14 * 392;
pub const ARITH_EQ_COST: u64 = 90 * 16; // ArithEq (all equations), only air proving secp256r1
pub const ARITH_256_COST: u64 = 54 * 16; // Arith256X (arith256, arith256_mod)
pub const ARITH_SECP256K1_COST: u64 = 72 * 16; // ArithSecp256K1
pub const ARITH_BN254_COST: u64 = 78 * 16; // ArithBn254 (curve and complex ops)
pub const FCALL_COST: u64 = INTERNAL_COST;
pub const ARITH_EQ_384_COST: u64 = 80 * 24;
pub const ADD256_COST: u64 = 104;
pub const TWISTED_EDWARDS_COST: u64 = 109 * 16;
pub const BLAKE2B_COST: u64 = 8 * 233;
pub const BLAKE3_COST: u64 = (56 * 221_u64).div_ceil(2); // 2 lanes
pub const BLAKE2S_COST: u64 = 80 * 115;
pub const POSEIDON2_KOALABEAR_COST: u64 = 30 * 499;
pub const MAIN_COST: u64 = 245_u64.div_ceil(4); // Main (4 lanes)

// Zba costs. sh<n>add and slli.uw are proven natively (a single Binary / BinaryExtension
// operation); the .uw shift-and-adds are transpiled to `and` + native sh<n>add, and add.uw to
// `and` + `add`, so their cost is the one of the sequence: one extra main step plus both ops.
pub const ADD_U_W_COST: u64 = MAIN_COST + BINARY_COST + BINARY_ADD_COST; // step extra + and + add
pub const SH_ADD_COST: u64 = MAIN_COST + BINARY_E_COST + BINARY_ADD_COST; // decomposed sh<n>add: step extra + ssl + add (the native one is a single BINARY_COST op)
pub const SH_ADD_U_W_COST: u64 = MAIN_COST + BINARY_COST + BINARY_COST; // step extra + and + sh<n>add
pub const SLL_U_W_COST: u64 = BINARY_E_COST; // native slli.uw: a single binary extension op

/*
    Hash throughput comparison, where cost is clocks per columns:

    +------------+------------------------+-----------------+--------------+----------+----------+
    | Hash       | Full-op cost           | Msg bytes/block | Cost / byte  | Relative |    BF    |
    +------------+------------------------+-----------------+--------------+----------+----------+
    | Poseidon   | 14 x 392 = 5.488       |  96 (nominal)   |     57,2     |   1,0x   |    1     |
    | SHA2-256   | 72 x 122 = 8.784       |  64             |    137,3     |   2,4x   |    1     |
    | Blake2b    | 12 x 24 x 209 = 60.192 | 128             |    470,3     |   8,2x   |    1     |
    | Keccak-256 | 25 x 3023 = 75.575     | 136             |    555,7     |   9,7x   |    1     |
    +------------+------------------------+-----------------+--------------+----------+----------+

    Notes:
    - Poseidon bytes are nominal (12 Goldilocks elements x 8 bytes); a Goldilocks element
    holds ~63.99 bits, so the truly absorbable payload is slightly under 96 bytes.
    - Blake2b's BLAKE2B_COST is the cost of a single round; a full compression is 12 rounds,
    which is the full-op cost used in the table above.
*/

// Costs for DMA

// A single Dma instance (all operations enabled) proves every DMA operation, so all pay its row.
pub const DMA_COST: u64 = 63;
pub const DMA_INPUTCPY_COST: u64 = DMA_COST;
pub const DMA_MEMCMP_COST: u64 = DMA_COST;
pub const DMA_MEMCPY_COST: u64 = DMA_COST;
pub const DMA_MEMSET_COST: u64 = DMA_COST;
// Cost for computing the jumpdest bitmap of 1 64-bit source word: one op of JumpDest (2 ops x row)
pub const JUMP_DEST_COST: u64 = 66_u64.div_ceil(2);

// Costs for DMA PrePost (a single instance proves every operation)

pub const DMA_PRE_POST_COST: u64 = 104;
pub const DMA_PRE_POST_INPUTCPY_COST: u64 = DMA_PRE_POST_COST;
pub const DMA_PRE_POST_MEMCMP_COST: u64 = DMA_PRE_POST_COST;
pub const DMA_PRE_POST_MEMCPY_COST: u64 = DMA_PRE_POST_COST;
pub const DMA_PRE_POST_MEMSET_COST: u64 = DMA_PRE_POST_COST;

// Costs for DMA 64-bits aligned loops

pub const DMA_64_ALIGNED_COST: u64 = 77;
pub const DMA_64_ALIGNED_DIVISOR: u64 = 4;

// Only the general airs (Dma64Aligned, Dma64AlignedLarge) prove an input copy.
pub const DMA_64_ALIGNED_INPUTCPY_COST: u64 = DMA_64_ALIGNED_COST;
pub const DMA_64_ALIGNED_INPUTCPY_DIVISOR: u64 = DMA_64_ALIGNED_DIVISOR;

pub const DMA_64_ALIGNED_MEM_COST: u64 = 50;
pub const DMA_64_ALIGNED_MEM_DIVISOR: u64 = 4;

pub const DMA_64_ALIGNED_MEMCMP_COST: u64 = DMA_64_ALIGNED_MEM_COST;
pub const DMA_64_ALIGNED_MEMCMP_DIVISOR: u64 = DMA_64_ALIGNED_MEM_DIVISOR;

pub const DMA_64_ALIGNED_MEMCPY_COST: u64 = 67;
pub const DMA_64_ALIGNED_MEMCPY_DIVISOR: u64 = 8;

pub const DMA_64_ALIGNED_MEMSET_COST: u64 = 35;
pub const DMA_64_ALIGNED_MEMSET_DIVISOR: u64 = 8;

// Costs for DMA unaligned loops

pub const DMA_UNALIGNED_COST: u64 = 42;
pub const DMA_UNALIGNED_INPUTCPY_COST: u64 = DMA_UNALIGNED_COST;
pub const DMA_UNALIGNED_MEMCMP_COST: u64 = DMA_UNALIGNED_COST;
pub const DMA_UNALIGNED_MEMCPY_COST: u64 = DMA_UNALIGNED_COST;
pub const DMA_UNALIGNED_MEMSET_COST: u64 = DMA_UNALIGNED_COST;
