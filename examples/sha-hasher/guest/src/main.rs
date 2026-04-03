// This example program takes a number `n` as input and computes the SHA-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

use sha2::{Digest, Sha256};
use alloy_sol_types::{sol, SolValue};

sol! {
    struct Output {
        bytes32 hash;
        uint32 iterations;
        uint32 magic_number;
    }
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

    let output = Output { hash: hash.into(), iterations: n, magic_number: 0xDEADBEEF };

    println!("Computed hash: {:02x?}", output.hash);
    println!("Iterations: {}", output.iterations);

    let bytes = output.abi_encode();

    println!("Bytes to commit: {:?}", bytes);
    
    ziskos::io::commit(&bytes);
}
