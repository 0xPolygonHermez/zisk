mod build;
mod command;
mod utils;

use build::build_program_internal;
// pub use build::{execute_build_program, generate_elf_paths};

use clap::Parser;

pub const RUSTUP_TOOLCHAIN_NAME: &str = "zisk";

pub const ZISK_VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("VERGEN_GIT_SHA"),
    " ",
    env!("VERGEN_BUILD_TIMESTAMP"),
    ")"
);

pub const ZISK_TARGET: &str = "riscv64ima-zisk-zkvm-elf";

pub const HELPER_TARGET_SUBDIR: &str = "elf";

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

    #[clap(short = 'z', long, value_name = "ZISK_PATH")]
    zisk_path: Option<String>,

    #[clap(long, value_name = "HINTS")]
    pub hints: Option<bool>,
}

pub fn build_program(path: &str) {
    build_program_internal(path, None)
}

pub fn build_program_with_args(path: &str, args: BuildArgs) {
    build_program_internal(path, Some(args))
}
