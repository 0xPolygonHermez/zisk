#![no_main]
ziskos::entrypoint!(main);

use ziskos::zisklib::{blake2b_compress, keccak256, ripemd160, sha256};

/// Iterations per hash. Cost scales linearly with this; tune to trade benchmark
/// runtime against signal strength.
const ITERS: u64 = 1000;

fn main() {
    run_keccak256(ITERS);
    run_sha256(ITERS);
    run_ripemd160(ITERS);
    run_blake2b(ITERS);
}

fn run_keccak256(iters: u64) {
    let mut data = [0u8; 32];
    for _ in 0..iters {
        data = keccak256(&data);
    }
}

fn run_sha256(iters: u64) {
    let mut data = [0u8; 32];
    for _ in 0..iters {
        data = sha256(&data);
    }
}

fn run_ripemd160(iters: u64) {
    let mut data = [0u8; 32];
    for _ in 0..iters {
        data = ripemd160(&data);
    }
}

fn run_blake2b(iters: u64) {
    // Standard 12-round Blake2b compression over a fixed message block; the hash
    // state `h` evolves in place across iterations.
    let rounds = 12;
    let mut h: [u64; 8] = [0x6a09_e667_f3bc_c908, 0, 0, 0, 0, 0, 0, 0];
    let m = [0u64; 16];
    let t = [0u64; 2];
    let f = false;
    for _ in 0..iters {
        blake2b_compress(rounds, &mut h, &m, &t, f);
    }
}
