#![no_main]
ziskos::entrypoint!(main);

// Matched keccak256 A/B. Both backends call the same ZisK keccak-f precompile;
// only the sponge wrapper differs: compiled Rust zisklib vs the hand-written
// `.zisk` routine (redirected at transpile time).
#[cfg(not(feature = "ziskasm"))]
use ziskos::zisklib::keccak256;
#[cfg(feature = "ziskasm")]
use zisklib::keccak256;

fn main() {
    let mut buf = [0u8; 200];
    for i in 0..200 {
        buf[i] = i as u8;
    }
    // Chain 2000 hashes so the per-call cost dominates fixed overhead.
    let mut h = [0u8; 32];
    for _ in 0..2000 {
        h = keccak256(&buf);
        buf[..32].copy_from_slice(&h);
    }
    println!("keccak_ab {:02x}{:02x}", h[0], h[31]);
}
