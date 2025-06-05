extern crate libc;

use asm_runner::{AsmRunnerMT, AsmRunnerOptionsBuilder};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Zisk Asm Emulator Runner", long_about = None)]
struct Args {
    /// Path to the assembly runner binary path
    asm_runner_path: PathBuf,

    /// Path to the inputs file
    inputs_path: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let runner_options = AsmRunnerOptionsBuilder::new().with_log_output().with_metrics().build();

    let _ = AsmRunnerMT::run(
        &args.asm_runner_path,
        args.inputs_path.as_deref(),
        1 << 32,
        1 << 15,
        runner_options,
    );

    println!("Done!");
}
