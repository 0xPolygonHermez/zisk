extern crate libc;

mod asm_runner;

use std::path::Path;

use asm_runner::{AsmRunner, AsmRunnerOptions};

fn main() {
    let inputs_path =
        Path::new("../zisk-testvectors/pessimistic-proof/inputs/pessimistic-proof.bin");
    let ziskemuasm_path = Path::new("emulator-asm/build/ziskemuasm");

    let runner_options = AsmRunnerOptions {
        log_output: true,
        metrics: true,
        verbose: false,
        trace_level: asm_runner::AsmTraceLevel::None,
        keccak_trace: false,
    };
    let _ = AsmRunner::run(inputs_path, ziskemuasm_path, 1 << 32, 1 << 20, runner_options);

    println!("Done!");

    // for i in 0..result.vec_chunks.len() {
    //     println!("Chunk {}: {:#x?}", i, result.vec_chunks[i]);
    // }
}
