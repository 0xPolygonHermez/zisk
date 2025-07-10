#![no_main]
ziskos::entrypoint!(main);

use bytemuck::cast_slice;
use ziskos::{set_output, read_input};

fn main() {
    let input: Vec<u8> = read_input();
    let proof_slice: &[u64] = cast_slice(&input);

    // Verify the proof
    let valid = verifier::verify(proof_slice);
    set_output(0, valid.into());
}
