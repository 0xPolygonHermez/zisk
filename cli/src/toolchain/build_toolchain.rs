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
        // Get enviroment variables.
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
        let ci = std::env::var("CI").unwrap_or("false".to_string()) == "true";
        let config_toml =
            if ci { include_str!("config-ci.toml") } else { include_str!("config.toml") };
        let config_file = rust_dir.join("config.toml");
        std::fs::write(&config_file, config_toml)
            .with_context(|| format!("while writing configuration to {:?}", config_file))?;
        // Build the toolchain (stage 1).
        Command::new("python3")
            .env("CARGO_TARGET_RISCV64IMA_POLYGON_ZISKOS_ELF_RUSTFLAGS", "")
            .args(["x.py", "build"])
            .current_dir(&rust_dir)
            .run()?;

        // Build the toolchain (stage 2).
        Command::new("python3")
            .env("CARGO_TARGET_RISCV64IMA_POLYGON_ZISKOS_ELF_RUSTFLAGS", "")
            .args(["x.py", "build", "--stage", "2"])
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

        // Copy over the stage2-tools-bin directory to the toolchain bin directory.
        /*
        let tools_bin_dir = toolchain_dir.parent().unwrap().join("stage2-tools-bin");
        let target_bin_dir = toolchain_dir.join("bin");
        for tool in tools_bin_dir.read_dir()? {
            let tool = tool?;
            let tool_name = tool.file_name();
            std::fs::copy(&tool.path(), target_bin_dir.join(tool_name))?;
        }*/

        // Link the toolchain to rustup.
        Command::new("rustup")
            .args(["toolchain", "link", RUSTUP_TOOLCHAIN_NAME])
            .arg(&toolchain_dir)
            .run()?;
        println!("Successfully linked the toolchain to rustup.");

        // Compressing toolchain directory to tar.gz.
        let target = get_target();
        let tar_gz_path = format!("rust-toolchain-{}.tar.gz", target);
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
        println!("Successfully compressed the toolchain to {}.", tar_gz_path);

        Ok(())
    }
}
