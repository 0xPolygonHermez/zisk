extern crate libc;
mod asm_runner;

use std::path::PathBuf;
use clap::Parser;
use asm_runner::{AsmRunner, AsmRunnerOptions};

#[derive(Parser)]
#[command(version, about = "Zisk Asm Emulator Runner", long_about = None)]
struct Args {
    /// Path to the assembly runner binary path
    asm_runner_path: PathBuf,

    /// Path to the inputs file
    inputs_path: PathBuf,

}

fn main() {
    let args = Args::parse();

    let runner_options = AsmRunnerOptions {
        log_output: true,
        metrics: true,
        verbose: false,
        trace_level: asm_runner::AsmTraceLevel::None,
        keccak_trace: false,
    };
    let result = AsmRunner::run(&args.asm_runner_path, &args.inputs_path, 1 << 32, 1 << 15, runner_options);

    println!("Done!");

    for i in 0..result.vec_chunks.len() {
        println!("Chunk {}: {:#x?}", i, result.vec_chunks[i]);
    }
}
