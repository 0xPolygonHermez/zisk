//! Calls every C-ABI function of the Rust ZisK library (`ziskos::zisklib`) that
//! writes through a caller-supplied output pointer, with that output at byte
//! offsets 0..7. Each call must write the same bytes, and return the same value,
//! as a call with an aligned output, and touch nothing around them.
//!
//! Input: two `u64`s, the first and last case to run (one case alone finds the
//! one that halts or panics; `0, u64::MAX` runs them all). Each case prints `case <n> <name> ret=<r> mask=<m>`,
//! where bit `o` of `m` is set when offset `o` was right (0xff when all eight are);
//! the committed output is 1 when every case run was 0xff, and the guest panics
//! otherwise (so a plain emulation run fails on a wrong case).
//!
//! Build it in debug too: Rust's own alignment checks then panic on any reference
//! or typed access through an unaligned output pointer, which release builds
//! would silently accept.

#![no_main]
ziskos::entrypoint!(main);

use core::hint::black_box;

// ---- the functions under test (ziskos #[no_mangle] exports) ------------------
#[allow(improper_ctypes)]
extern "C" {
    // uint256, little-endian limbs
    fn checked_add256_c(a: *const u64, b: *const u64, r: *mut u64) -> u8;
    fn checked_neg256_c(a: *const u64, r: *mut u64) -> u8;
    fn checked_sub256_c(a: *const u64, b: *const u64, r: *mut u64) -> u8;
    fn overflowing_add256_c(a: *const u64, b: *const u64, r: *mut u64) -> u8;
    fn overflowing_neg256_c(a: *const u64, r: *mut u64) -> u8;
    fn overflowing_sub256_c(a: *const u64, b: *const u64, r: *mut u64) -> u8;
    fn saturating_add256_c(a: *const u64, b: *const u64, r: *mut u64);
    fn saturating_sub256_c(a: *const u64, b: *const u64, r: *mut u64);
    fn wrapping_add256_c(a: *const u64, b: *const u64, r: *mut u64);
    fn wrapping_sub256_c(a: *const u64, b: *const u64, r: *mut u64);
    fn wrapping_neg256_c(a: *const u64, r: *mut u64);
    fn checked_div256_c(a: *const u64, b: *const u64, r: *mut u64) -> u8;
    fn checked_rem256_c(a: *const u64, b: *const u64, r: *mut u64) -> u8;
    fn div_rem256_c(a: *const u64, b: *const u64, q: *mut u64, r: *mut u64);
    fn div_ceil256_c(a: *const u64, b: *const u64, r: *mut u64);
    fn wrapping_div256_c(a: *const u64, b: *const u64, r: *mut u64);
    fn wrapping_rem256_c(a: *const u64, b: *const u64, r: *mut u64);
    fn reduce_mod256_c(a: *const u64, m: *const u64, r: *mut u64);
    fn add_mod256_c(a: *const u64, b: *const u64, m: *const u64, r: *mut u64);
    fn mul_mod256_c(a: *const u64, b: *const u64, m: *const u64, r: *mut u64);
    fn square_mod256_c(a: *const u64, m: *const u64, r: *mut u64);
    fn pow_mod256_c(b: *const u64, e: *const u64, m: *const u64, r: *mut u64);
    fn inv_mod256_c(a: *const u64, m: *const u64, r: *mut u64) -> u8;
    fn mul_mod_bytes256_c(a: *const u8, b: *const u8, m: *const u8, r: *mut u8);
    fn reduce_mod_bytes256_c(a: *const u8, m: *const u8, r: *mut u8);
    fn add_mod_bytes256_c(a: *const u8, b: *const u8, m: *const u8, r: *mut u8);
    fn square_mod_bytes256_c(a: *const u8, m: *const u8, r: *mut u8);
    fn pow_mod_bytes256_c(b: *const u8, e: *const u8, m: *const u8, r: *mut u8);
    fn inv_mod_bytes256_c(a: *const u8, m: *const u8, r: *mut u8) -> u8;
    fn checked_mul256_c(a: *const u64, b: *const u64, r: *mut u64) -> u8;
    fn checked_square256_c(a: *const u64, r: *mut u64) -> u8;
    fn overflowing_mul256_c(a: *const u64, b: *const u64, r: *mut u64) -> u8;
    fn overflowing_square256_c(a: *const u64, r: *mut u64) -> u8;
    fn saturating_mul256_c(a: *const u64, b: *const u64, r: *mut u64);
    fn saturating_square256_c(a: *const u64, r: *mut u64);
    fn wrapping_mul256_c(a: *const u64, b: *const u64, r: *mut u64);
    fn wrapping_square256_c(a: *const u64, r: *mut u64);
    fn inv256_c(a: *const u64, r: *mut u64) -> u8;
    fn checked_pow256_c(b: *const u64, e: *const u64, r: *mut u64) -> u8;
    fn overflowing_pow256_c(b: *const u64, e: *const u64, r: *mut u64) -> u8;
    fn wrapping_pow256_c(b: *const u64, e: *const u64, r: *mut u64);
    fn saturating_pow256_c(b: *const u64, e: *const u64, r: *mut u64);
    // bigint
    fn modexp_u64_c(
        b: *const u64,
        bl: usize,
        e: *const u64,
        el: usize,
        m: *const u64,
        ml: usize,
        r: *mut u64,
    ) -> usize;
    // secp256k1
    fn jacobian_to_affine_secp256k1_c(p: *const u64, r: *mut u64);
    fn lift_x_secp256k1_c(x: *const u8, y_is_odd: u8, r: *mut u64) -> u8;
    fn double_scalar_mul_with_g_secp256k1_c(
        k1: *const u64,
        k2: *const u64,
        p: *const u64,
        r: *mut u64,
    ) -> u8;
    fn reduce_fp_secp256k1_c(x: *const u64, r: *mut u64);
    fn add_fp_secp256k1_c(x: *const u64, y: *const u64, r: *mut u64);
    fn neg_fp_secp256k1_c(x: *const u64, r: *mut u64);
    fn sub_fp_secp256k1_c(x: *const u64, y: *const u64, r: *mut u64);
    fn mul_fp_secp256k1_c(x: *const u64, y: *const u64, r: *mut u64);
    fn square_fp_secp256k1_c(x: *const u64, r: *mut u64);
    fn sqrt_fp_secp256k1_c(x: *const u64, parity: u64, r: *mut u64) -> u8;
    fn inv_fp_secp256k1_c(x: *const u64, r: *mut u64);
    fn reduce_fn_secp256k1_c(x: *const u64, r: *mut u64);
    fn add_fn_secp256k1_c(x: *const u64, y: *const u64, r: *mut u64);
    fn neg_fn_secp256k1_c(x: *const u64, r: *mut u64);
    fn sub_fn_secp256k1_c(x: *const u64, y: *const u64, r: *mut u64);
    fn mul_fn_secp256k1_c(x: *const u64, y: *const u64, r: *mut u64);
    fn square_fn_secp256k1_c(x: *const u64, r: *mut u64);
    fn inv_fn_secp256k1_c(x: *const u64, r: *mut u64);
    // bn254
    fn jacobian_to_affine_bn254_c(p: *const u64, r: *mut u64) -> u8;
    fn add_bn254_c(p1: *const u64, p2: *const u64, r: *mut u64) -> u8;
    fn scalar_mul_bn254_c(p: *const u64, k: *const u64, r: *mut u64) -> u8;
    fn jacobian_to_affine_twist_bn254_c(p: *const u64, r: *mut u64);
    // bls12-381
    fn decompress_bls12_381_c(input: *const u8, r: *mut u64) -> u8;
    fn add_bls12_381_c(p1: *const u64, p2: *const u64, r: *mut u64) -> u8;
    fn scalar_mul_bls12_381_c(p: *const u64, k: *const u64, r: *mut u64) -> u8;
    fn final_exp_bls12_381_c(f: *const u64, r: *mut u64);
    fn mul_fp12_bls12_381_c(a: *const u64, b: *const u64, r: *mut u64);
    fn miller_loop_bls12_381_c(p: *const u64, q: *const u64, r: *mut u64);
    fn decompress_twist_bls12_381_c(input: *const u8, r: *mut u64) -> u8;
    fn add_twist_bls12_381_c(p1: *const u64, p2: *const u64, r: *mut u64) -> u8;
    fn scalar_mul_twist_bls12_381_c(p: *const u64, k: *const u64, r: *mut u64) -> u8;
    // hashes
    fn native_keccak256(bytes: *const u8, len: usize, out: *mut u8);
    fn sha256f_compress_c(state: *mut u32, blocks: *const u8, num_blocks: usize);
    // EF zkVM accelerator ABI (zkvm_accelerators.h) and its extensions
    fn zkvm_keccak256(data: *const u8, len: usize, out: *mut u8) -> i32;
    fn zkvm_sha256(data: *const u8, len: usize, out: *mut u8) -> i32;
    fn zkvm_ripemd160(data: *const u8, len: usize, out: *mut u8) -> i32;
    fn zkvm_modexp(
        b: *const u8,
        bl: usize,
        e: *const u8,
        el: usize,
        m: *const u8,
        ml: usize,
        out: *mut u8,
    ) -> i32;
    fn zkvm_bn254_g1_add(p1: *const u8, p2: *const u8, r: *mut u8) -> i32;
    fn zkvm_bn254_g1_mul(p: *const u8, k: *const u8, r: *mut u8) -> i32;
    fn zkvm_blake2f(rounds: u32, h: *mut u8, m: *const u8, t: *const u8, f: u8) -> i32;
    fn zkvm_bls12_g1_add(p1: *const u8, p2: *const u8, r: *mut u8) -> i32;
    fn zkvm_bls12_g1_msm(pairs: *const u8, n: usize, r: *mut u8) -> i32;
    fn zkvm_bls12_g2_add(p1: *const u8, p2: *const u8, r: *mut u8) -> i32;
    fn zkvm_bls12_g2_msm(pairs: *const u8, n: usize, r: *mut u8) -> i32;
    fn zkvm_bls12_map_fp_to_g1(fe: *const u8, r: *mut u8) -> i32;
    fn zkvm_bls12_map_fp2_to_g2(fe: *const u8, r: *mut u8) -> i32;
    fn zkvm_secp256k1_ecrecover(msg: *const u8, sig: *const u8, recid: u8, out: *mut u8) -> i32;
    fn zkvm_mulmod256(a: *const u8, b: *const u8, m: *const u8, out: *mut u8) -> i32;
    fn zkvm_reduce_mod256(a: *const u8, m: *const u8, out: *mut u8) -> i32;
    fn zkvm_add_mod256(a: *const u8, b: *const u8, m: *const u8, out: *mut u8) -> i32;
    fn zkvm_square_mod256(a: *const u8, m: *const u8, out: *mut u8) -> i32;
    fn zkvm_pow_mod256(b: *const u8, e: *const u8, m: *const u8, out: *mut u8) -> i32;
    fn zkvm_inv_mod256(a: *const u8, m: *const u8, out: *mut u8) -> i32;
}

// ---- inputs ------------------------------------------------------------------
const A: [u64; 4] =
    [0x0123456789abcdef, 0xfedcba9876543210, 0x0f1e2d3c4b5a6978, 0x1122334455667788];
const B: [u64; 4] =
    [0x8877665544332211, 0x0011223344556677, 0x99aabbccddeeff00, 0x0000000000000abc];
const M: [u64; 4] =
    [0xffffffff00000001, 0x0000000000000000, 0x00000000ffffffff, 0xffffffff00000000];
const M8: [u64; 8] = [
    0x0123456789abcdef,
    0xfedcba9876543210,
    0x0f1e2d3c4b5a6978,
    0x1122334455667788,
    0x8877665544332211,
    0x0011223344556677,
    0x99aabbccddeeff00,
    0x0000000000000abd,
];
const E: [u64; 4] = [0x10001, 0, 0, 0];
const E3: [u64; 4] = [3, 0, 0, 0];
const SMALL: [u64; 4] = [0x1234, 0, 0, 0];
/// secp256k1 generator, x then y, little-endian limbs
const SECP_G: [u64; 8] = [
    0x59F2815B16F81798,
    0x029BFCDB2DCE28D9,
    0x55A06295CE870B07,
    0x79BE667EF9DCBBAC,
    0x9C47D08FFB10D4B8,
    0xFD17B448A6855419,
    0x5DA4FBFC0E1108A8,
    0x483ADA7726A3C465,
];
/// secp256k1 ECDSA vector (as in the ziskasm unaligned_out guest), little-endian limbs
const R: [u64; 4] =
    [0x68df7d8d7e0fb36b, 0xc2189fe681cd6e78, 0xc85ba1fd6238ecb5, 0x3e125456c8338994];
const S: [u64; 4] =
    [0xd4e89d1ae75aeea2, 0xb8e33178783bd1a3, 0x0866acebc9e141ec, 0x3a816b1c33739e41];
const Z: [u64; 4] =
    [0x1bf86a1816a52f52, 0xd31e26c3da73dda8, 0xa3b71997594da038, 0x17560495f6944673];
/// BLS12-381 generators, little-endian limbs (x then y; G2: x0 x1 y0 y1)
const BLS_G1: [u64; 12] = [
    0xFB3AF00ADB22C6BB,
    0x6C55E83FF97A1AEF,
    0xA14E3A3F171BAC58,
    0xC3688C4F9774B905,
    0x2695638C4FA9AC0F,
    0x17F1D3A73197D794,
    0x0CAA232946C5E7E1,
    0xD03CC744A2888AE4,
    0x00DB18CB2C04B3ED,
    0xFCF5E095D5D00AF6,
    0xA09E30ED741D8AE4,
    0x08B3F481E3AAA0F1,
];
const BLS_G2: [u64; 24] = [
    0xD48056C8C121BDB8,
    0x0BAC0326A805BBEF,
    0xB4510B647AE3D177,
    0xC6E47AD4FA403B02,
    0x260805272DC51051,
    0x024AA2B2F08F0A91,
    0xE5AC7D055D042B7E,
    0x334CF11213945D57,
    0xB5DA61BBDC7F5049,
    0x596BD0D09920B61A,
    0x7DACD3A088274F65,
    0x13E02B6052719F60,
    0xE193548608B82801,
    0x923AC9CC3BACA289,
    0x6D429A695160D12C,
    0xADFD9BAA8CBDD3A7,
    0x8CC9CDC6DA2E351A,
    0x0CE5D527727D6E11,
    0xAAA9075FF05F79BE,
    0x3F370D275CEC1DA1,
    0x267492AB572E99AB,
    0xCB3E287E85A763AF,
    0x32ACD2B02BC28B99,
    0x0606C4A02EA734CC,
];

/// Inputs built at run time (some from the library itself, with aligned outputs).
struct Inputs {
    ba: [u8; 32],
    bb: [u8; 32],
    bm: [u8; 32],
    be: [u8; 32],
    secp_gx_be: [u8; 32],
    secp_jac: [u64; 12],
    secp_y2: [u64; 4],
    bn_g1: [u64; 8],
    bn_jac: [u64; 12],
    bn_twist_jac: [u64; 24],
    bls_g1_comp: [u8; 48],
    bls_g2_comp: [u8; 96],
    fp12: [u64; 72],
    hb: [u8; 300],
    blocks: [u8; 128],
    ef_bn_g1: [u8; 64],
    ef_bn_k: [u8; 32],
    ef_rmsg: [u8; 32],
    ef_rsig: [u8; 64],
    ef_fp: [u8; 48],
    ef_fp2: [u8; 96],
    ef_g1: [u8; 96],
    ef_g2: [u8; 192],
    ef_g1_pair: [u8; 128],
    ef_g2_pair: [u8; 224],
    ef_bm: [u8; 128],
    ef_bt: [u8; 16],
    code: [u64; 16],
}

/// Big-endian bytes of `n` little-endian limbs.
fn be_bytes(le: &[u64], out: &mut [u8]) {
    let n = le.len();
    for i in 0..n * 8 {
        out[i] = (le[n - 1 - i / 8] >> (8 * (7 - i % 8))) as u8;
    }
}

fn setup() -> Inputs {
    let mut x = Inputs {
        ba: [0; 32],
        bb: [0; 32],
        bm: [0; 32],
        be: [0; 32],
        secp_gx_be: [0; 32],
        secp_jac: [0; 12],
        secp_y2: [0; 4],
        bn_g1: [1, 0, 0, 0, 2, 0, 0, 0],
        bn_jac: [1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0],
        bn_twist_jac: [0; 24],
        bls_g1_comp: [0; 48],
        bls_g2_comp: [0; 96],
        fp12: [0; 72],
        hb: [0; 300],
        blocks: [0; 128],
        ef_bn_g1: [0; 64],
        ef_bn_k: [0; 32],
        ef_rmsg: [0; 32],
        ef_rsig: [0; 64],
        ef_fp: [0; 48],
        ef_fp2: [0; 96],
        ef_g1: [0; 96],
        ef_g2: [0; 192],
        ef_g1_pair: [0; 128],
        ef_g2_pair: [0; 224],
        ef_bm: [0; 128],
        ef_bt: [0; 16],
        code: [0; 16],
    };
    be_bytes(&A, &mut x.ba);
    be_bytes(&B, &mut x.bb);
    be_bytes(&M, &mut x.bm);
    be_bytes(&E, &mut x.be);
    be_bytes(&SECP_G[..4], &mut x.secp_gx_be);
    x.secp_jac[..8].copy_from_slice(&SECP_G);
    x.secp_jac[8] = 1;
    unsafe {
        mul_fp_secp256k1_c(SECP_G[4..].as_ptr(), SECP_G[4..].as_ptr(), x.secp_y2.as_mut_ptr())
    };
    // bn254 twist point in Jacobian form with z = 1 (x, y arbitrary: the conversion
    // does not check the curve)
    for (i, v) in x.bn_twist_jac.iter_mut().enumerate().take(16) {
        *v = (i as u64 + 1) * 0x0101_0101;
    }
    x.bn_twist_jac[16] = 1;
    // compressed BLS12-381 generators: big-endian x with the compression flag
    be_bytes(&BLS_G1[..6], &mut x.bls_g1_comp);
    x.bls_g1_comp[0] |= 0x80;
    be_bytes(&BLS_G2[6..12], &mut x.bls_g2_comp[..48]); // x1 first
    be_bytes(&BLS_G2[..6], &mut x.bls_g2_comp[48..]);
    x.bls_g2_comp[0] |= 0x80;
    unsafe { miller_loop_bls12_381_c(BLS_G1.as_ptr(), BLS_G2.as_ptr(), x.fp12.as_mut_ptr()) };
    for (i, b) in x.hb.iter_mut().enumerate() {
        *b = (i * 7 + 1) as u8;
    }
    for (i, b) in x.blocks.iter_mut().enumerate() {
        *b = (i * 13 + 5) as u8;
    }
    // EF encodings
    x.ef_bn_g1[31] = 1;
    x.ef_bn_g1[63] = 2;
    be_bytes(&A, &mut x.ef_bn_k);
    be_bytes(&Z, &mut x.ef_rmsg);
    be_bytes(&R, &mut x.ef_rsig[..32]);
    be_bytes(&S, &mut x.ef_rsig[32..]);
    x.ef_fp[47] = 5;
    x.ef_fp2[47] = 3;
    x.ef_fp2[95] = 4;
    unsafe {
        zkvm_bls12_map_fp_to_g1(x.ef_fp.as_ptr(), x.ef_g1.as_mut_ptr());
        zkvm_bls12_map_fp2_to_g2(x.ef_fp2.as_ptr(), x.ef_g2.as_mut_ptr());
    }
    x.ef_g1_pair[..96].copy_from_slice(&x.ef_g1);
    be_bytes(&B, &mut x.ef_g1_pair[96..]);
    x.ef_g2_pair[..192].copy_from_slice(&x.ef_g2);
    be_bytes(&B, &mut x.ef_g2_pair[192..]);
    for (i, b) in x.ef_bm.iter_mut().enumerate() {
        *b = i as u8;
    }
    x.ef_bt[0] = 128;
    // EVM bytecode with JUMPDESTs (0x5b) inside and outside PUSH data
    let code = unsafe { core::slice::from_raw_parts_mut(x.code.as_mut_ptr() as *mut u8, 128) };
    for (i, b) in code.iter_mut().enumerate() {
        *b = [0x5b, 0x60, 0x5b, 0x01, 0x61, 0x5b, 0x5b, 0x5b, 0x7f][i % 9];
    }
    x
}

// ---- cases ---------------------------------------------------------------------
/// One call: writes its result(s) to `out` (seeded first when in/out), returns a
/// value to compare too.
type CaseFn = unsafe fn(&Inputs, *mut u8) -> u64;

macro_rules! u64p {
    ($o:expr) => {
        $o as *mut u64
    };
}

/// A case's return value, as a number to compare (`()` for functions without one).
trait Ret {
    fn ret(self) -> u64;
}
impl Ret for () {
    fn ret(self) -> u64 {
        0
    }
}
macro_rules! ret_as_u64 {
    ($($t:ty),*) => {$(impl Ret for $t { fn ret(self) -> u64 { self as u64 } })*};
}
ret_as_u64!(u8, i32, usize);

macro_rules! cases {
    ($($name:ident, $len:expr, |$x:ident, $o:ident| $body:expr;)*) => {
        $(
            #[allow(unused_variables)]
            unsafe fn $name($x: &Inputs, $o: *mut u8) -> u64 { Ret::ret($body) }
        )*
        const CASES: &[(&str, usize, CaseFn)] = &[$((stringify!($name), $len, $name as CaseFn)),*];
    };
}

cases! {
    // uint256
    checked_add256, 32, |x, o| checked_add256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    checked_neg256, 32, |x, o| checked_neg256_c([0u64; 4].as_ptr(), u64p!(o));
    checked_sub256, 32, |x, o| checked_sub256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    overflowing_add256, 32, |x, o| overflowing_add256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    overflowing_neg256, 32, |x, o| overflowing_neg256_c(A.as_ptr(), u64p!(o));
    overflowing_sub256, 32, |x, o| overflowing_sub256_c(B.as_ptr(), A.as_ptr(), u64p!(o));
    saturating_add256, 32, |x, o| saturating_add256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    saturating_sub256, 32, |x, o| saturating_sub256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    wrapping_add256, 32, |x, o| wrapping_add256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    wrapping_sub256, 32, |x, o| wrapping_sub256_c(B.as_ptr(), A.as_ptr(), u64p!(o));
    wrapping_neg256, 32, |x, o| wrapping_neg256_c(A.as_ptr(), u64p!(o));
    checked_div256, 32, |x, o| checked_div256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    checked_rem256, 32, |x, o| checked_rem256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    div_rem256, 64, |x, o| div_rem256_c(A.as_ptr(), B.as_ptr(), u64p!(o), u64p!(o.add(32)));
    div_ceil256, 32, |x, o| div_ceil256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    wrapping_div256, 32, |x, o| wrapping_div256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    wrapping_rem256, 32, |x, o| wrapping_rem256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    reduce_mod256, 32, |x, o| reduce_mod256_c(A.as_ptr(), M.as_ptr(), u64p!(o));
    add_mod256, 32, |x, o| add_mod256_c(A.as_ptr(), B.as_ptr(), M.as_ptr(), u64p!(o));
    mul_mod256, 32, |x, o| mul_mod256_c(A.as_ptr(), B.as_ptr(), M.as_ptr(), u64p!(o));
    square_mod256, 32, |x, o| square_mod256_c(A.as_ptr(), M.as_ptr(), u64p!(o));
    pow_mod256, 32, |x, o| pow_mod256_c(A.as_ptr(), E.as_ptr(), M.as_ptr(), u64p!(o));
    inv_mod256, 32, |x, o| inv_mod256_c(B.as_ptr(), M.as_ptr(), u64p!(o));
    mul_mod_bytes256, 32, |x, o| mul_mod_bytes256_c(x.ba.as_ptr(), x.bb.as_ptr(), x.bm.as_ptr(), o);
    reduce_mod_bytes256, 32, |x, o| reduce_mod_bytes256_c(x.ba.as_ptr(), x.bm.as_ptr(), o);
    add_mod_bytes256, 32, |x, o| add_mod_bytes256_c(x.ba.as_ptr(), x.bb.as_ptr(), x.bm.as_ptr(), o);
    square_mod_bytes256, 32, |x, o| square_mod_bytes256_c(x.ba.as_ptr(), x.bm.as_ptr(), o);
    pow_mod_bytes256, 32, |x, o| pow_mod_bytes256_c(x.ba.as_ptr(), x.be.as_ptr(), x.bm.as_ptr(), o);
    inv_mod_bytes256, 32, |x, o| inv_mod_bytes256_c(x.bb.as_ptr(), x.bm.as_ptr(), o);
    checked_mul256, 32, |x, o| checked_mul256_c(SMALL.as_ptr(), B.as_ptr(), u64p!(o));
    checked_square256, 32, |x, o| checked_square256_c(SMALL.as_ptr(), u64p!(o));
    overflowing_mul256, 32, |x, o| overflowing_mul256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    overflowing_square256, 32, |x, o| overflowing_square256_c(A.as_ptr(), u64p!(o));
    saturating_mul256, 32, |x, o| saturating_mul256_c(SMALL.as_ptr(), B.as_ptr(), u64p!(o));
    saturating_square256, 32, |x, o| saturating_square256_c(SMALL.as_ptr(), u64p!(o));
    wrapping_mul256, 32, |x, o| wrapping_mul256_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    wrapping_square256, 32, |x, o| wrapping_square256_c(A.as_ptr(), u64p!(o));
    inv256, 32, |x, o| inv256_c(A.as_ptr(), u64p!(o));
    checked_pow256, 32, |x, o| checked_pow256_c(SMALL.as_ptr(), E3.as_ptr(), u64p!(o));
    overflowing_pow256, 32, |x, o| overflowing_pow256_c(B.as_ptr(), E.as_ptr(), u64p!(o));
    wrapping_pow256, 32, |x, o| wrapping_pow256_c(B.as_ptr(), E.as_ptr(), u64p!(o));
    saturating_pow256, 32, |x, o| saturating_pow256_c(SMALL.as_ptr(), E3.as_ptr(), u64p!(o));
    // bigint
    modexp_u64, 32, |x, o| modexp_u64_c(A.as_ptr(), 4, E.as_ptr(), 1, M.as_ptr(), 4, u64p!(o));
    modexp_u64_long, 64, |x, o| modexp_u64_c(A.as_ptr(), 4, E.as_ptr(), 1, M8.as_ptr(), 8, u64p!(o));
    // secp256k1
    jacobian_to_affine_secp256k1, 64, |x, o| jacobian_to_affine_secp256k1_c(x.secp_jac.as_ptr(), u64p!(o));
    lift_x_secp256k1, 64, |x, o| lift_x_secp256k1_c(x.secp_gx_be.as_ptr(), 0, u64p!(o));
    double_scalar_mul_with_g_secp256k1, 64, |x, o|
        double_scalar_mul_with_g_secp256k1_c(A.as_ptr(), B.as_ptr(), SECP_G.as_ptr(), u64p!(o));
    reduce_fp_secp256k1, 32, |x, o| reduce_fp_secp256k1_c(A.as_ptr(), u64p!(o));
    add_fp_secp256k1, 32, |x, o| add_fp_secp256k1_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    neg_fp_secp256k1, 32, |x, o| neg_fp_secp256k1_c(A.as_ptr(), u64p!(o));
    sub_fp_secp256k1, 32, |x, o| sub_fp_secp256k1_c(B.as_ptr(), A.as_ptr(), u64p!(o));
    mul_fp_secp256k1, 32, |x, o| mul_fp_secp256k1_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    square_fp_secp256k1, 32, |x, o| square_fp_secp256k1_c(A.as_ptr(), u64p!(o));
    sqrt_fp_secp256k1, 32, |x, o| sqrt_fp_secp256k1_c(x.secp_y2.as_ptr(), 0, u64p!(o));
    inv_fp_secp256k1, 32, |x, o| inv_fp_secp256k1_c(A.as_ptr(), u64p!(o));
    reduce_fn_secp256k1, 32, |x, o| reduce_fn_secp256k1_c(A.as_ptr(), u64p!(o));
    add_fn_secp256k1, 32, |x, o| add_fn_secp256k1_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    neg_fn_secp256k1, 32, |x, o| neg_fn_secp256k1_c(A.as_ptr(), u64p!(o));
    sub_fn_secp256k1, 32, |x, o| sub_fn_secp256k1_c(B.as_ptr(), A.as_ptr(), u64p!(o));
    mul_fn_secp256k1, 32, |x, o| mul_fn_secp256k1_c(A.as_ptr(), B.as_ptr(), u64p!(o));
    square_fn_secp256k1, 32, |x, o| square_fn_secp256k1_c(A.as_ptr(), u64p!(o));
    inv_fn_secp256k1, 32, |x, o| inv_fn_secp256k1_c(A.as_ptr(), u64p!(o));
    // bn254
    jacobian_to_affine_bn254, 64, |x, o| jacobian_to_affine_bn254_c(x.bn_jac.as_ptr(), u64p!(o));
    add_bn254, 64, |x, o| add_bn254_c(x.bn_g1.as_ptr(), x.bn_g1.as_ptr(), u64p!(o));
    scalar_mul_bn254, 64, |x, o| scalar_mul_bn254_c(x.bn_g1.as_ptr(), A.as_ptr(), u64p!(o));
    jacobian_to_affine_twist_bn254, 128, |x, o| jacobian_to_affine_twist_bn254_c(x.bn_twist_jac.as_ptr(), u64p!(o));
    // bls12-381
    decompress_bls12_381, 96, |x, o| decompress_bls12_381_c(x.bls_g1_comp.as_ptr(), u64p!(o));
    add_bls12_381, 96, |x, o| add_bls12_381_c(BLS_G1.as_ptr(), BLS_G1.as_ptr(), u64p!(o));
    scalar_mul_bls12_381, 96, |x, o| scalar_mul_bls12_381_c(BLS_G1.as_ptr(), B.as_ptr(), u64p!(o));
    final_exp_bls12_381, 576, |x, o| final_exp_bls12_381_c(x.fp12.as_ptr(), u64p!(o));
    mul_fp12_bls12_381, 576, |x, o| mul_fp12_bls12_381_c(x.fp12.as_ptr(), x.fp12.as_ptr(), u64p!(o));
    miller_loop_bls12_381, 576, |x, o| miller_loop_bls12_381_c(BLS_G1.as_ptr(), BLS_G2.as_ptr(), u64p!(o));
    decompress_twist_bls12_381, 192, |x, o| decompress_twist_bls12_381_c(x.bls_g2_comp.as_ptr(), u64p!(o));
    add_twist_bls12_381, 192, |x, o| add_twist_bls12_381_c(BLS_G2.as_ptr(), BLS_G2.as_ptr(), u64p!(o));
    scalar_mul_twist_bls12_381, 192, |x, o| scalar_mul_twist_bls12_381_c(BLS_G2.as_ptr(), B.as_ptr(), u64p!(o));
    // hashes
    keccak256_native, 32, |x, o| native_keccak256(x.hb.as_ptr(), 300, o);
    sha256f_compress, 32, |x, o| {
        core::ptr::copy_nonoverlapping(x.hb.as_ptr(), o, 32);
        sha256f_compress_c(o as *mut u32, x.blocks.as_ptr(), 2)
    };
    // EF zkVM accelerator ABI
    ef_keccak256, 32, |x, o| zkvm_keccak256(x.hb.as_ptr(), 300, o);
    ef_sha256, 32, |x, o| zkvm_sha256(x.hb.as_ptr(), 300, o);
    ef_ripemd160, 32, |x, o| zkvm_ripemd160(x.hb.as_ptr(), 300, o);
    ef_modexp, 48, |x, o| zkvm_modexp(x.hb.as_ptr(), 37, x.hb.as_ptr().add(40), 19, x.hb.as_ptr().add(64), 48, o);
    ef_bn254_g1_add, 64, |x, o| zkvm_bn254_g1_add(x.ef_bn_g1.as_ptr(), x.ef_bn_g1.as_ptr(), o);
    ef_bn254_g1_mul, 64, |x, o| zkvm_bn254_g1_mul(x.ef_bn_g1.as_ptr(), x.ef_bn_k.as_ptr(), o);
    ef_blake2f, 64, |x, o| {
        core::ptr::copy_nonoverlapping(x.hb.as_ptr(), o, 64);
        zkvm_blake2f(12, o, x.ef_bm.as_ptr(), x.ef_bt.as_ptr(), 1)
    };
    ef_bls12_g1_add, 96, |x, o| zkvm_bls12_g1_add(x.ef_g1.as_ptr(), x.ef_g1.as_ptr(), o);
    ef_bls12_g1_msm, 96, |x, o| zkvm_bls12_g1_msm(x.ef_g1_pair.as_ptr(), 1, o);
    ef_bls12_g2_add, 192, |x, o| zkvm_bls12_g2_add(x.ef_g2.as_ptr(), x.ef_g2.as_ptr(), o);
    ef_bls12_g2_msm, 192, |x, o| zkvm_bls12_g2_msm(x.ef_g2_pair.as_ptr(), 1, o);
    ef_bls12_map_fp_to_g1, 96, |x, o| zkvm_bls12_map_fp_to_g1(x.ef_fp.as_ptr(), o);
    ef_bls12_map_fp2_to_g2, 192, |x, o| zkvm_bls12_map_fp2_to_g2(x.ef_fp2.as_ptr(), o);
    ef_secp256k1_ecrecover, 64, |x, o| zkvm_secp256k1_ecrecover(x.ef_rmsg.as_ptr(), x.ef_rsig.as_ptr(), 1, o);
    ef_mulmod256, 32, |x, o| zkvm_mulmod256(x.ba.as_ptr(), x.bb.as_ptr(), x.bm.as_ptr(), o);
    ef_reduce_mod256, 32, |x, o| zkvm_reduce_mod256(x.ba.as_ptr(), x.bm.as_ptr(), o);
    ef_add_mod256, 32, |x, o| zkvm_add_mod256(x.ba.as_ptr(), x.bb.as_ptr(), x.bm.as_ptr(), o);
    ef_square_mod256, 32, |x, o| zkvm_square_mod256(x.ba.as_ptr(), x.bm.as_ptr(), o);
    ef_pow_mod256, 32, |x, o| zkvm_pow_mod256(x.ba.as_ptr(), x.be.as_ptr(), x.bm.as_ptr(), o);
    ef_inv_mod256, 32, |x, o| zkvm_inv_mod256(x.bb.as_ptr(), x.bm.as_ptr(), o);
    // EVM: the JUMPDEST bitmap of 100 bytecode bytes, 2 words (raw-pointer forms)
    jump_dest_ptr, 16, |x, o| ziskos::ziskos_jump_dest!(ptr: o as *mut u64, x.code.as_ptr() as *const u8, 100usize);
    jump_dest_dst_ptr, 16, |x, o| {
        let code = core::slice::from_raw_parts(x.code.as_ptr() as *const u8, 128);
        ziskos::ziskos_jump_dest!(dst_ptr: o as *mut u64, code, 100usize)
    };
}

// ---- the check -----------------------------------------------------------------
const MAX_LEN: usize = 576;
const GUARD: usize = 8;
const FILL: u8 = 0xA5;

/// (bit `o` set when offset `o` wrote the aligned result's bytes and nothing else,
/// the aligned call's return value)
fn check(x: &Inputs, f: CaseFn, len: usize) -> (u8, u64) {
    let mut reference = [0u64; MAX_LEN / 8];
    let r0 = unsafe { f(x, reference.as_mut_ptr() as *mut u8) };
    let e = unsafe { core::slice::from_raw_parts(reference.as_ptr() as *const u8, len) };
    let mut buf = [0u64; (MAX_LEN + 2 * GUARD) / 8];
    let mut mask = 0u8;
    for o in 0..8 {
        let b = buf.as_mut_ptr() as *mut u8;
        let n = core::mem::size_of_val(&buf);
        unsafe { core::ptr::write_bytes(b, FILL, n) };
        let r = unsafe { f(x, black_box(b.add(GUARD + o))) };
        let got = unsafe { core::slice::from_raw_parts(b, n) };
        let lo = GUARD + o;
        let ok = r == r0
            && got.iter().enumerate().all(|(i, &v)| {
                let want = if i >= lo && i < lo + len { e[i - lo] } else { FILL };
                v == want
            });
        mask |= (ok as u8) << o;
    }
    (mask, r0)
}

fn main() {
    let first: u64 = ziskos::io::read();
    let last: u64 = ziskos::io::read();
    let x = setup();
    let mut all_ok = true;
    for (n, &(name, len, f)) in CASES.iter().enumerate() {
        if (n as u64) < first || (n as u64) > last {
            continue;
        }
        let (mask, r0) = check(&x, f, len);
        println!("case {n} {name} ret={r0} mask={mask:#04x}");
        all_ok &= mask == 0xff;
    }
    ziskos::io::commit(&(all_ok as u64));
    assert!(all_ok, "an unaligned output was written wrong (see the case lines above)");
}
