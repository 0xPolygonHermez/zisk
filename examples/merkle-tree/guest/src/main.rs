//! Merkle-tree guest program: the computation executed inside the zkVM for
//! which a zero-knowledge proof is generated.
//!
//! The guest reads a leaf count `n` from its standard input, builds `n` SHA-256
//! leaves from the sequence `1..=n`, computes the Merkle root using the
//! `sha2`-based implementation, and commits the 32-byte root as a public value
//! of the proof.

#![no_main]
ziskos::entrypoint!(main);

use merkle_common::{hex, merkle_root, Digest, Hash, Sha256};

fn main() {
    // Read the number of leaves from the guest's standard input stream.
    let n: u64 = ziskos::io::read::<u64>();

    // Build leaves by hashing each index in the range 1..=n.
    let leaves: Vec<Hash> = (1..=n).map(|i| Sha256::digest(&i.to_le_bytes()).into()).collect();

    // Compute the Merkle root over the leaf set.
    let root = merkle_root(leaves);

    // Commit the root as a public output so a verifier can inspect it
    // without re-executing the program.
    ziskos::io::commit_slice(&root);

    println!("merkle-root({n}) => 0x{}", hex::encode(root));
}
