//! Reads a u32 `n` from stdin, computes SHA-256 hashed `n` times sequentially,
//! and commits the ABI-encoded `Output { hash, iterations, magic_number }`.

#![no_main]
ziskos::entrypoint!(main);

use alloy_sol_types::SolValue;
use sha2::{Digest, Sha256};
use sha_hasher_common::Output;

fn main() {
    let n: u32 = ziskos::io::read();

    let mut hash = [0u8; 32];

    for _ in 0..n {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let digest = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*digest);
    }

    let output = Output { hash: hash.into(), iterations: n, magic_number: 0xDEADBEEF };

    println!("Computed hash: {:02x?}", output.hash);
    println!("Iterations: {}", output.iterations);

    let bytes = output.abi_encode();
    println!("Bytes to commit: {:?}", bytes);

    ziskos::io::commit_slice(&bytes);
}
