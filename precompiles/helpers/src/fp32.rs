//! Integer-only binary32 RNE semantics shared by ordered dot witness generation.
//! This module alone is software arithmetic, not an arithmetic proof.

const SIGN: u32 = 0x8000_0000;
const INF: u32 = 0x7f80_0000;
const QNAN: u32 = 0x7fc0_0000;
const FRAC: u32 = 0x007f_ffff;
const NX: u8 = 1;
const UF: u8 = 2;
const OF: u8 = 4;
const NV: u8 = 16;

/// Raw result bits and sticky RISC-V/SoftFloat flags (NX, UF, OF, DZ, NV).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fp32Result {
    pub bits: u32,
    pub flags: u8,
}

/// Maximum number of pairs in one bounded dot call; longer dots pass the result on.
pub use zisk_definitions::FLOATDOT_MAX_LEN;

fn nan(a: u32, b: u32) -> Option<Fp32Result> {
    if a & !SIGN > INF || b & !SIGN > INF {
        let signaling = |v: u32| v & !SIGN > INF && v & 0x0040_0000 == 0;
        Some(Fp32Result { bits: QNAN, flags: if signaling(a) || signaling(b) { NV } else { 0 } })
    } else {
        None
    }
}

/// Finite value = (-1)^sign * significand * 2^(exponent - 23).
fn finite(bits: u32) -> (u64, i32) {
    let exponent = (bits >> 23) & 255;
    ((bits & FRAC) as u64 | (u64::from(exponent != 0) << 23), exponent.max(1) as i32 - 127)
}

fn shift_jam(value: u64, shift: u32) -> u64 {
    match shift {
        0 => value,
        1..=63 => (value >> shift) | u64::from(value << (64 - shift) != 0),
        _ => u64::from(value != 0),
    }
}

/// Round a nonnegative significand scaled by 2^(exponent - 26), including sticky bit 0.
fn round(sign: u32, mut sig: u64, mut exponent: i32) -> Fp32Result {
    if sig == 0 {
        return Fp32Result { bits: sign, flags: 0 };
    }
    let high = 63 - sig.leading_zeros() as i32;
    if high > 26 {
        sig = shift_jam(sig, (high - 26) as u32);
    } else {
        sig <<= (26 - high) as u32;
    }
    exponent += high - 26;
    // SoftFloat detects tininess before reducing precision for a subnormal result.
    let tiny = exponent < -126 && (exponent < -127 || sig < (1 << 27) - 4);
    if exponent < -126 {
        sig = shift_jam(sig, (-126 - exponent) as u32);
        exponent = -126;
    }
    let tail = sig & 7;
    let mut rounded = (sig >> 3) + u64::from(tail > 4 || (tail == 4 && sig & 8 != 0));
    let mut flags = if tail != 0 { NX } else { 0 };
    if rounded >= 1 << 24 {
        rounded >>= 1;
        exponent += 1;
    }
    if exponent > 127 {
        return Fp32Result { bits: sign | INF, flags: OF | NX };
    }
    let normal = rounded >= 1 << 23;
    if tiny && flags != 0 {
        flags |= UF;
    }
    let exp_bits = if normal { ((exponent + 127) as u32) << 23 } else { 0 };
    Fp32Result { bits: sign | exp_bits | (rounded as u32 & FRAC), flags }
}

/// Correctly rounded binary32 addition, RNE, gradual underflow and canonical NaNs.
pub fn fp32_add(a: u32, b: u32) -> Fp32Result {
    if let Some(result) = nan(a, b) {
        return result;
    }
    let aa = a & !SIGN;
    let ab = b & !SIGN;
    if aa == INF || ab == INF {
        return if aa == INF && ab == INF && (a ^ b) & SIGN != 0 {
            Fp32Result { bits: QNAN, flags: NV }
        } else {
            Fp32Result { bits: if aa == INF { a } else { b }, flags: 0 }
        };
    }
    // Sorting magnitudes makes subtraction nonnegative, including all subnormals.
    let (large, small) = if aa >= ab { (a, b) } else { (b, a) };
    let (ma, ea) = finite(large);
    let (mb, eb) = finite(small);
    let lhs = ma << 3;
    let rhs = shift_jam(mb << 3, (ea - eb) as u32);
    if (a ^ b) & SIGN == 0 {
        round(large & SIGN, lhs + rhs, ea)
    } else {
        let difference = lhs - rhs;
        round(if difference == 0 { 0 } else { large & SIGN }, difference, ea)
    }
}

/// Correctly rounded binary32 multiplication; never fused with an addition.
pub fn fp32_mul(a: u32, b: u32) -> Fp32Result {
    if let Some(result) = nan(a, b) {
        return result;
    }
    let aa = a & !SIGN;
    let ab = b & !SIGN;
    let sign = (a ^ b) & SIGN;
    if aa == INF || ab == INF {
        return if aa == 0 || ab == 0 {
            Fp32Result { bits: QNAN, flags: NV }
        } else {
            Fp32Result { bits: sign | INF, flags: 0 }
        };
    }
    let (ma, ea) = finite(a);
    let (mb, eb) = finite(b);
    round(sign, ma * mb, ea + eb - 20)
}

/// Fold two equally sized bit-pattern slices in order, rounding each product and sum.
/// Empty calls preserve the initial bits and flags verbatim. No ambient FP state is used.
pub fn fp32_dot(x: &[u32], y: &[u32], mut initial: Fp32Result) -> Result<Fp32Result, &'static str> {
    if x.len() != y.len() || x.len() > FLOATDOT_MAX_LEN {
        return Err("floatdot requires equal lengths no greater than 4096");
    }
    if initial.flags & !31 != 0 {
        return Err("floatdot initial flags contain reserved bits");
    }
    for (&a, &b) in x.iter().zip(y) {
        let product = fp32_mul(a, b);
        let sum = fp32_add(initial.bits, product.bits);
        initial = Fp32Result { bits: sum.bits, flags: initial.flags | product.flags | sum.flags };
    }
    Ok(initial)
}

/// Exact BF16 embedding: the BF16 bits are the high half of the binary32 encoding.
/// Raises no flag; a signaling NaN stays signaling for the following multiply.
pub fn bf16_to_fp32(h: u16) -> u32 {
    u32::from(h) << 16
}

/// Exact FP16 widening. Subnormals become binary32 normals; NaN payload and quiet bit are
/// preserved, so a signaling NaN raises NV in the following multiply, not here.
pub fn fp16_to_fp32(h: u16) -> u32 {
    let sign = u32::from(h >> 15) << 31;
    let exponent = u32::from((h >> 10) & 31);
    let mantissa = u32::from(h & 0x3ff);
    match (exponent, mantissa) {
        (0, 0) => sign,
        (0, m) => {
            let k = 31 - m.leading_zeros();
            sign | ((103 + k) << 23) | (((m - (1 << k)) << (23 - k)) & FRAC)
        }
        (31, m) => sign | INF | (m << 13),
        (e, m) => sign | ((e + 112) << 23) | (m << 13),
    }
}
