//! Generates the guest's hints binary as a side effect of building its ELF.
//!
//! Hints are produced by running the guest *natively* (the crate's
//! `.cargo/config.toml` enables `--cfg zisk_hints` for the host target, which
//! makes the accelerators emit hints to `tmp/hints.bin`). This script wires
//! that native run into the normal build so `tmp/hints.bin` stays in sync with
//! the guest source — the `diagnostic_hints_smoke` test then just loads it.
//!
//! ### Why this doesn't recurse
//! The ELF is compiled for the zkVM target (`riscv64ima-zisk-zkvm-elf`); the
//! generator is a host build. We only spawn the generator from the zkVM build —
//! the host build (including the `cargo run` we spawn) is a no-op here. So the
//! nested build never re-enters this branch.

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// The zkVM target triple guests are cross-compiled to (see `ziskbuild::ZISK_TARGET`).
const ZISK_TARGET: &str = "riscv64ima-zisk-zkvm-elf";

fn main() {
    let target = env::var("TARGET").unwrap_or_default();

    // Only the zkVM (ELF) build drives hints generation. Any host build —
    // including the `cargo run` spawned below — falls through and does nothing,
    // which is also what prevents infinite recursion.
    if target != ZISK_TARGET {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Regenerate only when the guest itself changes.
    println!("cargo:rerun-if-changed={}", manifest_dir.join("src").display());
    println!("cargo:rerun-if-changed={}", manifest_dir.join("Cargo.toml").display());
    println!("cargo:rerun-if-changed={}", manifest_dir.join(".cargo/config.toml").display());

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let hints_out = manifest_dir.join("tmp/hints.bin");
    std::fs::create_dir_all(manifest_dir.join("tmp"))
        .expect("failed to create diagnostic_hints/tmp");

    let gen_target = manifest_dir.join("tmp/native-gen-target");
    let status = Command::new(&cargo)
        .current_dir(&manifest_dir)
        .args(["run", "--release"])
        .env("CARGO_TARGET_DIR", &gen_target)
        .env("ZISK_HINTS_OUTPUT", &hints_out)
        // Drop inherited flags so the crate's `.cargo/config.toml` applies the
        // `--cfg zisk_hints` rustflags to the native build (env would override it).
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!("cargo:warning=diagnostic_hints: hints generation exited with {s}; tmp/hints.bin may be stale");
        }
        Err(e) => {
            println!("cargo:warning=diagnostic_hints: failed to run hints generator ({e}); tmp/hints.bin may be stale");
        }
    }
}
