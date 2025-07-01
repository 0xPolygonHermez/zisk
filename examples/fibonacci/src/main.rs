// This example program takes a number `n` as input and computes the nth Fibonacci number.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

use std::convert::TryInto;
use ziskos::{read_input, set_output};

fn main() {
    // Read the input data as a byte array from ziskos
    let input: Vec<u8> = read_input();

    // Convert the input data to a u64 integer
    let n: u64 = match input.try_into() {
        Ok(input_bytes) => u64::from_le_bytes(input_bytes),
        Err(input) => panic!(
            "Invalid input length. Expected 8 bytes, got {}",
            input.len()
        ),
    };

    // Compute the nth Fibonacci number
    let fib_result = fibonacci(n);

    // Output the Fibonacci result as two 32-bit values (low and high parts)
    let low = (fib_result & 0xFFFFFFFF) as u32;
    let high = ((fib_result >> 32) & 0xFFFFFFFF) as u32;

    set_output(0, low);
    set_output(1, high);

    // Set remaining outputs to 0
    for i in 2..8 {
        set_output(i, 0);
    }
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }

    let mut a = 0u64;
    let mut b = 1u64;

    for _ in 2..=n {
        let temp = a.wrapping_add(b);
        a = b;
        b = temp;
    }

    b
}
