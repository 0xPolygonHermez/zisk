//! Demo guest: calls a function implemented in the ZisK library (`ziskasm/lib/`)
//! rather than in Rust. The `ziskos_add` stub (see `ziskos.rs`) is redirected by
//! the transpiler to the hand-written `zisklib_add` routine, which runs as ZisK
//! instructions in the guest's place. The committed result (7) proves the
//! redirect happened — the stub's own body would return 0xBAD.

#![no_main]

ziskos::entrypoint!(main);

use core::hint::black_box;
use zisklib::{inv256, keccak256, ziskos_add};

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

/// keccak256 via the ziskasm-backed wrapper (`zisklib::keccak256` → redirected
/// `zisklib_keccak`), checked against a hardcoded expected digest.
fn keccak_matches(input: &[u8], expected: &[u8; 32]) -> bool {
    &keccak256(input) == expected
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

    let ok = sum == 7 && empty_ok && big_ok && inv_ok && noinv_ok;
    ziskos::io::commit(&ok);
    println!(
        "add=0x{sum:x} keccak(empty)={empty_ok} keccak(144B)={big_ok} inv256={inv_ok} noinv={noinv_ok} => ok={ok}"
    );
}
