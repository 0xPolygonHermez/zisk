//! Merkle-tree guest program (ZisK built-in SHA-256 variant).
//!
//! Identical to `main.rs` but uses `ziskos::zisklib::sha256` for leaf hashing
//! and delegates the inner hash of each tree level to the same built-in via
//! [`merkle_common::merkle_root_zisklib`]. This variant benchmarks the savings from
//! the optimised ROM operation compared to the user-space `sha2` crate.

#![no_main]
ziskos::entrypoint!(main);

use merkle_common::{hex, merkle_root_zisklib, Hash};

fn main() {
    // Read the number of leaves from the guest's standard input stream.
    let n: u64 = ziskos::io::read::<u64>();

    // Build leaves using the ZisK Lib built-in SHA-256 operation.
    let leaves: Vec<Hash> = (1..=n).map(|i| ziskos::zisklib::sha256(&i.to_le_bytes())).collect();

    // Compute the Merkle root using the built-in SHA-256 variant.
    let root = merkle_root_zisklib(leaves);

    // Commit the root as a public output so a verifier can inspect it
    // without re-executing the program.
    ziskos::io::commit_slice(&root);

    println!("merkle-root({n}) => 0x{}", hex::encode(root));
}
