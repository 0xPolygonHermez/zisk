//! Collatz guest program (multiple outputs): the computation executed inside
//! the zkVM for which a zero-knowledge proof is generated.
//!
//! The guest reads a single [`u64`] value `n` from its standard input stream,
//! computes the Collatz sequence starting at `n`, and commits both the input
//! and the full sequence as separate public values of the proof. The host can
//! then verify those commitments without re-running the program.

#![no_main]
ziskos::entrypoint!(main);

use collatz_common::collatz;

fn main() {
    // Read the starting value from the guest's standard input stream.
    let input = ziskos::io::read::<u64>();

    // Compute the Collatz sequence we want to prove.
    let sequence = collatz(input);

    // Commit the input and the full sequence as separate public outputs so a
    // verifier can inspect them without re-executing the program.
    ziskos::io::commit(&input);
    ziskos::io::commit(&sequence);

    println!("collatz({input}) => {:?}", sequence);
}
