mod build;
mod command;
mod utils;

use build::build_program_internal;
// pub use build::{execute_build_program, generate_elf_paths};

use clap::Parser;

pub const RUSTUP_TOOLCHAIN_NAME: &str = "zisk";

pub const ZISK_VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " [",
    env!("ZISK_COMPUTE_MODE"),
    "]",
    " (",
    env!("VERGEN_GIT_SHA"),
    " ",
    env!("VERGEN_BUILD_TIMESTAMP"),
    ")"
);

pub const ZISK_TARGET: &str = "riscv64ima-zisk-zkvm-elf";

/// Rust target triple for the wasm guest machine.  Stock target — no custom toolchain required.
pub const ZISK_WASM_TARGET: &str = "wasm32-wasip1";

pub const HELPER_TARGET_SUBDIR: &str = "elf";

/// The guest machine a program is built for.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum GuestMachine {
    /// Bare-metal RISC-V (the default Zisk guest), built with the `zisk` toolchain.
    #[default]
    Riscv,
    /// wasm32 with WASI, built with the stock `wasm32-wasip1` target.
    Wasm,
}

/// Arguments for building a ZisK program.
#[derive(Default, Clone, Parser, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
pub struct BuildArgs {
    #[clap(short = 'F', long)]
    features: Option<String>,

    #[clap(long)]
    all_features: bool,

    #[clap(long)]
    release: bool,

    #[clap(long)]
    no_default_features: bool,

    #[clap(long, value_name = "OUTPUT_DIRECTORY")]
    output_directory: Option<String>,

    #[clap(long, value_name = "ELF_NAME")]
    elf_name: Option<String>,

    #[clap(long, value_name = "ASM")]
    pub asm: Option<bool>,

    #[clap(long, value_name = "HINTS")]
    pub hints: Option<bool>,

    #[clap(long = "package", value_name = "PACKAGE")]
    pub packages: Vec<String>,

    #[clap(long = "bin", value_name = "BIN")]
    pub binaries: Vec<String>,

    /// Guest machine to build for: `riscv` (default) or `wasm` (wasm32-wasip1).
    #[clap(long, value_enum, default_value_t = GuestMachine::Riscv)]
    pub machine: GuestMachine,
}

pub fn build_program(path: &str) {
    build_program_internal(path, None)
}

pub fn build_program_asm(path: &str) {
    let args = BuildArgs { asm: Some(true), ..Default::default() };
    build_program_internal(path, Some(args))
}

pub fn build_program_with_args(path: &str, args: BuildArgs) {
    build_program_internal(path, Some(args))
}
