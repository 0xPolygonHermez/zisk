//! The number formats of the weights and activations in the matrix
//! multiplications: int8, as llama2.c, and the 8-bit and 4-bit floats most
//! used by AI models, which the experimental `dot_f8` precompile also takes
//! (see `zisk_definitions::dot_f8` for their decoding):
//!
//! | Format | Bits | Largest value | Used by |
//! |---|---|---|---|
//! | [`Format::Int8`] | 8 | 127 | llama2.c's `runq.c` |
//! | [`Format::E4m3`] (OCP FP8) | 8 | 448 | DeepSeek-V3, Qwen3 and Llama FP8 checkpoints |
//! | [`Format::E4m3Fnuz`] | 8 | 240 | AMD MI300 |
//! | [`Format::E2m1`] (FP4) | 4 | 6 | MXFP4 (gpt-oss) and NVFP4 elements |
//! | [`Format::Bf16`] | 16 | 3.4·10^38 | most published models (SmolLM2, Qwen3, Llama 3) |
//! | [`Format::Mxfp4`] | 4.25 | 6·2^127 | gpt-oss; OCP Microscaling |
//! | [`Format::Nvfp4`] | 4.5 | 6·448·(tensor scale) | NVIDIA's Blackwell checkpoints |
//!
//! MXFP4 and NVFP4 are weight formats used with BF16 activations, through the same
//! precompile: E2M1 values with a scale per block of values, an E8M0 power of two per 32
//! (MXFP4) or an E4M3 value per 16 plus an `f32` per tensor (NVFP4). See
//! [`crate::checkpoint`] for how they are quantized and stored.
//!
//! BF16 is used apart: it has the range of `f32`, so it needs no scales. Its
//! values are the `f32` values rounded to nearest even, and the matrix
//! multiplications use the experimental `dot_bf16` precompile, whose result
//! is the exact dot product rounded once to `f32` (see
//! `zisk_definitions::dot_bf16`). The rest of this description is about the
//! other formats.
//!
//! Every other format is used the same way: values are quantized in groups, each
//! with one `f32` scale that maps the group's largest absolute value to the
//! format's largest value. That is llama2.c's recipe for int8, and the block
//! scaling of FP8 and FP4 checkpoints.
//!
//! Each value of a format decodes to an integer number of units (1 for int8,
//! 2^-9 for E4M3, ...). Dot products are computed exactly on these integers,
//! so the scales are stored per unit: `value = units · scale`.
//!
//! The conversion from `f32` works on its bits with integer instructions
//! only, rounding to the nearest value, ties to even, as hardware does. int8
//! keeps llama2.c's `roundf`, which rounds ties away from zero.

use zisk_definitions::{
    DOT_F8_E2M1, DOT_F8_E4M3, DOT_F8_E4M3_FNUZ, DOT_F8_I8, bf16_from_f32, dot_f8_bits,
    dot_f8_unit_shift, dot_f8_value,
};

/// The model blob's IDs for [`Format::Bf16`], [`Format::Mxfp4`] and [`Format::Nvfp4`],
/// outside `dot_f8`'s IDs.
pub const BF16_ID: u8 = 16;
pub const MXFP4_ID: u8 = 17;
pub const NVFP4_ID: u8 = 18;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Int8,
    E4m3,
    E4m3Fnuz,
    E2m1,
    Bf16,
    Mxfp4,
    Nvfp4,
}

impl Format {
    pub const ALL: [Format; 7] = [
        Format::Int8,
        Format::E4m3,
        Format::E4m3Fnuz,
        Format::E2m1,
        Format::Bf16,
        Format::Mxfp4,
        Format::Nvfp4,
    ];

    /// The formats quantized in groups with scales, which `dot_f8` takes.
    pub const SCALED: [Format; 4] = [Format::Int8, Format::E4m3, Format::E4m3Fnuz, Format::E2m1];

    /// The format's ID in the model blob, which is also its ID in `dot_f8`
    /// for the scaled formats.
    pub fn id(self) -> u8 {
        match self {
            Format::Int8 => DOT_F8_I8,
            Format::E4m3 => DOT_F8_E4M3,
            Format::E4m3Fnuz => DOT_F8_E4M3_FNUZ,
            Format::E2m1 => DOT_F8_E2M1,
            Format::Bf16 => BF16_ID,
            Format::Mxfp4 => MXFP4_ID,
            Format::Nvfp4 => NVFP4_ID,
        }
    }

    pub fn from_id(id: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.id() as u32 == id)
    }

    pub fn name(self) -> &'static str {
        match self {
            Format::Int8 => "int8",
            Format::E4m3 => "e4m3",
            Format::E4m3Fnuz => "e4m3fnuz",
            Format::E2m1 => "e2m1",
            Format::Bf16 => "bf16",
            Format::Mxfp4 => "mxfp4",
            Format::Nvfp4 => "nvfp4",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }

    /// Bits per value: 8, 4 for E2M1 (two values per byte, low nibble first)
    /// or 16 for BF16.
    pub fn bits(self) -> usize {
        match self {
            Format::Bf16 => 16,
            Format::Mxfp4 | Format::Nvfp4 => 4,
            _ => dot_f8_bits(self.id()) as usize,
        }
    }

    /// Whether the format is FP4 weights with block scales, used with BF16 activations.
    pub fn is_fp4_weights(self) -> bool {
        matches!(self, Format::Mxfp4 | Format::Nvfp4)
    }

    /// Whether the format goes through the `dot_bf16` precompile (BF16, MXFP4, NVFP4) rather
    /// than `dot_f8`.
    pub fn uses_bf16(self) -> bool {
        matches!(self, Format::Bf16 | Format::Mxfp4 | Format::Nvfp4)
    }

    /// The `flags` of a `dot_bf16` call with these FP4 weights.
    pub fn dot_bf16_flags(self) -> u64 {
        match self {
            Format::Mxfp4 => zisk_definitions::DOT_BF16_FLAGS_MXFP4,
            Format::Nvfp4 => zisk_definitions::DOT_BF16_FLAGS_NVFP4,
            _ => panic!("not FP4 weights"),
        }
    }

    /// Values per block scale of FP4 weights: 32 (MXFP4) or 16 (NVFP4).
    pub fn fp4_block(self) -> usize {
        zisk_definitions::dot_bf16_fp4_block(self.dot_bf16_flags())
    }

    /// Bytes holding `n` values.
    pub fn bytes(self, n: usize) -> usize {
        n * self.bits() / 8
    }

    /// Values are integers in units of `2^-unit_shift()` (not BF16).
    pub fn unit_shift(self) -> i32 {
        assert!(!self.uses_bf16(), "{self:?} values are not integers");
        dot_f8_unit_shift(self.id()) as i32
    }

    /// The largest value, in the format's own scale (127, 448, 240 or 6);
    /// not BF16, which has no scales.
    pub fn max_value(self) -> f32 {
        match self {
            Format::Int8 => 127.0,
            Format::E4m3 => 448.0,
            Format::E4m3Fnuz => 240.0,
            Format::E2m1 => 6.0,
            Format::Bf16 | Format::Mxfp4 | Format::Nvfp4 => panic!("{self:?} has no group scales"),
        }
    }

    /// Value `i` of a vector stored in `bytes`, in units (not BF16).
    pub fn units(self, bytes: &[u8], i: usize) -> i32 {
        assert!(!self.uses_bf16(), "{self:?} values are not integers");
        dot_f8_value(self.id(), bytes, i)
    }

    /// (exponent bits, mantissa bits, bias, largest code) of a float format.
    fn layout(self) -> (i32, u32, i32, u32) {
        match self {
            Format::Int8 | Format::Bf16 | Format::Mxfp4 | Format::Nvfp4 => unreachable!(),
            Format::E4m3 => (4, 3, 7, 0x7e),
            Format::E4m3Fnuz => (4, 3, 8, 0x7f),
            Format::E2m1 => (2, 1, 1, 0x7),
        }
    }

    /// The code of a float format nearest to a finite `f32` given as bits,
    /// ties to even, saturating to the largest value. `f32` subnormals become
    /// 0. For E2M1 the code is in the low 4 bits.
    pub fn encode_float(self, bits: u32) -> u8 {
        let (ebits, mbits, bias, max_code) = self.layout();
        let sign_bit = 1u32 << (ebits as u32 + mbits);
        let exp = (bits >> 23) as i32 & 0xff;
        let mag = if exp == 0 {
            0
        } else {
            let sig = (bits & 0x7f_ffff) as u64 | 0x80_0000; // value sig · 2^(x - 23)
            let x = exp - 127;
            let code = if x >= 1 - bias {
                // Normal: keep mbits + 1 significant bits. A carry to
                // 2^(mbits+1) moves to the next exponent, and adding it to the
                // encoding does that by itself.
                let q = shr_round_even(sig, 23 - mbits) as i64;
                (((x + bias) as i64) << mbits) + q - (1 << mbits)
            } else {
                // Subnormal: in units 2^-(bias - 1 + mbits). A result of
                // 2^mbits is the smallest normal value's encoding, as it
                // should be.
                let sh = 23 - x - (bias - 1 + mbits as i32);
                if sh > 40 { 0 } else { shr_round_even(sig, sh as u32) as i64 }
            };
            code.min(max_code as i64) as u32
        };
        // No negative zero: E4M3FNUZ uses that code for NaN.
        if mag == 0 || bits >> 31 == 0 { mag as u8 } else { (mag | sign_bit) as u8 }
    }
}

/// `x >> sh`, rounded to the nearest integer, ties to even (`sh` in 1..=63).
fn shr_round_even(x: u64, sh: u32) -> u64 {
    let q = x >> sh;
    let rem = x & ((1u64 << sh) - 1);
    let half = 1u64 << (sh - 1);
    if rem > half || (rem == half && q & 1 == 1) { q + 1 } else { q }
}

/// Quantizes `v` (one group) into `q`, the bytes of its values, and returns
/// the group's scale per unit: `v[i] ≈ units(q, i) · scale`. For int8 this is
/// llama2.c's `max|v| / 127` and `q = round(v / scale)`. For BF16 the values
/// are rounded to BF16 (little-endian) and the scale is 1.
pub fn quantize_group(f: Format, v: &[f32], q: &mut [u8]) -> f32 {
    if f == Format::Bf16 {
        for (qi, &x) in q.chunks_exact_mut(2).zip(v) {
            qi.copy_from_slice(&bf16_from_f32(x).to_le_bytes());
        }
        return 1.0;
    }
    let vmax = v.iter().fold(0.0f32, |m, &x| m.max(libm::fabsf(x)));
    let scale = vmax / f.max_value();
    match f {
        Format::Int8 => {
            for (qi, &x) in q.iter_mut().zip(v) {
                let qv = if scale == 0.0 { 0.0 } else { libm::roundf(x / scale) };
                *qi = qv as i8 as u8;
            }
        }
        Format::E4m3 | Format::E4m3Fnuz => {
            for (qi, &x) in q.iter_mut().zip(v) {
                *qi = if scale == 0.0 { 0 } else { f.encode_float((x / scale).to_bits()) };
            }
        }
        Format::Bf16 | Format::Mxfp4 | Format::Nvfp4 => unreachable!(),
        Format::E2m1 => {
            q[..v.len() / 2].fill(0);
            for (i, &x) in v.iter().enumerate() {
                let c = if scale == 0.0 { 0 } else { f.encode_float((x / scale).to_bits()) };
                q[i / 2] |= c << (4 * (i & 1));
            }
        }
    }
    // The scale of one unit: scale · 2^-unit_shift, exact (a power of two).
    scale / (1u32 << f.unit_shift()) as f32
}

/// The E2M1 code (low 4 bits) nearest to `v`, ties to even, saturating to ±6.
fn e2m1_code(v: f32) -> u8 {
    Format::E2m1.encode_float(v.to_bits())
}

/// Quantizes one row `w` to MXFP4 (OCP Microscaling): per block of 32 values, the scale is
/// `2^X` with `X = floor(log2(max|w|)) - 2` (the largest E2M1 exponent), stored as the E8M0
/// byte `X + 127`, and the values are `w / 2^X` rounded to E2M1, ties to even, saturating at
/// ±6. Writes the packed values to `q` and one scale byte per block to `scales`.
pub fn quantize_mxfp4_row(w: &[f32], q: &mut [u8], scales: &mut [u8]) {
    for (b, block) in w.chunks_exact(32).enumerate() {
        let amax = block.iter().fold(0u32, |m, v| m.max(v.to_bits() & 0x7fff_ffff));
        // floor(log2(amax)) from the exponent bits; f32 subnormals and zero take the smallest
        let e = (amax >> 23) as i32;
        let x = if e == 0 { -127 } else { (e - 127 - 2).clamp(-127, 127) };
        scales[b] = (x + 127) as u8;
        let inv = libm::ldexpf(1.0, -x); // 2^-X, exact for X in -127..=127
        for (k, &v) in block.iter().enumerate() {
            let i = 32 * b + k;
            q[i / 2] |= e2m1_code(v * inv) << (4 * (i & 1));
        }
    }
}

/// The per-tensor `f32` scale of NVFP4: `max|w| / (6 · 448)` over the whole tensor, so that the
/// block scales fit E4M3; 1 for an all-zero tensor.
pub fn nvfp4_tensor_scale(w: &[f32]) -> f32 {
    let amax = w.iter().fold(0.0f32, |m, &v| m.max(libm::fabsf(v)));
    if amax == 0.0 { 1.0 } else { amax / (6.0 * 448.0) }
}

/// Quantizes one row `w` to NVFP4 with the tensor scale `t`: per block of 16 values, the scale
/// is `max|w| / 6 / t` rounded to E4M3, and the values are `w / (scale · t)` rounded to E2M1,
/// ties to even, saturating at ±6. Writes the packed values to `q` and one E4M3 byte per block
/// to `scales`.
pub fn quantize_nvfp4_row(w: &[f32], t: f32, q: &mut [u8], scales: &mut [u8]) {
    for (b, block) in w.chunks_exact(16).enumerate() {
        let amax = block.iter().fold(0.0f32, |m, &v| m.max(libm::fabsf(v)));
        let code = Format::E4m3.encode_float((amax / 6.0 / t).to_bits());
        scales[b] = code;
        let s = Format::E4m3.units(&[code], 0) as f32 / 512.0 * t;
        for (k, &v) in block.iter().enumerate() {
            let i = 16 * b + k;
            let c = if s == 0.0 { 0 } else { e2m1_code(v / s) };
            q[i / 2] |= c << (4 * (i & 1));
        }
    }
}

/// The value of weight `i` of an FP4 row: `e2m1 · block scale`, without NVFP4's tensor scale.
pub fn fp4_weight(f: Format, q: &[u8], scales: &[u8], i: usize) -> f32 {
    let code = zisk_definitions::fp4_code(q, i);
    match zisk_definitions::fp4_weight_decode(f.dot_bf16_flags(), code, scales[i / f.fp4_block()]) {
        Some((neg, sig, exp)) => {
            let v = libm::ldexpf(sig as f32, exp);
            if neg { -v } else { v }
        }
        None => f32::NAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    const FLOATS: [Format; 3] = [Format::E4m3, Format::E4m3Fnuz, Format::E2m1];

    fn value(f: Format, c: u8) -> f64 {
        f.units(&[c], 0) as f64 / (1u64 << f.unit_shift()) as f64
    }

    fn codes(f: Format) -> impl Iterator<Item = u8> {
        let (ebits, mbits, _, max_code) = f.layout();
        let sign = 1u32 << (ebits as u32 + mbits);
        (0..=max_code).chain((1..=max_code).map(move |c| c | sign)).map(|c| c as u8)
    }

    #[test]
    fn every_code_round_trips() {
        for f in FLOATS {
            for c in codes(f) {
                let v = value(f, c) as f32;
                assert_eq!(f.encode_float(v.to_bits()), c, "{f:?} code {c:#04x} ({v})");
            }
        }
    }

    /// The nearest code by exhaustive search, ties to the even code.
    fn nearest(f: Format, v: f64) -> u8 {
        let (ebits, mbits, _, max_code) = f.layout();
        let mut best = 0u8;
        for c in 0..=max_code as u8 {
            let (d, db) = ((value(f, c) - v.abs()).abs(), (value(f, best) - v.abs()).abs());
            if d < db || (d == db && c & 1 == 0) {
                best = c;
            }
        }
        if v < 0.0 && best != 0 { best | (1 << (ebits as u32 + mbits)) as u8 } else { best }
    }

    #[test]
    fn rounds_to_nearest_even_and_saturates() {
        for f in FLOATS {
            let mut s: u64 = 12345;
            for i in 0..20000 {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let mag = ((s >> 11) as f64 / (1u64 << 53) as f64) * 2.0f64.powi((i % 26) - 14);
                let v = if s & 1 == 1 { -mag } else { mag } as f32;
                assert_eq!(f.encode_float(v.to_bits()), nearest(f, v as f64), "{f:?} {v}");
            }
        }
        // 1 + 1/16 is halfway between 1 and 1.125, and goes to the even 1.
        assert_eq!(Format::E4m3.encode_float(1.0625f32.to_bits()), 0x38);
        assert_eq!(Format::E4m3.encode_float(1000.0f32.to_bits()), 0x7e);
        assert_eq!(Format::E4m3Fnuz.encode_float(1000.0f32.to_bits()), 0x7f);
        assert_eq!(Format::E4m3Fnuz.encode_float((-1.0e-9f32).to_bits()), 0x00);
        // 2.5 is halfway between 2 and 3: ties to the even code, 2 (0b100).
        assert_eq!(Format::E2m1.encode_float(2.5f32.to_bits()), 0x4);
        assert_eq!(Format::E2m1.encode_float((-7.0f32).to_bits()), 0xf);
    }

    #[test]
    fn quantize_group_maps_the_largest_value_to_the_format_maximum() {
        let v = [0.003f32, -0.25, 0.1, 0.0, 0.2, 0.05, -0.01, 0.125];
        for f in Format::SCALED {
            let mut q = [0u8; 8];
            let scale = quantize_group(f, &v, &mut q);
            assert_eq!(f.units(&q, 1) as f32 * scale, -0.25, "{f:?}");
            let largest = (0..8).map(|i| f.units(&q, i).abs()).max().unwrap();
            assert_eq!(largest as f32, f.max_value() * (1u32 << f.unit_shift()) as f32);
        }
    }

    #[test]
    fn mxfp4_and_nvfp4_round_trip_their_grid() {
        // A block holding the E2M1 grid times 2^-5, and one with a different range
        let grid = [0.0f32, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0];
        let w: Vec<f32> = (0..64)
            .map(|i| {
                let g = grid[i % 8] * if i % 16 < 8 { 1.0 } else { -1.0 };
                if i < 32 { g / 32.0 } else { g * 1024.0 }
            })
            .collect();
        let (mut q, mut s) = (vec![0u8; 32], vec![0u8; 2]);
        quantize_mxfp4_row(&w, &mut q, &mut s);
        assert_eq!(s, [127 - 5, 127 + 10]);
        for i in 0..64 {
            assert_eq!(fp4_weight(Format::Mxfp4, &q, &s, i), w[i], "{i}");
        }
        // NVFP4: the largest block maps to 6 · its E4M3 scale
        let t = nvfp4_tensor_scale(&w);
        let (mut q, mut s) = (vec![0u8; 32], vec![0u8; 4]);
        quantize_nvfp4_row(&w, t, &mut q, &mut s);
        let max =
            (0..64).map(|i| fp4_weight(Format::Nvfp4, &q, &s, i).abs() * t).fold(0.0, f32::max);
        assert!((max - 6144.0).abs() / 6144.0 < 0.01, "{max}");
    }
}
