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
//!
//! Every format is used the same way: values are quantized in groups, each
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
    DOT_F8_E2M1, DOT_F8_E4M3, DOT_F8_E4M3_FNUZ, DOT_F8_I8, dot_f8_bits, dot_f8_unit_shift,
    dot_f8_value,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Int8,
    E4m3,
    E4m3Fnuz,
    E2m1,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Int8, Format::E4m3, Format::E4m3Fnuz, Format::E2m1];

    /// The format's ID in `dot_f8` and in the model blob.
    pub fn id(self) -> u8 {
        match self {
            Format::Int8 => DOT_F8_I8,
            Format::E4m3 => DOT_F8_E4M3,
            Format::E4m3Fnuz => DOT_F8_E4M3_FNUZ,
            Format::E2m1 => DOT_F8_E2M1,
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
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }

    /// Bits per value: 8, or 4 for E2M1 (two values per byte, low nibble
    /// first).
    pub fn bits(self) -> usize {
        dot_f8_bits(self.id()) as usize
    }

    /// Bytes holding `n` values.
    pub fn bytes(self, n: usize) -> usize {
        n * self.bits() / 8
    }

    /// Values are integers in units of `2^-unit_shift()`.
    pub fn unit_shift(self) -> i32 {
        dot_f8_unit_shift(self.id()) as i32
    }

    /// The largest value, in the format's own scale (127, 448, 240 or 6).
    pub fn max_value(self) -> f32 {
        match self {
            Format::Int8 => 127.0,
            Format::E4m3 => 448.0,
            Format::E4m3Fnuz => 240.0,
            Format::E2m1 => 6.0,
        }
    }

    /// Value `i` of a vector stored in `bytes`, in units.
    pub fn units(self, bytes: &[u8], i: usize) -> i32 {
        dot_f8_value(self.id(), bytes, i)
    }

    /// (exponent bits, mantissa bits, bias, largest code) of a float format.
    fn layout(self) -> (i32, u32, i32, u32) {
        match self {
            Format::Int8 => unreachable!(),
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
/// llama2.c's `max|v| / 127` and `q = round(v / scale)`.
pub fn quantize_group(f: Format, v: &[f32], q: &mut [u8]) -> f32 {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        for f in Format::ALL {
            let mut q = [0u8; 8];
            let scale = quantize_group(f, &v, &mut q);
            assert_eq!(f.units(&q, 1) as f32 * scale, -0.25, "{f:?}");
            let largest = (0..8).map(|i| f.units(&q, i).abs()).max().unwrap();
            assert_eq!(largest as f32, f.max_value() * (1u32 << f.unit_shift()) as f32);
        }
    }
}
