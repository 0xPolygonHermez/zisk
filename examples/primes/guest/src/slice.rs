//! Primes guest program: the computation executed inside the zkVM for which a
//! zero-knowledge proof is generated.
//!
//! The guest reads a single [`slice`] from its standard input stream, desarializes it
//! using rykv, computes the sum of the values that are prime and commits a [`u64`] result
//! as a public value of the proof. The host can then verify that commitment without
//! re-running the program.

#![no_main]
ziskos::entrypoint!(main);

use primes_common::{InputZeroCopyDTO, is_prime, rkyv};

fn main() {
    // Read the input from the guest's standard input.
    let raw_input = ziskos::io::read_slice();

    // Zero-copy view: no deserialization, no allocation
    let input =
        rkyv::api::high::from_bytes::<InputZeroCopyDTO, rkyv::rancor::Error>(raw_input.as_ref())
            .unwrap();

    // Compute the value we want to prove.
    let mut result = 0;
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
