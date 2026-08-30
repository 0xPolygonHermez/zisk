// Cost definitions: Area x Op
pub const INTERNAL_COST: u64 = 0;
pub const BINARY_COST: u64 = 60;
pub const BINARY_ADD_COST: u64 = 25;
pub const BINARY_E_COST: u64 = 56;
pub const ARITHA32_COST: u64 = 97;
pub const ARITHAM32_COST: u64 = 97;
pub const KECCAK_COST: u64 = 2652 * 29 / 2;
pub const SHA256_COST: u64 = 72 * 122;
pub const POSEIDON_COST: u64 = 14 * 392;
pub const ARITH_EQ_COST: u64 = 90 * 16;
pub const FCALL_COST: u64 = INTERNAL_COST;
pub const ARITH_EQ_384_COST: u64 = 80 * 24;
pub const ADD256_COST: u64 = 104;
// CLOCKS (8) x area (cm1 119 + cm2 109 + cm3 6 = 234). Was 24 * 209: the 24 is
// the pre-21d497cd3 CLOCKS (CLOCKS_PER_G was 3) and 209 the area before that
// commit reshaped the AIR, so it overstated a blake2b round by 2.68x.
pub const BLAKE2_COST: u64 = 8 * 234;
// CLOCKS (8) x area (cm1 73 + cm2 76 + cm3 6 = 155), read from the compiled
// Blake2sr.starkinfo.json.
pub const BLAKE2S_COST: u64 = 8 * 155;
pub const MAIN_COST: u64 = 68;

pub const ADD_U_W_COST: u64 = MAIN_COST + BINARY_COST + BINARY_ADD_COST; // step extra + and + add
pub const SH_ADD_COST: u64 = MAIN_COST + BINARY_E_COST + BINARY_ADD_COST; // step extra + ssl + add
pub const SH_ADD_U_W_COST: u64 = 2 * MAIN_COST + BINARY_E_COST + BINARY_COST + BINARY_ADD_COST; // 2 * step extra + ssl + and + add
pub const SLL_U_W_COST: u64 = MAIN_COST + BINARY_COST + BINARY_E_COST; // step extra + and + ssl

/*
    Hash throughput comparison, where cost is clocks per columns:

    +------------+------------------------+-----------------+--------------+----------+----------+
    | Hash       | Full-op cost           | Msg bytes/block | Cost / byte  | Relative |    BF    |
    +------------+------------------------+-----------------+--------------+----------+----------+
    | Poseidon   | 14 x 392 = 5.488       |  96 (nominal)   |     57,2     |   1,0x   |    1     |
    | SHA2-256   | 72 x 122 = 8.784       |  64             |    137,3     |   2,4x   |    1     |
    | Blake2b    | 12 x 8 x 234 = 22.464  | 128             |    175,5     |   3,1x   |    1     |
    | Blake2s    | 10 x 8 x 155 = 12.400  |  64             |    193,8     |   3,4x   |    1     |
    | Keccak-256 | 25 x 3023 = 75.575     | 136             |    555,7     |   9,7x   |    1     |
    +------------+------------------------+-----------------+--------------+----------+----------+

    Notes:
    - Poseidon bytes are nominal (12 Goldilocks elements x 8 bytes); a Goldilocks element
    holds ~63.99 bits, so the truly absorbable payload is slightly under 96 bytes.
    - BLAKE2_COST and BLAKE2S_COST are the cost of a single round; a full compression is
    12 rounds for Blake2b and 10 for Blake2s, which is the full-op cost used above.
    - The Keccak-256 row still shows 25 x 3023 = 75.575, which predates 170673fab changing
    KECCAK_COST to 2652 x 29 / 2 = 38.454 (282,8 per byte, 4,9x). Left as found.
*/

// Costs for DMA

pub const DMA_COST: u64 = 61;
pub const DMA_INPUTCPY_COST: u64 = 40;
pub const DMA_MEMCMP_COST: u64 = DMA_COST;
pub const DMA_MEMCPY_COST: u64 = 46;
pub const DMA_MEMSET_COST: u64 = DMA_COST;
pub const JUMP_DEST_COST: u64 = 63 * 4; // Cost for computing the jumpdest bitmap 1 64-bit word

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
