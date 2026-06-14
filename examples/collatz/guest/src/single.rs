//! Collatz guest program (struct output): the computation executed inside the
//! zkVM for which a zero-knowledge proof is generated.
//!
//! The guest reads a single [`u64`] value `n` from its standard input stream,
//! computes the Collatz sequence starting at `n`, and commits an [`OutputDTO`]
//! containing both the input and the sequence as a single public value of the
//! proof. The host can then verify that commitment without re-running the
//! program.

#![no_main]
ziskos::entrypoint!(main);

use collatz_common::{OutputDTO, collatz};

fn main() {
    // Read the starting value from the guest's standard input stream.
    let input = ziskos::io::read::<u64>();

    // Compute the Collatz sequence we want to prove.
    let sequence = collatz(input);

    // Pack the input and sequence into a single struct and commit it as a
    // public output so a verifier can inspect it without re-executing the
    // program.
    let result = OutputDTO { n: input, sequence };
    ziskos::io::commit(&result);

    println!("collatz({input}) => {:?}", result.sequence);
}
