//! GCD guest program: the computation executed inside the zkVM for which a
//! zero-knowledge proof is generated.
//!
//! The guest reads two [`u64`] values `a` and `b` from its standard input
//! stream, computes `gcd(a, b)` using the Euclidean algorithm, and commits
//! the result as a public value of the proof. The host can then verify that
//! commitment without re-running the program.

#![no_main]

ziskos::entrypoint!(main);

use gcd_common::gcd;

fn main() {
    // Read the two inputs from the guest's standard input stream.
    let a = ziskos::io::read::<u64>();
    let b = ziskos::io::read::<u64>();

    // Compute the value we want to prove.
    let result = gcd(a, b);

    // Commit the result as a public output so a verifier can inspect it
    // without re-executing the program.
    ziskos::io::commit(&result);

    println!("gcd({a}, {b}) => {result}");
}
