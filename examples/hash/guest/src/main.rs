//! Guest program for the hash example.
//!
//! This binary runs inside the ZiskVM. It reads a String from the
//! prover's standard input, computes its SHA-256 digest, and commits the
//! 32-byte result as a public value that will be included in the proof.

#![no_main]
ziskos::entrypoint!(main);

use hash_common::{Digest, Sha256, hex};

fn main() {
    // Reading input
    let input = ziskos::io::read::<String>();

    // Executing program logic
    let digest = Sha256::digest(&input);

    // Commiting outputs to proof public values
    ziskos::io::commit_slice(&digest);

    println!("sha256('{input}') => 0x{}", hex::encode(digest));
}
