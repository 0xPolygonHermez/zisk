//! Leaf guest: reads 12 private u64s, commits only their Poseidon1 digest — so
//! the proof attests knowledge of a preimage without revealing it.
#![no_main]
ziskos::entrypoint!(main);

use recurser_hash_common::{hash12, RATE};

fn main() {
    let secret: [u64; RATE] = ziskos::io::read();

    // Commit each digest field element as two little-endian u32 limbs (8 slots).
    for &elem in hash12(&secret).iter() {
        ziskos::io::commit_slice(&elem.to_le_bytes());
    }
}
