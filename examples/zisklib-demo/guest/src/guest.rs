//! Demo guest: calls a function implemented in the ZisK library (`ziskasm/lib/`)
//! rather than in Rust. The `ziskos_add` stub (see `ziskos.rs`) is redirected by
//! the transpiler to the hand-written `zisklib_add` routine, which runs as ZisK
//! instructions in the guest's place. The committed result (7) proves the
//! redirect happened — the stub's own body would return 0xBAD.

#![no_main]

ziskos::entrypoint!(main);

use core::hint::black_box;
use zisklib::{inv256, keccak256, ziskos_add};

/// keccak256 via the ziskasm-backed wrapper (`zisklib::keccak256` → redirected
/// `zisklib_keccak`), checked against the reference ziskos sponge.
fn keccak_matches(input: &[u8]) -> bool {
    let reference = ziskos::zisklib::keccak256(input);
    let mine = keccak256(input);
    mine == reference
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
    let empty_ok = keccak_matches(&[]);
    let words: [u64; 18] = core::array::from_fn(|i| (i as u64).wrapping_mul(0x0101_0101_0101_0101));
    let big: &[u8] = unsafe { core::slice::from_raw_parts(words.as_ptr() as *const u8, 18 * 8) };
    let big_ok = keccak_matches(big);

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
