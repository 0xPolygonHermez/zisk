//! Element formats of the experimental `dot_f8` precompile: int8 and the 8-bit (and 4-bit)
//! floats most used by AI models.
//!
//! Every value of these formats is an integer number of units `2^-unit_shift`, so `dot_f8`
//! decodes each element to an integer, multiplies and adds exactly: there is no rounding. Every
//! product is below 2^36, so the `i64` sums are exact for vectors of up to [`DOT_F8_MAX_N`] values.
//!
//! | ID | Format | Bits | Unit | Largest | Notes |
//! |---|---|---|---|---|---|
//! | 0 | int8 | 8 | 1 | 127 | two's complement |
//! | 1 | E4M3 (OCP FP8, "E4M3FN") | 8 | 2^-9 | 448 | bias 7, no infinities, NaN `S.1111.111` |
//! | 2 | E4M3FNUZ (AMD, Graphcore) | 8 | 2^-10 | 240 | bias 8, no negative zero, NaN `0x80` |
//! | 3 | E2M1 (FP4: MXFP4, NVFP4 elements) | 4 | 2^-1 | 6 | bias 1, two per byte, low nibble first |
//!
//! NaN codes decode to 0, so every code has a value and the operation never fails.

/// Signed int8.
pub const DOT_F8_I8: u8 = 0;
/// OCP FP8 E4M3 (E4M3FN).
pub const DOT_F8_E4M3: u8 = 1;
/// FP8 E4M3FNUZ.
pub const DOT_F8_E4M3_FNUZ: u8 = 2;
/// FP4 E2M1, two values per byte, low nibble first.
pub const DOT_F8_E2M1: u8 = 3;
/// Number of formats; valid IDs are below it.
pub const DOT_F8_FORMATS: u8 = 4;

/// The largest vector length for which the `i64` sums cannot overflow: every product is below
/// 2^36.
pub const DOT_F8_MAX_N: u64 = 1 << 26;

/// Each value of format `fmt` is an integer number of units `2^-dot_f8_unit_shift(fmt)`.
pub const fn dot_f8_unit_shift(fmt: u8) -> u32 {
    match fmt {
        DOT_F8_I8 => 0,
        DOT_F8_E4M3 => 9,
        DOT_F8_E4M3_FNUZ => 10,
        DOT_F8_E2M1 => 1,
        _ => panic!("dot_f8: unknown format"),
    }
}

/// Bits per value of format `fmt`: 8, or 4 for E2M1.
pub const fn dot_f8_bits(fmt: u8) -> u32 {
    if fmt == DOT_F8_E2M1 {
        4
    } else {
        8
    }
}

/// The largest absolute value of format `fmt`, in its units.
pub const fn dot_f8_max_units(fmt: u8) -> i32 {
    match fmt {
        DOT_F8_I8 => 127,
        DOT_F8_E4M3 => 448 << 9,
        DOT_F8_E4M3_FNUZ => 240 << 10,
        DOT_F8_E2M1 => 6 << 1,
        _ => panic!("dot_f8: unknown format"),
    }
}

/// The value of code `c` of format `fmt`, in its units. For E2M1 only the low 4 bits are used.
pub const fn dot_f8_decode_code(fmt: u8, c: u8) -> i32 {
    match fmt {
        DOT_F8_I8 => c as i8 as i32,
        DOT_F8_E4M3 => {
            let (e, m) = ((c >> 3) & 15, (c & 7) as i32);
            let mag = if e == 15 && m == 7 {
                0 // NaN
            } else if e == 0 {
                m
            } else {
                (8 + m) << (e - 1)
            };
            if c & 0x80 != 0 {
                -mag
            } else {
                mag
            }
        }
        DOT_F8_E4M3_FNUZ => {
            if c == 0x80 {
                return 0; // NaN
            }
            let (e, m) = ((c >> 3) & 15, (c & 7) as i32);
            let mag = if e == 0 { m } else { (8 + m) << (e - 1) };
            if c & 0x80 != 0 {
                -mag
            } else {
                mag
            }
        }
        DOT_F8_E2M1 => {
            let (e, m) = ((c >> 1) & 3, (c & 1) as i32);
            let mag = if e == 0 { m } else { (2 + m) << (e - 1) };
            if c & 8 != 0 {
                -mag
            } else {
                mag
            }
        }
        _ => panic!("dot_f8: unknown format"),
    }
}

const fn decode_tables() -> [[i32; 256]; DOT_F8_FORMATS as usize] {
    let mut t = [[0i32; 256]; DOT_F8_FORMATS as usize];
    let mut f = 0;
    while f < DOT_F8_FORMATS {
        let mut c = 0;
        while c < 256 {
            // E2M1 entries 16..256 repeat the 16 codes, so a byte can be looked up by nibble.
            let code = if f == DOT_F8_E2M1 { (c & 15) as u8 } else { c as u8 };
            t[f as usize][c] = dot_f8_decode_code(f, code);
            c += 1;
        }
        f += 1;
    }
    t
}

/// `DOT_F8_DECODE[fmt][code]`: the value of each code in its format's units. For E2M1, index
/// with a nibble (entries repeat every 16).
pub static DOT_F8_DECODE: [[i32; 256]; DOT_F8_FORMATS as usize] = decode_tables();

/// Value `i` of a vector of format `fmt` stored at `bytes`.
#[inline(always)]
pub fn dot_f8_value(fmt: u8, bytes: &[u8], i: usize) -> i32 {
    if fmt == DOT_F8_E2M1 {
        let b = bytes[i / 2];
        DOT_F8_DECODE[fmt as usize][(if i & 1 == 0 { b & 15 } else { b >> 4 }) as usize]
    } else {
        DOT_F8_DECODE[fmt as usize][bytes[i] as usize]
    }
}

/// Packs the formats of the two vectors into the `formats` word of the parameter block.
pub const fn dot_f8_formats(x: u8, y: u8) -> u64 {
    x as u64 | (y as u64) << 8
}

/// The reference semantics of `dot_f8`, on byte slices: returns `Σ x[i]·y[i]` over `n` values
/// in units `2^-(unit_shift(fx) + unit_shift(fy))`, and writes the sum of each block of `block`
/// values to `out` when given. Checks nothing: the callers validate the parameters.
pub fn dot_f8_reference(
    fx: u8,
    x: &[u8],
    fy: u8,
    y: &[u8],
    n: usize,
    block: usize,
    mut out: Option<&mut [i64]>,
) -> i64 {
    let mut total = 0i64;
    for b in 0..n / block {
        let mut acc = 0i64;
        for i in b * block..(b + 1) * block {
            acc += dot_f8_value(fx, x, i) as i64 * dot_f8_value(fy, y, i) as i64;
        }
        if let Some(out) = out.as_deref_mut() {
            out[b] = acc;
        }
        total += acc;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(fmt: u8, c: u8) -> f64 {
        dot_f8_decode_code(fmt, c) as f64 / (1u64 << dot_f8_unit_shift(fmt)) as f64
    }

    #[test]
    fn known_values() {
        assert_eq!(value(DOT_F8_I8, 0x80), -128.0);
        assert_eq!(value(DOT_F8_E4M3, 0x01), 1.0 / 512.0);
        assert_eq!(value(DOT_F8_E4M3, 0x08), 1.0 / 64.0);
        assert_eq!(value(DOT_F8_E4M3, 0x38), 1.0);
        assert_eq!(value(DOT_F8_E4M3, 0x7e), 448.0);
        assert_eq!(value(DOT_F8_E4M3, 0xfe), -448.0);
        assert_eq!(value(DOT_F8_E4M3, 0x7f), 0.0);
        assert_eq!(value(DOT_F8_E4M3_FNUZ, 0x40), 1.0); // e = 8 = bias
        assert_eq!(value(DOT_F8_E4M3_FNUZ, 0x7f), 240.0);
        assert_eq!(value(DOT_F8_E4M3_FNUZ, 0x01), 1.0 / 1024.0);
        assert_eq!(value(DOT_F8_E4M3_FNUZ, 0x80), 0.0);
        let fp4: [f64; 8] = [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0];
        for (c, v) in fp4.iter().enumerate() {
            assert_eq!(value(DOT_F8_E2M1, c as u8), *v);
            assert_eq!(value(DOT_F8_E2M1, c as u8 | 8), -*v);
        }
    }

    #[test]
    fn largest_values_and_products_fit() {
        for f in 0..DOT_F8_FORMATS {
            let max = (0..=255u8).map(|c| dot_f8_decode_code(f, c).abs()).max().unwrap();
            assert_eq!(max, if f == DOT_F8_I8 { 128 } else { dot_f8_max_units(f) });
            assert!((max as i64) * (max as i64) < 1 << 36);
        }
    }

    #[test]
    fn reference_blocks_and_nibbles() {
        // x: E2M1 values 1, 6, -0.5, 2 (codes 2, 7, 9, 4), packed low nibble first
        let x = [0x72u8, 0x49];
        // y: E4M3 values 1, 0.5, 2, -1
        let y = [0x38u8, 0x30, 0x40, 0xb8];
        let mut out = [0i64; 2];
        let total = dot_f8_reference(DOT_F8_E2M1, &x, DOT_F8_E4M3, &y, 4, 2, Some(&mut out));
        let unit = (1u64 << (1 + 9)) as f64;
        assert_eq!(out[0] as f64 / unit, 1.0 + 3.0);
        assert_eq!(out[1] as f64 / unit, -1.0 - 2.0);
        assert_eq!(total as f64 / unit, 1.0);
    }
}
