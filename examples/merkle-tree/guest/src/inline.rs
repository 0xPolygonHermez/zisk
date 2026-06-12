//! Merkle-tree guest program (profiling variant).
//!
//! Same logic as `main.rs` but wraps the two main phases — leaf preparation
//! and root computation — with `profile_report_start!/end!` markers so the
//! ZisK profiler can attribute cycles to each phase independently.

#![no_main]
ziskos::entrypoint!(main);

use merkle_common::{hex, merkle_root, Digest, Hash, Sha256};

fn main() {
    // Read the number of leaves from the guest's standard input stream.
    let n: u64 = ziskos::io::read::<u64>();

    // Profile the leaf-preparation phase separately from the root computation.
    ziskos::profile_report_start!(PREPARE_LEAVES);
    let leaves: Vec<Hash> = (1..=n).map(|i| Sha256::digest(i.to_le_bytes()).into()).collect();
    ziskos::profile_report_end!(PREPARE_LEAVES);

    // Profile the Merkle-root computation phase.
    ziskos::profile_report_start!(COMPUTE_ROOT);
    let root = merkle_root(leaves);
    ziskos::profile_report_end!(COMPUTE_ROOT);

    // Commit the root as a public output so a verifier can inspect it
    // without re-executing the program.
    ziskos::io::commit_slice(&root);

    println!("merkle-root({n}) => 0x{}", hex::encode(root));
}
