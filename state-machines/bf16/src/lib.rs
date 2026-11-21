//! bfloat16 arithmetic and the layout of the `Bf16Table` lookup tables
//! (`state-machines/bf16/pil/bf16_table.pil`).
//!
//! Semantics: IEEE 754 arithmetic on the bfloat16 format (1 sign, 8 exponent and
//! 7 fraction bits), round to nearest, ties to even, subnormals kept. Any NaN
//! result is the canonical quiet NaN [`BF16_CANONICAL_NAN`].
//!
//! The exact result is computed in `f64` and rounded once to bfloat16:
//! - The product of two bfloat16 values has at most 16 significant bits and its
//!   exponent stays within the `f64` normal range, so it is exact in `f64`.
//! - The sum may be rounded in `f64`, but `f64` has 53 bits, at least 2 * 8 + 2,
//!   so rounding first to `f64` and then to bfloat16 gives the same value as
//!   rounding the exact sum directly (double rounding is innocuous). Every
//!   bfloat16 value is an `f64` normal, so no subnormal step breaks that.

/// Number of distinct bfloat16 encodings, and of A and B values in a table.
pub const BF16_VALUES: usize = 1 << 16;

/// Canonical quiet NaN, the bfloat16 counterpart of RISC-V's 0x7FC00000.
pub const BF16_CANONICAL_NAN: u16 = 0x7FC0;

/// bfloat16 operations proven by `Bf16Table`, with their opcodes in
/// `pil/operations.pil`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bf16Op {
    Add,
    Mul,
}

impl Bf16Op {
    pub const fn opcode(self) -> u8 {
        match self {
            Bf16Op::Add => 0xC2,
            Bf16Op::Mul => 0xC3,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Bf16Op::Add => "add",
            Bf16Op::Mul => "mul",
        }
    }

    /// Prefix of the AIR aliases in zisk.pil: `Bf16Add0`, `Bf16Add1`, ...
    pub const fn air_prefix(self) -> &'static str {
        match self {
            Bf16Op::Add => "Bf16Add",
            Bf16Op::Mul => "Bf16Mul",
        }
    }

    pub fn apply(self, a: u16, b: u16) -> u16 {
        match self {
            Bf16Op::Add => bf16_add(a, b),
            Bf16Op::Mul => bf16_mul(a, b),
        }
    }
}

/// Exact conversion of a bfloat16 encoding to `f64`.
pub fn bf16_to_f64(x: u16) -> f64 {
    f32::from_bits((x as u32) << 16) as f64
}

/// Rounds an `f64` to bfloat16, to nearest with ties to even.
///
/// Every finite `x` with `|x| >= 2^-1022` is handled exactly; smaller non-zero
/// values never come out of an operation on two bfloat16 values.
pub fn f64_to_bf16(x: f64) -> u16 {
    if x.is_nan() {
        return BF16_CANONICAL_NAN;
    }
    let sign: u16 = if x.is_sign_negative() { 0x8000 } else { 0 };
    if x.is_infinite() {
        return sign | 0x7F80;
    }
    if x == 0.0 {
        return sign;
    }

    // Unbiased exponent of |x|, floor(log2 |x|). x is an f64 normal.
    let exp = ((x.to_bits() >> 52) & 0x7FF) as i32 - 1023;
    // Weight of the last fraction bit of the bfloat16 grid around x: 7 fraction
    // bits for normals, and the fixed 2^-133 step for subnormals (exp < -126).
    let ulp_exp = exp.max(-126) - 7;
    let scaled = x.abs() * 2f64.powi(-ulp_exp); // exact, power of two scaling
    let rounded = scaled.round_ties_even() * 2f64.powi(ulp_exp); // exact again

    // Largest finite bfloat16 is (2 - 2^-7) * 2^127; rounding past it overflows.
    if rounded >= 2f64.powi(128) {
        return sign | 0x7F80;
    }
    // rounded is on the bfloat16 grid, so it is an f32 with the low 16 bits zero.
    let bits = (rounded as f32).to_bits();
    debug_assert_eq!(bits & 0xFFFF, 0);
    sign | (bits >> 16) as u16
}

pub fn bf16_add(a: u16, b: u16) -> u16 {
    f64_to_bf16(bf16_to_f64(a) + bf16_to_f64(b))
}

pub fn bf16_mul(a: u16, b: u16) -> u16 {
    f64_to_bf16(bf16_to_f64(a) * bf16_to_f64(b))
}

/// Split of one operation's table into `Bf16Table` instances. Must match the
/// instantiation in zisk.pil (`BF16_TABLE_N`, `BF16_TABLE_A_COUNT`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bf16TableLayout {
    /// Rows of each instance, a power of two >= 2^16.
    pub n: usize,
    /// A values covered by each instance, a multiple of `n / 2^16`.
    pub a_count: usize,
}

/// Where the lookup of one (a, b) pair lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bf16TableLocation {
    pub instance: usize,
    pub block: usize,
    pub row: usize,
}

impl Bf16TableLayout {
    pub fn new(n: usize, a_count: usize) -> Result<Self, String> {
        if !n.is_power_of_two() || n < BF16_VALUES {
            return Err(format!("N = {n} must be a power of two >= 2^16"));
        }
        let layout = Self { n, a_count };
        if a_count == 0 || a_count % layout.a_x_col() != 0 {
            return Err(format!(
                "A_count = {a_count} must be a positive multiple of N / 2^16 = {}",
                layout.a_x_col()
            ));
        }
        if BF16_VALUES % a_count != 0 {
            return Err(format!(
                "A_count = {a_count} must divide 2^16 so the instances cover every A"
            ));
        }
        Ok(layout)
    }

    /// Distinct A values in one column of N rows.
    pub const fn a_x_col(&self) -> usize {
        self.n / BF16_VALUES
    }

    /// Lookups per row, i.e. C columns and multiplicities per instance.
    pub const fn blocks(&self) -> usize {
        self.a_count / self.a_x_col()
    }

    pub const fn instances(&self) -> usize {
        BF16_VALUES / self.a_count
    }

    pub const fn from_a(&self, instance: usize) -> usize {
        instance * self.a_count
    }

    pub fn locate(&self, a: u16, b: u16) -> Bf16TableLocation {
        let a = a as usize;
        let local_a = a % self.a_count;
        Bf16TableLocation {
            instance: a / self.a_count,
            block: local_a / self.a_x_col(),
            row: (local_a % self.a_x_col()) * BF16_VALUES + b as usize,
        }
    }

    /// Inverse of [`locate`](Self::locate): the (a, b) of one table cell.
    pub fn operands(&self, instance: usize, block: usize, row: usize) -> (u16, u16) {
        let a = self.from_a(instance) + block * self.a_x_col() + row / BF16_VALUES;
        (a as u16, (row % BF16_VALUES) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: u16 = 0x3F80;
    const TWO: u16 = 0x4000;
    const INF: u16 = 0x7F80;
    const NEG_INF: u16 = 0xFF80;
    const MAX: u16 = 0x7F7F;
    const MIN_SUB: u16 = 0x0001;

    #[test]
    fn basic_values() {
        assert_eq!(bf16_add(ONE, ONE), TWO);
        assert_eq!(bf16_mul(TWO, TWO), 0x4080); // 4.0
        assert_eq!(bf16_add(ONE, 0xBF80), 0x0000); // 1 + -1 = +0
        assert_eq!(bf16_add(0x8000, 0x8000), 0x8000); // -0 + -0 = -0
        assert_eq!(bf16_add(0x0000, 0x8000), 0x0000); // +0 + -0 = +0
        assert_eq!(bf16_mul(0x8000, ONE), 0x8000);
    }

    #[test]
    fn ties_to_even() {
        // 1 + 2^-8 is exactly halfway between 1 and 1 + 2^-7: ties to 1 (even).
        assert_eq!(bf16_add(ONE, 0x3B80), ONE);
        // (1 + 2^-7) + 2^-8 is halfway between 1 + 2^-7 and 1 + 2^-6: ties to the even one.
        assert_eq!(bf16_add(0x3F81, 0x3B80), 0x3F82);
        // Just above the tie rounds up: 2^-8 + 2^-15 > half an ulp of 1.
        assert_eq!(bf16_add(ONE, 0x3B81), 0x3F81);
    }

    #[test]
    fn special_values() {
        assert_eq!(bf16_add(INF, NEG_INF), BF16_CANONICAL_NAN);
        assert_eq!(bf16_mul(INF, 0x0000), BF16_CANONICAL_NAN);
        assert_eq!(bf16_add(0xFFC1, ONE), BF16_CANONICAL_NAN); // any NaN payload in
        assert_eq!(bf16_add(MAX, MAX), INF);
        assert_eq!(bf16_mul(MAX, 0xC000), NEG_INF);
        assert_eq!(bf16_add(INF, ONE), INF);
    }

    #[test]
    fn subnormals() {
        assert_eq!(bf16_add(MIN_SUB, MIN_SUB), 0x0002);
        // 2^-133 * 0.5 is a tie between 0 and 2^-133: ties to 0.
        assert_eq!(bf16_mul(MIN_SUB, 0x3F00), 0x0000);
        // 2^-133 * 0.75 rounds up to 2^-133.
        assert_eq!(bf16_mul(MIN_SUB, 0x3F40), MIN_SUB);
        // Smallest normal minus the smallest subnormal is the largest subnormal.
        assert_eq!(bf16_add(0x0080, 0x8001), 0x007F);
    }

    /// Cross-check against f32 arithmetic plus round-to-nearest-even, which is
    /// also exact here: away from the subnormal range f32 has 24 >= 2 * 8 + 2 bits.
    #[test]
    fn matches_f32_on_normals() {
        fn f32_to_bf16(x: f32) -> u16 {
            let bits = x.to_bits();
            let round = 0x7FFF + ((bits >> 16) & 1);
            ((bits + round) >> 16) as u16
        }
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut checked = 0;
        while checked < 1_000_000 {
            let r = next();
            let (a, b) = (r as u16, (r >> 16) as u16);
            let (fa, fb) = (bf16_to_f64(a) as f32, bf16_to_f64(b) as f32);
            for (op, exact) in [(Bf16Op::Add, fa + fb), (Bf16Op::Mul, fa * fb)] {
                if !exact.is_normal() || exact.abs() >= f32::MAX / 2.0 {
                    continue;
                }
                assert_eq!(op.apply(a, b), f32_to_bf16(exact), "{op:?} {a:#06x} {b:#06x}");
                checked += 1;
            }
        }
    }

    #[test]
    fn layout_round_trip() {
        let layout = Bf16TableLayout::new(1 << 22, 512).unwrap();
        assert_eq!((layout.a_x_col(), layout.blocks(), layout.instances()), (64, 8, 128));
        for (a, b) in [(0u16, 0u16), (63, 65535), (64, 1), (511, 7), (512, 0), (0xFFFF, 0xFFFF)] {
            let loc = layout.locate(a, b);
            assert!(
                loc.instance < layout.instances()
                    && loc.block < layout.blocks()
                    && loc.row < layout.n
            );
            assert_eq!(layout.operands(loc.instance, loc.block, loc.row), (a, b));
        }
        assert!(Bf16TableLayout::new(1 << 15, 1).is_err());
        assert!(Bf16TableLayout::new(1 << 22, 96).is_err());
        assert!(Bf16TableLayout::new(1 << 22, 192).is_err()); // multiple of 64 but does not divide 2^16
    }
}
