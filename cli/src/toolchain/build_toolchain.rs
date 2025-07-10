use crate::{get_target, CommandExecutor, RUSTUP_TOOLCHAIN_NAME};
use anyhow::{Context, Result};
use clap::Parser;
use std::{path::PathBuf, process::Command};

#[derive(Parser)]
#[command(name = "build-toolchain", about = "Build the cargo-zisk toolchain.")]
pub struct BuildToolchainCmd {}

impl BuildToolchainCmd {
    pub fn run(&self) -> Result<()> {
        println!("Building toolchain...");
        // Get environment variables.
        let build_dir = std::env::var("ZISK_BUILD_DIR");
        let rust_dir = match build_dir {
            Ok(build_dir) => {
                println!("Detected ZISK_BUILD_DIR, skipping cloning rust.");
                PathBuf::from(build_dir).join("rust")
            }
            Err(_) => {
                let temp_dir = std::env::temp_dir();
                let dir = temp_dir.join("zisk-rust");
                if dir.exists() {
                    std::fs::remove_dir_all(&dir)?;
                }

                println!("No ZISK_BUILD_DIR detected, cloning rust.");
                let repo_url = "https://{}@github.com/0xPolygonHermez/rust";

                Command::new("git")
                    .args([
                        "clone",
                        repo_url,
                        "--depth=1",
                        "--single-branch",
                        "--branch=zisk",
                        "zisk-rust",
                    ])
                    .current_dir(&temp_dir)
                    .run()?;
                Command::new("git").args(["reset", "--hard"]).current_dir(&dir).run()?;
                Command::new("git")
                    .args(["submodule", "update", "--init", "--recursive", "--progress"])
                    .current_dir(&dir)
                    .run()?;
                dir
            }
        };
        // Install our config.toml.
        let config_toml = include_str!("config.toml");
        let config_file = rust_dir.join("config.toml");
        std::fs::write(&config_file, config_toml)
            .with_context(|| format!("while writing configuration to {config_file:?}"))?;

        // Work around target sanity check added in
        // rust-lang/rust@09c076810cb7649e5817f316215010d49e78e8d7.
        let temp_dir = std::env::temp_dir().join("rustc-targets");
        if !temp_dir.exists() {
            std::fs::create_dir_all(&temp_dir)?;
        }

        std::fs::File::create(temp_dir.join("riscv64ima-zisk-zkvm-elf.json"))?;

        // Build the toolchain.
        Command::new("python3")
            .env("RUST_TARGET_PATH", &temp_dir)
            .env("CARGO_TARGET_RISCV64IMA_ZISK_ZKVM_ELF_RUSTFLAGS", "-Cpasses=lower-atomic")
            .args(["x.py", "build", "--stage", "2", "compiler/rustc", "library"])
            .current_dir(&rust_dir)
            .run()?;

        // Remove the existing toolchain from rustup, if it exists.
        match Command::new("rustup").args(["toolchain", "remove", RUSTUP_TOOLCHAIN_NAME]).run() {
            Ok(_) => println!("Successfully removed existing toolchain."),
            Err(_) => println!("No existing toolchain to remove."),
        }

        // Find the toolchain directory.
        let mut toolchain_dir = None;
        for wentry in std::fs::read_dir(rust_dir.join("build"))? {
            let entry = wentry?;
            let toolchain_dir_candidate = entry.path().join("stage2");
            if toolchain_dir_candidate.is_dir() {
                toolchain_dir = Some(toolchain_dir_candidate);
                break;
            }
        }
        let toolchain_dir = toolchain_dir.unwrap();
        println!(
            "Found built toolchain directory at {}.",
            toolchain_dir.as_path().to_str().unwrap()
        );

        // Link the toolchain to rustup.
        Command::new("rustup")
            .args(["toolchain", "link", RUSTUP_TOOLCHAIN_NAME])
            .arg(&toolchain_dir)
            .run()?;
        println!("Successfully linked the toolchain to rustup.");

        // Compressing toolchain directory to tar.gz.
        let target = get_target();
        let tar_gz_path = format!("rust-toolchain-{target}.tar.gz");
        Command::new("tar")
            .args([
                "--exclude",
                "lib/rustlib/src",
                "--exclude",
                "lib/rustlib/rustc-src",
                "-hczvf",
                &tar_gz_path,
                "-C",
                toolchain_dir.to_str().unwrap(),
                ".",
            ])
            .run()?;
        println!("Successfully compressed the toolchain to {tar_gz_path}.");

        Ok(())
    }
}
