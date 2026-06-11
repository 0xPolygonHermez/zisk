// Cost definitions: Area x Op
pub const INTERNAL_COST: u64 = 0;
pub const BINARY_COST: u64 = 60;
pub const BINARY_ADD_COST: u64 = 25;
pub const BINARY_E_COST: u64 = 53;
pub const ARITHA32_COST: u64 = 95;
pub const ARITHAM32_COST: u64 = 95;
pub const KECCAK_COST: u64 = 25 * 3022;
pub const SHA256_COST: u64 = 72 * 121;
pub const POSEIDON2_COST: u64 = 14 * 75;
pub const ARITH_EQ_COST: u64 = 89 * 16;
pub const FCALL_COST: u64 = INTERNAL_COST;
pub const ARITH_EQ_384_COST: u64 = 79 * 24;
pub const ADD256_COST: u64 = 104;
pub const BLAKE2_COST: u64 = 24 * 205;

/*
    Hash throughput comparison:

    +------------+----------------------+-----------------+--------------+----------+
    | Hash       | Full-op cost         | Msg bytes/block | Cost / byte  | Relative |
    +------------+----------------------+-----------------+--------------+----------+
    | Poseidon2  |  1.050               |  96 (nominal)   |     10,9     |   1,0x   |
    | SHA2-256   |  8.712               |  64             |    136,1     |  12,4x   |
    | Blake2b    | 59.040  (12 x 4.920) | 128             |    461,3     |  42,2x   |
    | Keccak-256 | 75.550               | 136             |    555,5     |  50,9x   |
    +------------+----------------------+-----------------+--------------+----------+

    Notes:
    - Poseidon2 bytes are nominal (12 Goldilocks elements x 8 bytes); a Goldilocks element
    holds ~63.99 bits, so the truly absorbable payload is slightly under 96 bytes.
    - Blake2b's BLAKE2_COST is the cost of a single round; a full compression is 12 rounds,
    which is the full-op cost used in the table above.
*/

// Costs for DMA

pub const DMA_COST: u64 = 61;
pub const DMA_INPUTCPY_COST: u64 = 40;
pub const DMA_MEMCMP_COST: u64 = DMA_COST;
pub const DMA_MEMCPY_COST: u64 = 46;
pub const DMA_MEMSET_COST: u64 = DMA_COST;

// Costs for DMA PrePost

pub const DMA_PRE_POST_COST: u64 = 104;
pub const DMA_PRE_POST_INPUTCPY_COST: u64 = 59;
pub const DMA_PRE_POST_MEMCMP_COST: u64 = DMA_PRE_POST_COST;
pub const DMA_PRE_POST_MEMCPY_COST: u64 = 91;
pub const DMA_PRE_POST_MEMSET_COST: u64 = DMA_PRE_POST_COST;

// Costs for DMA 64-bits aligned loops

pub const DMA_64_ALIGNED_COST: u64 = 77;
pub const DMA_64_ALIGNED_DIVISOR: u64 = 4;

pub const DMA_64_ALIGNED_INPUTCPY_COST: u64 = 58;
pub const DMA_64_ALIGNED_INPUTCPY_DIVISOR: u64 = 4;

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
