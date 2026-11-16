use std::collections::HashMap;

use zisk_precomp_helpers::{
    bf16_to_fp32, fp16_to_fp32, fp32_dot, FloatDotCall, FloatDotFormat, Fp32Result,
};

const BF16: u64 = 1 << 8;
const FP16: u64 = 2 << 8;

/// Independent FP16 decoding through f64, which represents every FP16 value exactly.
fn fp16_value(h: u16) -> f64 {
    let sign = if h >> 15 == 1 { -1.0 } else { 1.0 };
    let (e, m) = (i32::from((h >> 10) & 31), f64::from(h & 0x3ff));
    sign * if e == 0 { m * 2f64.powi(-24) } else { (1024.0 + m) * 2f64.powi(e - 25) }
}

#[test]
fn fp16_widening_is_exact_for_every_encoding() {
    for h in 0..=u16::MAX {
        let bits = fp16_to_fp32(h);
        if (h >> 10) & 31 == 31 {
            // Infinity and NaN keep the sign, payload and quiet bit (9 -> 22).
            assert_eq!(
                bits,
                (u32::from(h >> 15) << 31) | 0x7f80_0000 | (u32::from(h & 0x3ff) << 13)
            );
        } else {
            let value = fp16_value(h);
            assert_eq!(f64::from(f32::from_bits(bits)), value, "{h:04x}");
            // Signed zeros stay signed; every nonzero finite value becomes an FP32 normal.
            assert_eq!(bits >> 31, u32::from(h >> 15));
            assert!(bits & 0x7fff_ffff == 0 || (bits >> 23) & 255 != 0, "{h:04x}");
        }
    }
    assert_eq!(fp16_to_fp32(0x0001), 0x3380_0000); // 2^-24
    assert_eq!(fp16_to_fp32(0x03ff), 0x387f_c000); // largest subnormal
    assert_eq!(fp16_to_fp32(0x7bff), 0x477f_e000); // 65504
    assert_eq!(fp16_to_fp32(0x7d00), 0x7fa0_0000); // signaling NaN stays signaling
    assert_eq!(fp16_to_fp32(0xfe00), 0xffc0_0000);
}

#[test]
fn bf16_embedding_is_the_high_half_for_every_encoding() {
    for h in 0..=u16::MAX {
        assert_eq!(bf16_to_fp32(h), u32::from(h) << 16);
    }
    // The same bits mean different values in the two 16-bit formats.
    assert_eq!(f32::from_bits(bf16_to_fp32(0x3c00)), 0.0078125);
    assert_eq!(f32::from_bits(fp16_to_fp32(0x3c00)), 1.0);
}

/// Sparse little-endian memory of 16-bit lanes; untouched lanes hold a NaN pattern.
struct Memory(HashMap<u32, u64>);

impl Memory {
    fn new() -> Self {
        Self(HashMap::new())
    }
    fn word(&self, address: u32) -> u64 {
        *self.0.get(&address).unwrap_or(&0x7fa1_7fa1_7fa1_7fa1)
    }
    fn put(&mut self, address: u32, h: u16) {
        let (word, shift) = (address & !7, 16 * ((address >> 1) & 3));
        let value = (self.word(word) & !(0xffff << shift)) | (u64::from(h) << shift);
        self.0.insert(word, value);
    }
    fn put_vector(&mut self, base: u32, stride: u32, values: &[u16]) {
        for (i, &h) in values.iter().enumerate() {
            self.put(base + 2 * stride * i as u32, h);
        }
    }
    /// Emulate the VM: decode the header, then read each enclosing word in payload order.
    fn payload(&self, descriptor: [u64; 5]) -> Vec<u64> {
        let mut header = descriptor.to_vec();
        header.push(0);
        let d = zisk_precomp_helpers::FloatDotCallHeader::decode(0x100, 1, &header)
            .unwrap()
            .descriptor();
        for address in
            d.word_addresses(d.x, d.stride_x).into_iter().chain(d.word_addresses(d.y, d.stride_y))
        {
            header.push(self.word(address));
        }
        header
    }
}

fn reference(format: FloatDotFormat, x: &[u16], y: &[u16], initial: Fp32Result) -> Fp32Result {
    let widen = |v: &[u16]| v.iter().map(|&h| format.widen(h)).collect::<Vec<_>>();
    fp32_dot(&widen(x), &widen(y), initial).unwrap()
}

#[test]
fn strided_bf16_selects_only_the_addressed_lanes() {
    let x = [0x3f80, 0x4000, 0xc040, 0x3f00, 0x0001]; // 1, 2, -3, 0.5, subnormal
    let y = [0x4040, 0x3f80, 0x3f80, 0x4080, 0x7f80]; // 3, 1, 1, 4, inf
    let mut memory = Memory::new();
    memory.put_vector(0x1002, 1, &x); // contiguous, starts in lane 1, crosses a word
    memory.put_vector(0x2006, 1024, &y); // V-like stride: 2048 bytes per element
    let payload = memory.payload([0x1002 | 1 << 32, 0x2006 | 1024 << 32, 5, 0, BF16]);
    let call = FloatDotCall::decode(0x100, 1, &payload).unwrap();
    let d = call.descriptor();
    assert_eq!((d.format, d.stride_x, d.stride_y), (FloatDotFormat::Bf16, 1, 1024));
    assert_eq!(call.x_words().len(), 5);
    let addresses: Vec<_> = call.memory_reads().iter().map(|r| r.address).collect();
    assert_eq!(
        addresses,
        [
            0x100, 0x108, 0x110, 0x118, 0x120, 0x1000, 0x1000, 0x1000, 0x1008, 0x1008, 0x2000,
            0x2800, 0x3000, 0x3800, 0x4000
        ]
    );
    let expected = reference(FloatDotFormat::Bf16, &x, &y, Fp32Result { bits: 0, flags: 0 });
    assert_eq!(call.reference_result(), expected);
    // 3 + 2 - 3 + 2 + subnormal*inf = inf, exactly.
    assert_eq!(expected, Fp32Result { bits: 0x7f80_0000, flags: 0 });
}

#[test]
fn same_memory_differs_between_bf16_and_fp16() {
    let x = [0x3c00, 0x4000, 0x0001];
    let y = [0x3c00, 0x3c00, 0x3c00];
    let mut memory = Memory::new();
    memory.put_vector(0x1000, 1, &x);
    memory.put_vector(0x1800, 3, &y);
    let initial = Fp32Result { bits: 0x3f80_0000, flags: 8 };
    for (format, code) in [(FloatDotFormat::Bf16, BF16), (FloatDotFormat::Fp16, FP16)] {
        let payload =
            memory.payload([0x1000 | 1 << 32, 0x1800 | 3 << 32, 3, 0x3f80_0000, 8 | code]);
        let call = FloatDotCall::decode(0x100, 1, &payload).unwrap();
        assert_eq!(call.reference_result(), reference(format, &x, &y, initial));
    }
    let bf = memory.payload([0x1000 | 1 << 32, 0x1800 | 3 << 32, 3, 0x3f80_0000, 8 | BF16]);
    let fp = memory.payload([0x1000 | 1 << 32, 0x1800 | 3 << 32, 3, 0x3f80_0000, 8 | FP16]);
    assert_ne!(
        FloatDotCall::decode(0x100, 1, &bf).unwrap().reference_result(),
        FloatDotCall::decode(0x100, 1, &fp).unwrap().reference_result()
    );
}

#[test]
fn fp16_nans_subnormals_and_signed_zeros_follow_the_fp32_composite() {
    // sNaN raises NV in the multiply; 0*-x keeps the XOR sign; -0 + -0 stays -0.
    let cases: [(&[u16], &[u16], Fp32Result, Fp32Result); 4] = [
        (
            &[0x7d00],
            &[0x3c00],
            Fp32Result { bits: 0, flags: 0 },
            Fp32Result { bits: 0x7fc0_0000, flags: 16 },
        ),
        (
            &[0x7e00],
            &[0x3c00],
            Fp32Result { bits: 0, flags: 0 },
            Fp32Result { bits: 0x7fc0_0000, flags: 0 },
        ),
        (
            &[0x8000],
            &[0x3c00],
            Fp32Result { bits: 0x8000_0000, flags: 0 },
            Fp32Result { bits: 0x8000_0000, flags: 0 },
        ),
        // 2^-24 * 2^-24 = 2^-48 is an exact FP32 normal: no UF/NX.
        (
            &[0x0001],
            &[0x0001],
            Fp32Result { bits: 0, flags: 0 },
            Fp32Result { bits: 0x2780_0000, flags: 0 },
        ),
    ];
    for (x, y, initial, expected) in cases {
        assert_eq!(reference(FloatDotFormat::Fp16, x, y, initial), expected);
        let mut memory = Memory::new();
        memory.put_vector(0x1000, 1, x);
        memory.put_vector(0x2000, 1, y);
        let payload =
            memory.payload([0x1000 | 1 << 32, 0x2000 | 1 << 32, 1, u64::from(initial.bits), FP16]);
        assert_eq!(FloatDotCall::decode(0x100, 1, &payload).unwrap().reference_result(), expected);
    }
}

#[test]
fn chunked_calls_match_one_longer_call() {
    let x: Vec<u16> = (0..37).map(|i| 0x3c00 + 7 * i).collect();
    let y: Vec<u16> = (0..37).map(|i| 0xb800 ^ (3 * i)).collect();
    let mut memory = Memory::new();
    memory.put_vector(0x1000, 1, &x);
    memory.put_vector(0x4000, 5, &y);
    let whole = memory.payload([0x1000 | 1 << 32, 0x4000 | 5 << 32, 37, 0, FP16]);
    let whole = FloatDotCall::decode(0x100, 1, &whole).unwrap().reference_result();
    let first = memory.payload([0x1000 | 1 << 32, 0x4000 | 5 << 32, 20, 0, FP16]);
    let first = FloatDotCall::decode(0x100, 1, &first).unwrap().reference_result();
    let second = memory.payload([
        (0x1000 + 40) | 1 << 32,
        (0x4000 + 200) | 5 << 32,
        17,
        u64::from(first.bits),
        u64::from(first.flags) | FP16,
    ]);
    assert_eq!(FloatDotCall::decode(0x100, 1, &second).unwrap().reference_result(), whole);
    assert_eq!(whole, reference(FloatDotFormat::Fp16, &x, &y, Fp32Result { bits: 0, flags: 0 }));
}

#[test]
fn empty_16_bit_calls_preserve_state_and_read_only_the_descriptor() {
    for code in [BF16, FP16] {
        let payload =
            [0xffff_fffe | 0xffff << 32, 0, 0, 0xff80_0001, 8 | code, 8 << 32 | 0xff80_0001];
        let call = FloatDotCall::decode(0x100, 1, &payload).unwrap();
        assert_eq!(call.memory_reads().len(), 5);
        assert_eq!(call.reference_result(), Fp32Result { bits: 0xff80_0001, flags: 8 });
    }
}

#[test]
fn decoder_freezes_format_stride_alignment_and_range() {
    let valid = |x: u64, n: u64, flags: u64| {
        let words = if flags >> 8 == 0 { n.div_ceil(2) } else { n };
        let y = if flags >> 8 == 0 { 0x2000 } else { 0x2000 | 1 << 32 };
        let mut payload = vec![x, y, n, 0, flags, 0];
        payload.resize(6 + 2 * words as usize, 0);
        FloatDotCall::decode(0x100, 1, &payload).is_ok()
    };
    assert!(valid(0x1000 | 1 << 32, 4, BF16));
    assert!(valid(0x1000 | 0xffff << 32, 4, FP16));
    assert!(!valid(0x1000 | 0x1_0000 << 32, 4, BF16), "stride exceeds 16 bits");
    assert!(!valid(0x1001 | 1 << 32, 4, BF16), "odd byte address");
    assert!(!valid(0x1000 | 1 << 32, 4, 3 << 8), "format 3");
    assert!(!valid(0x1000 | 1 << 32, 4, 4 << 8), "format bit 10");
    assert!(!valid(0x1000 | 1 << 32, 4, BF16 | 32), "reserved flag bit 5");
    assert!(!valid(0x1000 | 1 << 32, 4, BF16 | 0x80), "reserved flag bit 7");
    assert!(!valid(0x1000 | 1 << 32, 4097, BF16), "length");
    // Legacy FP32 keeps its exact encoding: no strides and 8-byte alignment.
    assert!(valid(0x1000, 4, 0));
    assert!(!valid(0x1000 | 1 << 32, 4, 0), "FP32 stride");
    assert!(!valid(0x1002, 4, 0), "FP32 alignment");
    // Last element must end at or below 2^32: x + 2*s*(n-1) + 2 <= 2^32.
    assert!(valid(0xffff_fffe | 0xffff << 32, 1, BF16));
    assert!(valid((0x1_0000_0000 - 2 - 2 * 0xffff * 4095) | 0xffff << 32, 4096, BF16));
    assert!(!valid((0x1_0000_0000 - 2 * 0xffff * 4095) | 0xffff << 32, 4096, BF16));
    assert!(!valid(0xffff_fffe | 1 << 32, 2, FP16), "second element wraps");
    assert!(valid(0xffff_fffe, 4096, FP16), "stride 0 repeats a fitting element");
    // Payload length is exactly one word per 16-bit element per vector.
    let mut payload = vec![0x1000 | 1 << 32, 0x2000 | 1 << 32, 3, 0, BF16, 0];
    payload.resize(6 + 6, 0);
    assert!(FloatDotCall::decode(0x100, 1, &payload).is_ok());
    for len in [10, 11, 13] {
        let mut wrong = payload.clone();
        wrong.resize(len, 0);
        assert!(FloatDotCall::decode(0x100, 1, &wrong).is_err(), "{len} words");
    }
}

#[test]
#[ignore = "run node precompiles/helpers/tests/fp32_oracle.cjs to build the bundled SoftFloat oracle"]
fn fp16_multiply_composite_matches_softfloat_f16_to_f32() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use zisk_precomp_helpers::fp32_mul;
    let executable = std::env::var_os("FP16_SOFTFLOAT_ORACLE").expect("oracle executable");
    // Every FP16 encoding against signed zeros, subnormals, normals, extremes, inf and NaNs.
    let partners = [
        0x0000, 0x0001, 0x0002, 0x01ff, 0x0200, 0x03ff, 0x0400, 0x0401, 0x3bff, 0x3c00, 0x3c01,
        0x3e00, 0x7bfe, 0x7bff, 0x7c00, 0x7c01, 0x7dff, 0x7e00, 0x7fff,
    ];
    let mut pairs = Vec::new();
    for x in 0..=u16::MAX {
        for &y in &partners {
            pairs.push((x, y));
            pairs.push((x, y | 0x8000));
        }
    }
    let mut child =
        Command::new(executable).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let queries = pairs.clone();
    let writer = std::thread::spawn(move || {
        let mut input = std::io::BufWriter::new(&mut input);
        for (x, y) in queries {
            writeln!(input, "{x:04x} {y:04x}").unwrap();
        }
    });
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), pairs.len());
    for ((x, y), line) in pairs.into_iter().zip(lines) {
        let actual = fp32_mul(fp16_to_fp32(x), fp16_to_fp32(y));
        assert_eq!(format!("{:08x} {:02x}", actual.bits, actual.flags), line, "{x:04x}*{y:04x}");
    }
}
