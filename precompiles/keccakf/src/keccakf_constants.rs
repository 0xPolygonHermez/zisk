pub(crate) const LANES: usize = 25;
pub(crate) const LANE_BITS: usize = 64;
pub(crate) const WIDTH: usize = LANES * LANE_BITS;
pub(crate) const ROUNDS: usize = 24;

/// Number of state lanes held by each trace row.
pub(crate) const LANES_PER_ROW: usize = 25;
pub(crate) const ROWS_PER_STATE: usize = LANES / LANES_PER_ROW;
pub(crate) const BITS_PER_ROW: usize = LANES_PER_ROW * LANE_BITS;

/// Sliced parities per row: the 320 (x,z) positions of a round, spread across
/// its ROWS_PER_STATE rows; position p lives at group-row p / C_PER_ROW,
/// column p % C_PER_ROW
pub(crate) const C_PER_ROW: usize = 320usize.div_ceil(ROWS_PER_STATE);

/// Two independent Keccak-f operations per slot: on round rows every state cell
/// is sliced, v = a + SLOT·b, holding the same-position values of both ops.
pub(crate) const OPS_PER_SLOT: usize = 2;
pub(crate) const SLOT: u8 = 8;

/// 2 input-bit groups + 25 sliced round groups + 2 output-bit groups
pub(crate) const GROUPS: usize = 2 + (1 + ROUNDS) + 2;
pub(crate) const CLOCKS: usize = GROUPS * ROWS_PER_STATE;

/// First rows of the slot's groups
pub(crate) const GROUP_IN_A: usize = 0;
pub(crate) const GROUP_IN_B: usize = ROWS_PER_STATE;
pub(crate) const GROUP_ROUND_0: usize = 2 * ROWS_PER_STATE;
pub(crate) const GROUP_OUT_A: usize = (3 + ROUNDS) * ROWS_PER_STATE;
pub(crate) const GROUP_OUT_B: usize = (4 + ROUNDS) * ROWS_PER_STATE;

/// χ-row S-box table: row = rc·16⁵ + Σ_x (tA_x + 4·tB_x)·16ˣ with tA,tB ∈ [0,3]
pub(crate) const CHI_TABLE_SIZE: u32 = 2 * 16u32.pow(5); // 2_097_152 = 2^21

/// The packed χ-row lookup input: rc·28⁵ + Σ_x (tA_x + 8·tB_x)·28ˣ
/// (base 28 since a sliced θ-output digit reaches 3 + 8·3 = 27)
pub(crate) const CHI_BASE: u32 = 28;
pub(crate) const CHI_SPAN: u32 = CHI_BASE.pow(5); // 17_210_368

/// xor5 table: row = Σ_k (sA_k + 6·sB_k)·36ᵏ with sA,sB ∈ [0,5], 3 positions per lookup
pub(crate) const XOR5_BATCH: usize = 3;
pub(crate) const XOR5_VALUES: u32 = 36;
pub(crate) const XOR5_TABLE_SIZE: u32 = XOR5_VALUES.pow(XOR5_BATCH as u32); // 46_656
/// xor5 lookups per trace row: batches of three c-column slots (mirrors the AIR)
pub(crate) const XOR5_GROUPS: usize = C_PER_ROW.div_ceil(XOR5_BATCH);

/// Keccak-f round constants
pub(crate) const RC: [u64; ROUNDS] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808A,
    0x8000000080008000,
    0x000000000000808B,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008A,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000A,
    0x000000008000808B,
    0x800000000000008B,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800A,
    0x800000008000000A,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

/// ρ rotation offsets r[x][y], from the standard (t+1)(t+2)/2 walk starting at (1,0)
pub(crate) const RHO_OFFSETS: [[usize; 5]; 5] = {
    let mut r = [[0usize; 5]; 5];
    let mut x = 1;
    let mut y = 0;
    let mut t = 0;
    while t < 24 {
        r[x][y] = ((t + 1) * (t + 2) / 2) % 64;
        let aux = y;
        y = (2 * x + 3 * y) % 5;
        x = aux;
        t += 1;
    }
    r
};
