//! Fibonacci guest program: the computation executed inside the zkVM for which a
//! zero-knowledge proof is generated.
//!
//! The guest reads a single [`u8`] index `n` from its standard input,
//! computes `fibonacci(n)` and commits the [`fibonacci_common::U256`] result as a public
//! value of the proof. The host can then verify that commitment without
//! re-running the program.

#![no_main]

ziskos::entrypoint!(main);

use fibonacci_common::fibonacci;

fn main() {
    // Read the input from the guest's standard input.
    let input = ziskos::io::read::<u8>();

    // Compute the value we want to prove.
    let result = fibonacci(input);

    // Commit the result as a public output so a verifier can inspect it
    // without re-executing the program.
    ziskos::io::commit(&result);

    println!("fibonacci({input}) => {result}");
}
