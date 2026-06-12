//! Primes guest program: the computation executed inside the zkVM for which a
//! zero-knowledge proof is generated.
//!
//! The guest reads a [`u64`] `len`, then reads `len` [`u64`] values one per
//! iteration, sums those that are prime, and commits the result as a public
//! value of the proof. The host can verify that commitment without re-running
//! the program.

#![no_main]
ziskos::entrypoint!(main);

use primes_common::is_prime;

fn main() {
    // Read the length from the guest's standard input stream.
    let len = ziskos::io::read::<u64>();

    print!("sum-primes([");
    // Reading each value fro mthe guest standard inputs stream and adding it if its prime
    let mut result = 0;
    for i in 0..len {
        let value = ziskos::io::read::<u64>();
        if i == 0 {
            print!("{value}");
        } else {
            print!(",{value}");
        }
        if is_prime(&value) {
            result += value;
        }
    }

    // Commit the result as a public output so a verifier can inspect it
    // without re-executing the program.
    ziskos::io::commit(&result);

    println!("]) => {result}");
}
