//! Collatz guest program (compressed output): the computation executed inside
//! the zkVM for which a zero-knowledge proof is generated.
//!
//! The guest reads a single [`u64`] value `n` from its standard input stream,
//! computes the Collatz sequence starting at `n`, and commits a SHA-256 digest
//! of the input concatenated with every sequence element as a compact public
//! value of the proof. The host can then verify that commitment without
//! re-running the program.

#![no_main]
ziskos::entrypoint!(main);

use collatz_common::{collatz, hex, Digest, Hash, Sha256};

fn main() {
    // Read the starting value from the guest's standard input stream.
    let input = ziskos::io::read::<u64>();

    // Compute the Collatz sequence we want to prove.
    let sequence = collatz(input);

    // Hash the input followed by every sequence element to produce a compact
    // digest that represents the full computation.
    let mut hasher = Sha256::new();
    hasher.update(input.to_le_bytes());
    sequence.iter().for_each(|value| hasher.update(value.to_le_bytes()));
    let result: Hash = hasher.finalize().into();

    // Commit the digest as a public output so a verifier can inspect it
    // without re-executing the program.
    ziskos::io::commit_slice(&result);

    println!("collatz({input}) => {:?} [digest: {:?}]", sequence, hex::encode(result));
}
