//! Primes guest program: the computation executed inside the zkVM for which a
//! zero-knowledge proof is generated.
//!
//! The guest reads a single [`InputDTO`] from its standard input stream,
//! computes the sum of the values that are prime and commits a [`u64`] result
//! as a public value of the proof. The host can then verify that commitment without
//! re-running the program.

#![no_main]
ziskos::entrypoint!(main);

use primes_common::{is_prime, InputDTO};

fn main() {
    // Read the input from the guest's standard input.
    let input = ziskos::io::read::<InputDTO>();

    // Compute the value we want to prove.
    let mut result = 0u64;
    for value in &input.values {
        if is_prime(value) {
            result += value;
        }
    }

    // Commit the result as a public output so a verifier can inspect it
    // without re-executing the program.
    ziskos::io::commit(&result);

    println!("sum-primes({:?}) => {result}", input.values);
}
