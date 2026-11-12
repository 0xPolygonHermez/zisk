mod benchmark;
mod evm;
mod generate;
mod seeds;

use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tools lives in solana").to_owned()
}

fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["generate-profile"] => generate::run(false),
        ["generate-profile", "--check"] => generate::run(true),
        ["check-benchmark", log] => benchmark::run(Path::new(log)),
        ["seed-fuzz"] => seeds::run(),
        ["benchmark-evm"] => evm::run(),
        _ => Err("usage: zisk-solana-tools {generate-profile [--check] | check-benchmark LOG | seed-fuzz | benchmark-evm}".into()),
    }
}
