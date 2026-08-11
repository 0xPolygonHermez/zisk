//! Demo guest: calls a function implemented in the ZisK library (`ziskasm/zisklib/`)
//! rather than in Rust. The `ziskos_add` stub (see `ziskos.rs`) is redirected by
//! the transpiler to the hand-written `zisklib_add` routine, which runs as ZisK
//! instructions in the guest's place. The committed result (7) proves the
//! redirect happened — the stub's own body would return 0xBAD.

#![no_main]

ziskos::entrypoint!(main);

use core::hint::black_box;
use zisklib::{
    add_mod256, blake2b_compress, checked_add256, checked_div256, checked_mul256, checked_pow256,
    checked_square256, checked_sub256, div_ceil256, div_rem256, inv256, inv_mod256, keccak256,
    mul_mod256, overflowing_add256, overflowing_mul256, overflowing_pow256, pow_mod256,
    reduce_mod256, saturating_pow256, saturating_sub256, sha256, square_mod256, wrapping_add256,
    wrapping_mul256, wrapping_neg256, wrapping_pow256, wrapping_rem256, wrapping_square256,
    wrapping_sub256, ziskos_add,
};

// Hardcoded expected keccak256 digests (independent of any keccak API), so the
// test is self-contained. keccak256("") is the canonical empty-string vector.
const KECCAK_EMPTY: [u8; 32] = [
    0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
    0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
];
// keccak256 of the 144-byte input built below (byte value i repeated 8 times, i = 0..17).
const KECCAK_BIG144: [u8; 32] = [
    0x52, 0xa0, 0x48, 0xee, 0x61, 0x22, 0x1c, 0x82, 0x92, 0xa1, 0x24, 0x59, 0x91, 0xe1, 0x22, 0x27,
    0x69, 0x41, 0x71, 0x29, 0x5a, 0xab, 0xc4, 0x03, 0xf0, 0x15, 0xe9, 0xc9, 0x57, 0x2c, 0x5e, 0xbd,
];
// keccak256 of the 13-byte input [0,1,..,12] — a non-multiple-of-8 length.
const KECCAK_13: [u8; 32] = [
    0xa1, 0xb3, 0x65, 0xd4, 0x5c, 0x3c, 0xde, 0x59, 0xc2, 0x47, 0xb8, 0x1f, 0xcf, 0xee, 0xb1, 0xc5,
    0x84, 0xcd, 0xad, 0x57, 0x3d, 0xfc, 0x2e, 0x3b, 0xb3, 0x2d, 0x42, 0x25, 0xd6, 0xbf, 0xe9, 0x89,
];

// SHA-256 digests (FIPS 180-4 test vectors): the empty string, "abc", and 56
// bytes of 'a' (which forces the two-block final-padding path).
const SHA256_EMPTY: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];
const SHA256_ABC: [u8; 32] = [
    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
    0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
];
const SHA256_56A: [u8; 32] = [
    0xb3, 0x54, 0x39, 0xa4, 0xac, 0x6f, 0x09, 0x48, 0xb6, 0xd6, 0xf9, 0xe3, 0xc6, 0xaf, 0x0f, 0x5f,
    0x59, 0x0c, 0xe2, 0x0f, 0x1b, 0xde, 0x70, 0x90, 0xef, 0x79, 0x70, 0x68, 0x6e, 0xc6, 0x73, 0x8a,
];

/// keccak256 via the ziskasm-backed wrapper (`zisklib::keccak256` → redirected
/// `zisklib_keccak`), checked against a hardcoded expected digest.
fn keccak_matches(input: &[u8], expected: &[u8; 32]) -> bool {
    &keccak256(input) == expected
}

/// SHA-256 via `zisklib::sha256` (→ redirected `zisklib_sha256`), checked against
/// a hardcoded expected digest.
fn sha256_matches(input: &[u8], expected: &[u8; 32]) -> bool {
    &sha256(input) == expected
}

fn main() {
    // 1. Simple function: `black_box` keeps args opaque so the call is real.
    let a = black_box(3u64);
    let b = black_box(4u64);
    let sum = ziskos_add(a, b);

    // 2. keccak256, checked against the reference ziskos implementation. Inputs
    // are 8-byte aligned with len % 8 == 0 (the current zisklib_keccak constraint):
    // the empty message (padding-only, one permutation) and a 144-byte message
    // (one full rate block + a final block, exercising the absorb loop).
    let empty_ok = keccak_matches(&[], &KECCAK_EMPTY);
    let words: [u64; 18] = core::array::from_fn(|i| (i as u64).wrapping_mul(0x0101_0101_0101_0101));
    let big: &[u8] = unsafe { core::slice::from_raw_parts(words.as_ptr() as *const u8, 18 * 8) };
    let big_ok = keccak_matches(big, &KECCAK_BIG144);
    // Non-multiple-of-8 length (13 bytes): exercises the byte-level final block.
    let odd: [u8; 13] = core::array::from_fn(|i| i as u8);
    let odd_ok = keccak_matches(&odd, &KECCAK_13);

    // 2b. SHA-256: empty, "abc", and 56×'a' (the two-block padding boundary).
    let a56: [u8; 56] = [b'a'; 56];
    let sha_ok = sha256_matches(&[], &SHA256_EMPTY)
        && sha256_matches(b"abc", &SHA256_ABC)
        && sha256_matches(&a56, &SHA256_56A);

    // 2c. BLAKE2b compression: one final compression of the empty message
    // reproduces blake2b512("") = 786a02f7...9be2ce, checking the state ends up
    // equal to that digest's eight little-endian words.
    let mut h = black_box([
        0x6A09_E667_F2BD_C948, // IV[0] ^ 0x01010040 (nn=64, kk=0)
        0xBB67_AE85_84CA_A73B,
        0x3C6E_F372_FE94_F82B,
        0xA54F_F53A_5F1D_36F1,
        0x510E_527F_ADE6_82D1,
        0x9B05_688C_2B3E_6C1F,
        0x1F83_D9AB_FB41_BD6B,
        0x5BE0_CD19_137E_2179,
    ]);
    blake2b_compress(12, &mut h, &black_box([0u64; 16]), &black_box([0u64; 2]), true);
    let blake2b_ok = h
        == [
            0x0359_0142_F702_6A78,
            0x72D2_5225_85FD_C6C6,
            0x6147_58E1_4047_2F91,
            0x1954_1FF7_17E2_868A,
            0x5358_EEAF_3110_5ED2,
            0x4BB0_4E93_4464_8913,
            0x55B7_4814_5B68_3A90,
            0xCEE2_9BFE_1A70_6FD5,
        ];

    // 3. inv256: an odd input is invertible (the ziskasm routine verifies
    // a*inv ≡ 1 mod 2^256 before returning); an even input is not.
    let inv_expected = [
        0xaaaa_aaaa_aaaa_aaab,
        0xaaaa_aaaa_aaaa_aaaa,
        0xaaaa_aaaa_aaaa_aaaa,
        0xaaaa_aaaa_aaaa_aaaa,
    ];
    let inv_ok = inv256(&black_box([3u64, 0, 0, 0])) == Some(inv_expected);
    let noinv_ok = inv256(&black_box([2u64, 0, 0, 0])).is_none();

    // 4. 256-bit add/sub/neg (all derived from the two overflowing cores).
    let max = [u64::MAX; 4];
    let three = black_box([3u64, 0, 0, 0]);
    let five = black_box([5u64, 0, 0, 0]);
    let addsub_ok = checked_add256(&black_box(max), &black_box([1, 0, 0, 0])).is_none()      // overflow
        && wrapping_add256(&black_box(max), &black_box([1, 0, 0, 0])) == [0, 0, 0, 0]         // wraps to 0
        && overflowing_add256(&black_box([5, 0, 0, 0]), &three) == ([8, 0, 0, 0], false)      // 5+3=8
        && checked_sub256(&three, &five).is_none()                                            // underflow
        && wrapping_sub256(&three, &five) == [0xffff_ffff_ffff_fffe, u64::MAX, u64::MAX, u64::MAX] // 3-5
        && saturating_sub256(&three, &five) == [0, 0, 0, 0]                                   // saturates to 0
        && wrapping_neg256(&black_box([1, 0, 0, 0])) == max;                                  // -1 = 2^256-1

    // 5. 256-bit mul/square (derived from the overflowing_mul256 core).
    let p128 = black_box([0u64, 0, 1, 0]); // 2^128
    let mul_ok = checked_mul256(&p128, &p128).is_none()                                      // 2^128·2^128 = 2^256 overflows
        && wrapping_mul256(&black_box([6, 0, 0, 0]), &black_box([7, 0, 0, 0])) == [42, 0, 0, 0]
        && overflowing_mul256(&p128, &p128) == ([0, 0, 0, 0], true)                          // low bits 0, overflow
        && wrapping_square256(&black_box([u64::MAX, 0, 0, 0])) == [1, 0xffff_ffff_ffff_fffe, 0, 0] // (2^64-1)^2
        && checked_square256(&p128).is_none();

    // 6. 256-bit div/rem (hint via fcall + arith256 verify + 256-bit compare).
    let hundred = black_box([100u64, 0, 0, 0]);
    let seven = black_box([7u64, 0, 0, 0]);
    let div_ok = div_rem256(&hundred, &seven) == ([14, 0, 0, 0], [2, 0, 0, 0])               // 100 = 7·14 + 2
        && checked_div256(&hundred, &black_box([0, 0, 0, 0])).is_none()                      // ÷0 -> None
        && wrapping_rem256(&hundred, &seven) == [2, 0, 0, 0]
        && div_ceil256(&hundred, &seven) == [15, 0, 0, 0]                                    // ceil(100/7) = 15
        && div_ceil256(&black_box([42, 0, 0, 0]), &seven) == [6, 0, 0, 0]                    // exact -> 6
        && div_rem256(&black_box([u64::MAX, u64::MAX, 0, 0]), &black_box([0, 1, 0, 0]))      // (2^128-1)/2^64
            == ([u64::MAX, 0, 0, 0], [u64::MAX, 0, 0, 0]);

    // 7. 256-bit modular arithmetic (arith256_mod precompile).
    let m7 = black_box([7u64, 0, 0, 0]);
    let p128 = black_box([0u64, 0, 1, 0]); // 2^128
    let p64 = black_box([0u64, 1, 0, 0]); // 2^64
    let mod_ok = reduce_mod256(&black_box([100, 0, 0, 0]), &m7) == [2, 0, 0, 0]              // 100 mod 7
        && reduce_mod256(&black_box([3, 0, 0, 0]), &m7) == [3, 0, 0, 0]                       // already reduced
        && reduce_mod256(&black_box([5, 0, 0, 0]), &black_box([0, 0, 0, 0])) == [0, 0, 0, 0]  // modulus 0 -> 0
        && add_mod256(&black_box([6, 0, 0, 0]), &black_box([6, 0, 0, 0]), &m7) == [5, 0, 0, 0] // (6+6) mod 7
        && mul_mod256(&black_box([5, 0, 0, 0]), &black_box([4, 0, 0, 0]), &m7) == [6, 0, 0, 0] // (5·4) mod 7
        && mul_mod256(&p128, &p128, &p64) == [0, 0, 0, 0]                                      // 2^256 mod 2^64
        && square_mod256(&black_box([5, 0, 0, 0]), &m7) == [4, 0, 0, 0];                       // 25 mod 7

    // 8. 256-bit modular inverse (fcall hint + verify, or gcd witness for none).
    let invmod_ok = inv_mod256(&black_box([3, 0, 0, 0]), &m7) == Some([5, 0, 0, 0])           // 3·5 ≡ 1 (mod 7)
        && inv_mod256(&black_box([3, 0, 0, 0]), &black_box([11, 0, 0, 0])) == Some([4, 0, 0, 0]) // 3·4 ≡ 1 (mod 11)
        && inv_mod256(&black_box([4, 0, 0, 0]), &black_box([8, 0, 0, 0])).is_none()            // gcd(4,8)=4
        && inv_mod256(&black_box([6, 0, 0, 0]), &black_box([9, 0, 0, 0])).is_none();           // gcd(6,9)=3

    // 9. 256-bit exponentiation: modular (pow_mod) and mod-2^256 (pow) with overflow.
    let pow_ok = pow_mod256(&black_box([2, 0, 0, 0]), &black_box([100, 0, 0, 0]), &black_box([13, 0, 0, 0])) == [3, 0, 0, 0] // 2^100 mod 13
        && pow_mod256(&black_box([2, 0, 0, 0]), &black_box([10, 0, 0, 0]), &black_box([1000, 0, 0, 0])) == [24, 0, 0, 0]     // 2^10 mod 1000
        && pow_mod256(&black_box([7, 0, 0, 0]), &black_box([3, 0, 0, 0]), &black_box([1, 0, 0, 0])) == [0, 0, 0, 0]          // mod 1 -> 0
        && checked_pow256(&black_box([2, 0, 0, 0]), &black_box([10, 0, 0, 0])) == Some([1024, 0, 0, 0])
        && overflowing_pow256(&black_box([3, 0, 0, 0]), &black_box([5, 0, 0, 0])) == ([243, 0, 0, 0], false)
        && checked_pow256(&black_box([2, 0, 0, 0]), &black_box([256, 0, 0, 0])).is_none()                                    // 2^256 overflows
        && wrapping_pow256(&black_box([2, 0, 0, 0]), &black_box([256, 0, 0, 0])) == [0, 0, 0, 0]
        && saturating_pow256(&black_box([2, 0, 0, 0]), &black_box([256, 0, 0, 0])) == [u64::MAX; 4]
        && wrapping_pow256(&black_box([2, 0, 0, 0]), &black_box([255, 0, 0, 0])) == [0, 0, 0, 0x8000_0000_0000_0000];        // 2^255

    let ok = sum == 7
        && empty_ok
        && big_ok
        && odd_ok
        && sha_ok
        && blake2b_ok
        && inv_ok
        && noinv_ok
        && addsub_ok
        && mul_ok
        && div_ok
        && mod_ok
        && invmod_ok
        && pow_ok;
    ziskos::io::commit(&ok);
    println!(
        "add=0x{sum:x} keccak(empty)={empty_ok} keccak(144B)={big_ok} keccak(13B)={odd_ok} sha256={sha_ok} blake2b={blake2b_ok} inv256={inv_ok} noinv={noinv_ok} addsub={addsub_ok} mul={mul_ok} div={div_ok} mod={mod_ok} invmod={invmod_ok} pow={pow_ok} => ok={ok}"
    );
}
