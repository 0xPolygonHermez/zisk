//! Tests the experimental dot_bf16 precompile (emulator only, see precompiles/dot_bf16/README.md):
//! the basic cases of the plan, with their exact expected results, then longer random vectors
//! with blocks, checked against the reference semantics computed in software in the guest.
//!
//! Not registered as a test-artifacts feature: no state machine proves dot_bf16 yet, so the
//! program only runs on ziskemu.

#![no_main]
ziskos::entrypoint!(main);

use ziskos::syscalls::{
    dot_bf16_fp4_reference, dot_bf16_reference, syscall_dot_bf16, syscall_dot_bf16_fp4,
    SyscallDotBf16Fp4Params, SyscallDotBf16Params, DOT_BF16_FLAGS_MXFP4, DOT_BF16_FLAGS_NVFP4,
    DOT_BF16_NAN,
};

/// Vectors and results must be 8-byte aligned.
#[repr(C, align(8))]
struct Aligned<T>(T);

const ONE: u16 = 0x3f80;
const TWO: u16 = 0x4000;
const MAX: u16 = 0x7f7f;
const INF: u16 = 0x7f80;

/// 2^k as a BF16, for k in -133..=127.
fn pow2(k: i32) -> u16 {
    if k >= -126 {
        ((k + 127) as u16) << 7
    } else {
        1 << (k + 133)
    }
}

fn neg(b: u16) -> u16 {
    b ^ 0x8000
}

/// Calls the precompile and returns the f32 bits of the result.
fn dot(x: &[u16], y: &[u16], block: usize, out: Option<&mut [u32]>) -> u32 {
    let params = SyscallDotBf16Params {
        x: x.as_ptr(),
        y: y.as_ptr(),
        n: x.len() as u64,
        flags: 0,
        block: block as u64,
        out: out.map_or(core::ptr::null_mut(), |o| o.as_mut_ptr()),
    };
    unsafe { syscall_dot_bf16(&params) }.to_bits()
}

fn dot4(x: [u16; 4], y: [u16; 4]) -> u32 {
    let (x, y) = (Aligned(x), Aligned(y));
    dot(&x.0, &y.0, 0, None)
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A random finite BF16 with an unbiased exponent in lo..=hi.
    fn bf16(&mut self, lo: i32, hi: i32) -> u16 {
        let e = lo + (self.next() % (hi - lo + 1) as u64) as i32;
        let m = (self.next() & 0x7f) as u16;
        let s = ((self.next() & 1) as u16) << 15;
        s | ((e + 127) as u16) << 7 | m
    }
}

fn main() {
    // The basic tests of precompiles/dot_bf16/README.md, numbered as there
    assert_eq!(dot4([ONE, TWO, 0x4040, 0x4080], [ONE; 4]), 0x4120_0000, "1: 1+2+3+4");
    assert_eq!(dot4([pow2(100), ONE, neg(pow2(100)), 0], [ONE, ONE, ONE, 0]), 0x3f80_0000, "2");
    assert_eq!(dot4([ONE, pow2(-24), 0, 0], [ONE, ONE, 0, 0]), 0x3f80_0000, "3");
    assert_eq!(dot4([ONE, pow2(-24), pow2(-23), 0], [ONE, ONE, ONE, 0]), 0x3f80_0002, "4");
    assert_eq!(dot4([ONE, pow2(-24), pow2(-60), 0], [ONE, ONE, ONE, 0]), 0x3f80_0001, "5");
    assert_eq!(dot4([pow2(-133), 0, 0, 0], [pow2(-133), 0, 0, 0]), 0, "6");
    assert_eq!(dot4([pow2(-75), 0, 0, 0], [pow2(-74), 0, 0, 0]), 1, "7");
    assert_eq!(dot4([MAX, 0, 0, 0], [TWO, 0, 0, 0]), 0x7f80_0000, "8");
    assert_eq!(dot4([MAX, MAX, neg(MAX), 0], [TWO, ONE, TWO, 0]), 0x7f7f_0000, "9");
    assert_eq!(dot4([neg(ONE), 0, 0, 0], [0, 0, 0, 0]), 0, "10");
    assert_eq!(dot4([INF, ONE, ONE, ONE], [0, ONE, ONE, ONE]), DOT_BF16_NAN, "11");
    assert_eq!(dot4([0x7fc1, 0, 0, 0], [ONE, 0, 0, 0]), DOT_BF16_NAN, "12");

    // 13: blocks, rounded on their own; the total from the exact total
    let x = Aligned([pow2(100), 0, 0, 0, neg(pow2(100)), ONE, 0, 0]);
    let y = Aligned([ONE; 8]);
    let mut out = Aligned([0u32; 2]);
    assert_eq!(dot(&x.0, &y.0, 4, Some(&mut out.0)), 0x3f80_0000, "13: total");
    assert_eq!(out.0, [0x7180_0000, 0xf180_0000], "13: blocks");

    // 14: empty
    assert_eq!(dot(&[], &[], 0, None), 0, "14");

    // Longer random vectors, with blocks of 64 and an odd number of blocks, over a moderate and
    // over the full exponent range, against the reference computed in software
    let mut r = Rng(0x9e37_79b9_7f4a_7c15);
    let mut x = Aligned([0u16; 4096]);
    let mut y = Aligned([0u16; 4096]);
    let mut out = Aligned([0u32; 64]);
    let mut expected = [0u32; 64];
    for (n, block, lo, hi) in [(1024, 64, -8, 8), (4096, 64, -8, 8), (4096, 64, -126, 127), (960, 64, -40, 40)] {
        for i in 0..n {
            x.0[i] = r.bf16(lo, hi);
            y.0[i] = r.bf16(lo, hi);
        }
        let (xs, ys) = (&x.0[..n], &y.0[..n]);
        let blocks = n / block;
        let c = dot(xs, ys, block, Some(&mut out.0[..blocks]));
        let e = dot_bf16_reference(xs, ys, n, block, Some(&mut expected[..blocks]));
        assert_eq!(c, e, "random n = {n}");
        assert_eq!(out.0[..blocks], expected[..blocks], "random blocks n = {n}");
    }

    fp4_tests(&mut r);

    println!("dot_bf16: all tests passed");
}

/// E2M1 codes: 0x2 = 1.0, 0x4 = 2.0, 0x5 = 3.0, 0x6 = 4.0, 0x7 = 6.0, 0x1 = 0.5; bit 3 = sign.
const E2M1_1: u8 = 0x2;
const E2M1_2: u8 = 0x4;
const E2M1_3: u8 = 0x5;
const E2M1_4: u8 = 0x6;
const E2M1_6: u8 = 0x7;
const E2M1_HALF: u8 = 0x1;

/// E8M0 scale of 2^k.
fn e8m0(k: i32) -> u8 {
    (k + 127) as u8
}

/// Calls the FP4 variant and returns the f32 bits of the result. `codes` are E2M1 nibbles.
fn dot_fp4(flags: u64, codes: &[u8], scales: &[u8], y: &[u16], block: usize, out: Option<&mut [u32]>) -> u32 {
    let n = codes.len();
    let mut x = Aligned([0u8; 2048]);
    for (i, &c) in codes.iter().enumerate() {
        x.0[i / 2] |= c << (4 * (i & 1));
    }
    let mut s = Aligned([0xffu8; 256]); // bytes past the scales must be ignored
    s.0[..scales.len()].copy_from_slice(scales);
    let params = SyscallDotBf16Fp4Params {
        x: x.0.as_ptr(),
        y: y.as_ptr(),
        n: n as u64,
        flags,
        block: block as u64,
        out: out.map_or(core::ptr::null_mut(), |o| o.as_mut_ptr()),
        scales: s.0.as_ptr(),
    };
    unsafe { syscall_dot_bf16_fp4(&params) }.to_bits()
}

fn codes_at_start(n: usize, vals: &[u8]) -> [u8; 4096] {
    let mut c = [0u8; 4096];
    c[..vals.len()].copy_from_slice(vals);
    let _ = n;
    c
}

fn fp4_tests(r: &mut Rng) {
    const MX: u64 = DOT_BF16_FLAGS_MXFP4;
    const NV: u64 = DOT_BF16_FLAGS_NVFP4;
    let ones = Aligned([ONE; 64]);

    // The basic tests of precompiles/dot_bf16/fp4.md, numbered as there
    let c = codes_at_start(32, &[E2M1_1, E2M1_2, E2M1_3, E2M1_4]);
    assert_eq!(dot_fp4(MX, &c[..32], &[e8m0(0)], &ones.0[..32], 0, None), 0x4120_0000, "fp4 1");
    assert_eq!(dot_fp4(MX, &c[..32], &[e8m0(-3)], &ones.0[..32], 0, None), 0x3fa0_0000, "fp4 2");
    let c = codes_at_start(32, &[E2M1_6, E2M1_6 | 8]);
    let mut y = Aligned([0u16; 32]);
    (y.0[0], y.0[1]) = (pow2(100), pow2(100));
    assert_eq!(dot_fp4(MX, &c[..32], &[e8m0(127)], &y.0, 0, None), 0, "fp4 3");
    let c = codes_at_start(32, &[E2M1_HALF]);
    let mut y = Aligned([0u16; 32]);
    y.0[0] = pow2(-133);
    assert_eq!(dot_fp4(MX, &c[..32], &[e8m0(-127)], &y.0, 0, None), 0, "fp4 4");
    let c = codes_at_start(32, &[E2M1_6]);
    y.0[0] = MAX;
    assert_eq!(dot_fp4(MX, &c[..32], &[e8m0(127)], &y.0, 0, None), 0x7f80_0000, "fp4 5");
    let mut c = [0u8; 64];
    (c[0], c[32]) = (E2M1_1, E2M1_1);
    let mut out = Aligned([0u32; 2]);
    let t = dot_fp4(MX, &c, &[e8m0(10), e8m0(-20)], &ones.0, 32, Some(&mut out.0));
    assert_eq!((t, out.0), (0x4480_0000, [0x4480_0000, 0x3580_0000]), "fp4 6, 7");
    let c = codes_at_start(32, &[E2M1_1]);
    assert_eq!(dot_fp4(MX, &c[..32], &[0xff], &ones.0[..32], 0, None), DOT_BF16_NAN, "fp4 8");
    let mut y = Aligned([ONE; 32]);
    y.0[0] = INF;
    assert_eq!(dot_fp4(MX, &c[..32], &[e8m0(0)], &y.0, 0, None), DOT_BF16_NAN, "fp4 9");
    let mut c = [0u8; 64];
    (c[0], c[32]) = (E2M1_1, E2M1_HALF);
    assert_eq!(dot_fp4(MX, &c, &[e8m0(0), e8m0(-23)], &ones.0, 0, None), 0x3f80_0000, "fp4 10");

    // NVFP4: E4M3 scales 1.5 (0x3C) and 0.375 (0x2C): 1·1.5 + 2·0.375 = 2.25
    let mut c = [0u8; 32];
    (c[0], c[16]) = (E2M1_1, E2M1_2);
    assert_eq!(dot_fp4(NV, &c, &[0x3c, 0x2c], &ones.0[..32], 0, None), 0x4010_0000, "nvfp4 scales");
    let c = codes_at_start(16, &[E2M1_1]);
    assert_eq!(dot_fp4(NV, &c[..16], &[0x7f], &ones.0[..16], 0, None), DOT_BF16_NAN, "nvfp4 NaN");

    // Random rows of 2,880 values (90 MXFP4 blocks, 180 NVFP4 blocks), with blocks of 288 and
    // a partial last word of scales, against the reference computed in software
    let n = 2880;
    let mut x = Aligned([0u8; 1440]);
    let mut s = Aligned([0u8; 184]);
    let mut y = Aligned([0u16; 2880]);
    let mut out = Aligned([0u32; 10]);
    let mut expected = [0u32; 10];
    for flags in [MX, NV] {
        for _ in 0..3 {
            for b in x.0.iter_mut() {
                *b = r.next() as u8;
            }
            let nb = n / if flags == MX { 32 } else { 16 };
            for v in s.0[..nb].iter_mut() {
                *v = if flags == MX { 117 + (r.next() % 21) as u8 } else { (r.next() % 0x7f) as u8 };
            }
            for v in s.0[nb..].iter_mut() {
                *v = 0xff; // must be ignored
            }
            for v in y.0.iter_mut() {
                *v = r.bf16(-20, 20);
            }
            let params = SyscallDotBf16Fp4Params {
                x: x.0.as_ptr(),
                y: y.0.as_ptr(),
                n: n as u64,
                flags,
                block: 288,
                out: out.0.as_mut_ptr(),
                scales: s.0.as_ptr(),
            };
            let c = unsafe { syscall_dot_bf16_fp4(&params) }.to_bits();
            let e = dot_bf16_fp4_reference(flags, &x.0, &s.0[..nb], &y.0, n, 288, Some(&mut expected));
            assert_eq!(c, e, "random fp4 flags {flags}");
            assert_eq!(out.0, expected, "random fp4 blocks flags {flags}");
        }
    }
}
