// This example program takes a number `n` as input and computes the SHA-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Debug)]
struct Output {
    hash: [u8; 32],
    iterations: u32,
    magic_number: u32,
}

fn main() {
    // Read the input data
    let n: u32 = ziskos::io::read();

    let mut hash = [0u8; 32];

    // Compute SHA-256 hashing 'n' times
    for _ in 0..n {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let digest = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*digest);
    }

    let output = Output { hash, iterations: n, magic_number: 0xDEADBEEF };

    println!("Computed hash: {:02x?}", output.hash);
    println!("Iterations: {}", output.iterations);

    ziskos::io::commit(&output);
}
