//! Precompile calls whose operands live in the part of the RAM above 4 GB (byte addresses with
//! bit 32 set). Every precompile joins the two 32-bit limbs of an operand address into a 33-bit
//! address and checks that the high limb is a bit; this program makes that bit 1.
//!
//! Each two-operand precompile is called with the first operand above 4 GB and the second below,
//! and the other way round, so that the two high bits differ and both take both values. Every
//! result is compared with the one of the same call made entirely below 4 GB.
//!
//! It needs the 2 GB RAM: link it with `cargo-zisk build --ram-2gb` and run it on tools built with
//! the `ram_2gb` feature. Public output: a digest of the results (u64 LE).

#![no_main]
ziskos::entrypoint!(main);

use ziskos::syscalls::{
    syscall_arith384_mod, syscall_babyjubjub_add, syscall_blake2b_round, syscall_blake2sf,
    syscall_blake3f, syscall_bls12_381_complex_mul, syscall_bls12_381_curve_add,
    syscall_bls12_381_curve_dbl, syscall_poseidon1, syscall_poseidon2, syscall_poseidon2_koalabear,
    syscall_sha256_f, SyscallArith384ModParams, SyscallComplex384, SyscallPoint256,
    SyscallPoint384,
};

/// The first byte above 4 GB. The heap of this program never grows near it.
const ABOVE_4GB: u64 = 1 << 32;

/// The BLS12-381 base field prime, little-endian 64-bit limbs.
const BLS12_381_P: [u64; 6] = [
    0xb9fe_ffff_ffff_aaab,
    0x1eab_fffe_b153_ffff,
    0x6730_d2a0_f6b0_f624,
    0x6477_4b84_f385_12bf,
    0x4b1b_a7b6_434b_acd7,
    0x1a01_11ea_397f_e69a,
];

/// The BLS12-381 G1 generator.
const BLS12_381_G: SyscallPoint384 = SyscallPoint384 {
    x: [
        0xfb3a_f00a_db22_c6bb,
        0x6c55_e83f_f97a_1aef,
        0xa14e_3a3f_171b_ac58,
        0xc368_8c4f_9774_b905,
        0x2695_638c_4fa9_ac0f,
        0x17f1_d3a7_3197_d794,
    ],
    y: [
        0x0caa_2329_46c5_e7e1,
        0xd03c_c744_a288_8ae4,
        0x00db_18cb_2c04_b3ed,
        0xfcf5_e095_d5d0_0af6,
        0xa09e_30ed_741d_8ae4,
        0x08b3_f481_e3aa_a0f1,
    ],
};

/// Two distinct Baby Jubjub points (circomlib test/babyjub.js "Should add two points").
const BJJ_P1: SyscallPoint256 = SyscallPoint256 {
    x: [7699839232871213100, 11377594457309997372, 11223204053576433119, 2832127448816122262],
    y: [5014193860014684243, 12891415696119316661, 16351138293120604662, 418439792016356479],
};
const BJJ_P2: SyscallPoint256 = SyscallPoint256 {
    x: [8787143617238384983, 14452031502512802791, 7941460534247489132, 2635075998581887399],
    y: [3786921421244027607, 17990144301093405137, 1791269356123149699, 3316665278388780408],
};

/// A value of type `T` placed at `ABOVE_4GB + offset`.
///
/// SAFETY (for every use below): the offsets are 8-byte aligned and far enough apart for the
/// values not to overlap; the addresses are inside the 2 GB RAM, above the heap of this program,
/// and used by nothing else.
unsafe fn high<'a, T>(offset: u64, value: T) -> &'a mut T {
    let ptr = (ABOVE_4GB + offset) as *mut T;
    ptr.write(value);
    &mut *ptr
}

fn words<const N: usize>(seed: u64) -> [u64; N] {
    let mut x = seed;
    core::array::from_fn(|_| {
        x = x.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(23) ^ 0x1234_5678_9ABC_DEF1;
        x
    })
}

fn fold(digest: &mut u64, values: &[u64]) {
    for v in values {
        *digest = digest.rotate_left(7) ^ v;
    }
}

/// A precompile with a state (operand a, overwritten) and an input (operand b), called with every
/// combination of the two above and below 4 GB.
fn state_and_input<const S: usize, const I: usize>(
    digest: &mut u64,
    offset: u64,
    seed: u64,
    f: impl Fn(&mut [u64; S], &[u64; I]),
) {
    let state: [u64; S] = words(seed);
    let input: [u64; I] = words(seed + 1);

    let mut expected = state;
    f(&mut expected, &input);

    // state above, input below
    let hi_state = unsafe { high(offset, state) };
    f(hi_state, &input);
    assert_eq!(*hi_state, expected);

    // state below, input above
    let hi_input = unsafe { high(offset + 0x100, input) };
    let mut lo_state = state;
    f(&mut lo_state, hi_input);
    assert_eq!(lo_state, expected);

    // both above
    let hi_state = unsafe { high(offset + 0x200, state) };
    f(hi_state, hi_input);
    assert_eq!(*hi_state, expected);

    fold(digest, &expected);
}

/// A precompile with a single state operand (b), small enough values to be field elements.
fn state_only<const S: usize>(digest: &mut u64, offset: u64, seed: u64, f: impl Fn(*mut [u64; S])) {
    let state: [u64; S] = core::array::from_fn(|i| seed + i as u64);
    let mut expected = state;
    f(&mut expected);
    let hi_state = unsafe { high(offset, state) };
    f(hi_state);
    assert_eq!(*hi_state, expected);
    fold(digest, &expected);
}

fn twisted_edwards(digest: &mut u64, offset: u64) {
    let mut expected = BJJ_P1;
    syscall_babyjubjub_add(&mut expected, &BJJ_P2);

    // p1 above, p2 below
    let p1 = unsafe { high(offset, BJJ_P1) };
    syscall_babyjubjub_add(p1, &BJJ_P2);
    assert_eq!((p1.x, p1.y), (expected.x, expected.y));

    // p1 below, p2 above
    let p2 = unsafe { high(offset + 0x100, BJJ_P2) };
    let mut p1 = BJJ_P1;
    syscall_babyjubjub_add(&mut p1, p2);
    assert_eq!((p1.x, p1.y), (expected.x, expected.y));

    fold(digest, &expected.x);
    fold(digest, &expected.y);
}

fn bls12_381(digest: &mut u64, offset: u64) {
    let g = || SyscallPoint384 { x: BLS12_381_G.x, y: BLS12_381_G.y };

    // dbl: its only operand above
    let mut expected_2g = g();
    syscall_bls12_381_curve_dbl(&mut expected_2g);
    let two_g = unsafe { high(offset, g()) };
    syscall_bls12_381_curve_dbl(two_g);
    assert_eq!((two_g.x, two_g.y), (expected_2g.x, expected_2g.y));

    // add: p1 = 2G above and p2 = G below, then p1 = G below and p2 = 2G above
    let mut expected_3g = g();
    syscall_bls12_381_curve_add(&mut expected_3g, &expected_2g);
    let p1 = unsafe { high(offset + 0x100, SyscallPoint384 { x: two_g.x, y: two_g.y }) };
    syscall_bls12_381_curve_add(p1, &g());
    assert_eq!((p1.x, p1.y), (expected_3g.x, expected_3g.y));
    let mut p1 = g();
    syscall_bls12_381_curve_add(&mut p1, two_g);
    assert_eq!((p1.x, p1.y), (expected_3g.x, expected_3g.y));

    // complex mul: f1 above and f2 below, then the other way round
    let f1 = || SyscallComplex384 { x: [3, 0, 0, 0, 0, 0], y: [5, 0, 0, 0, 0, 0] };
    let f2 = || SyscallComplex384 { x: [7, 1, 0, 0, 0, 0], y: [11, 0, 2, 0, 0, 0] };
    let mut expected_mul = f1();
    syscall_bls12_381_complex_mul(&mut expected_mul, &f2());
    let hi_f1 = unsafe { high(offset + 0x200, f1()) };
    syscall_bls12_381_complex_mul(hi_f1, &f2());
    assert_eq!((hi_f1.x, hi_f1.y), (expected_mul.x, expected_mul.y));
    let hi_f2 = unsafe { high(offset + 0x300, f2()) };
    let mut lo_f1 = f1();
    syscall_bls12_381_complex_mul(&mut lo_f1, hi_f2);
    assert_eq!((lo_f1.x, lo_f1.y), (expected_mul.x, expected_mul.y));

    fold(digest, &expected_3g.x);
    fold(digest, &expected_3g.y);
    fold(digest, &expected_mul.x);
    fold(digest, &expected_mul.y);
}

/// arith384_mod reads its five pointers from a parameter struct: the struct and the operands are
/// placed above and below 4 GB in both alternating patterns.
fn arith384_mod(digest: &mut u64, offset: u64) {
    let a: [u64; 6] = [3, 5, 7, 11, 13, 17];
    let b: [u64; 6] = [19, 23, 29, 31, 37, 41];
    let c: [u64; 6] = [43, 47, 53, 59, 61, 67];

    let mut expected = [0u64; 6];
    syscall_arith384_mod(&mut SyscallArith384ModParams {
        a: &a,
        b: &b,
        c: &c,
        module: &BLS12_381_P,
        d: &mut expected,
    });

    // struct, a, c and d above; b and the module below
    let hi_a = unsafe { high(offset, a) };
    let hi_c = unsafe { high(offset + 0x100, c) };
    let hi_d = unsafe { high(offset + 0x200, [0u64; 6]) };
    let params = unsafe {
        high(
            offset + 0x300,
            SyscallArith384ModParams { a: hi_a, b: &b, c: hi_c, module: &BLS12_381_P, d: hi_d },
        )
    };
    syscall_arith384_mod(params);
    assert_eq!(*params.d, expected);

    // struct, a, c and d below; b and the module above
    let hi_b = unsafe { high(offset + 0x400, b) };
    let hi_module = unsafe { high(offset + 0x500, BLS12_381_P) };
    let mut d = [0u64; 6];
    syscall_arith384_mod(&mut SyscallArith384ModParams {
        a: &a,
        b: hi_b,
        c: &c,
        module: hi_module,
        d: &mut d,
    });
    assert_eq!(d, expected);

    fold(digest, &expected);
}

fn main() {
    let mut digest = 0u64;

    state_and_input::<4, 8>(&mut digest, 0x1_0000, 1, |s, i| syscall_sha256_f(s, i));
    state_and_input::<16, 16>(&mut digest, 0x2_0000, 2, |s, i| syscall_blake2b_round(3, s, i));
    state_and_input::<8, 8>(&mut digest, 0x3_0000, 3, |s, i| syscall_blake2sf(s, i));
    state_and_input::<8, 8>(&mut digest, 0x4_0000, 4, |s, i| syscall_blake3f(s, i));

    state_only::<16>(&mut digest, 0x5_0000, 5, |s| unsafe { syscall_poseidon1(s) });
    state_only::<16>(&mut digest, 0x6_0000, 6, |s| unsafe { syscall_poseidon2(s) });
    state_only::<8>(&mut digest, 0x7_0000, 7, |s| unsafe { syscall_poseidon2_koalabear(s) });

    twisted_edwards(&mut digest, 0x8_0000);
    bls12_381(&mut digest, 0x9_0000);
    arith384_mod(&mut digest, 0xA_0000);

    ziskos::io::commit_slice(&digest.to_le_bytes());
}
