//! Demo guest: calls a function implemented in the ZisK library (`ziskasm/lib/`)
//! rather than in Rust. The `ziskos_add` stub (see `ziskos.rs`) is redirected by
//! the transpiler to the hand-written `zisklib_add` routine, which runs as ZisK
//! instructions in the guest's place. The committed result (7) proves the
//! redirect happened — the stub's own body would return 0xBAD.

#![no_main]

ziskos::entrypoint!(main);

#[path = "ziskos.rs"]
mod stubs;

use core::hint::black_box;
use stubs::{ziskos_add, ziskos_keccak};

/// keccak256 via the redirected ziskasm `zisklib_keccak`, checked against the
/// reference ziskos sponge. Returns whether they match.
fn keccak_matches(input: &[u8]) -> bool {
    let reference = ziskos::zisklib::keccak256(input);
    let mut mine = [0u8; 32];
    // SAFETY: `input` is a valid slice; `mine` is 32 writable bytes.
    unsafe { ziskos_keccak(input.as_ptr(), input.len(), mine.as_mut_ptr()) };
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

    let ok = sum == 7 && empty_ok && big_ok;
    ziskos::io::commit(&ok);
    println!("add=0x{sum:x} keccak(empty)={empty_ok} keccak(144B)={big_ok} => ok={ok}");
}
