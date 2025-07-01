// This example program takes a number `n` as input and computes the Keccak-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

use byteorder::ByteOrder;
use std::convert::TryInto;
use tiny_keccak::{Hasher, Keccak};
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

    let mut hash = [0u8; 32];

    // Compute Keccak-256 hashing 'n' times
    for _ in 0..n {
        let mut hasher = Keccak::v256();
        hasher.update(&hash);
        hasher.finalize(&mut hash);
    }

    // Split 'hash' value into chunks of 32 bits and write them to ziskos output
    for i in 0..8 {
        let val = byteorder::BigEndian::read_u32(&mut hash[i * 4..i * 4 + 4]);
        set_output(i, val);
    }
}
