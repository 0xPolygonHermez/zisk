#![no_main]
ziskos::entrypoint!(main);

use std::fs;

use bytemuck::cast_slice;
use ziskos::set_output;

fn main() {
    // TODO: DETERMINE HOW TO PASS THE PROOF

    let buffer = fs::read("proofs/vadcop_final_proof.bin").unwrap();
    let proof_slice: &[u64] = cast_slice(&buffer);

    // Verify the proof
    let valid = verifier::verify(proof_slice);
    set_output(0, valid.into());
}
