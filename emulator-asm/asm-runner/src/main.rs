extern crate libc;

mod asm_runner;

use std::path::Path;

use asm_runner::AsmRunner;

fn main() {
    let inputs_path =
        Path::new("../zisk-testvectors/pessimistic-proof/inputs/pessimistic-proof.bin");
    let ziskemuasm_path = Path::new("emulator-asm/build/ziskemuasm");

    let result = AsmRunner::run(inputs_path, ziskemuasm_path);

    println!("Done!");

    for i in 0..result.vec_chunks.len() {
        println!("Chunk {}: {:#x?}", i, result.vec_chunks[i]);
    }
}
