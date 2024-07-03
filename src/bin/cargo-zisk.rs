use anyhow::{Context, Result};
use cargo_zisk::commands::build_toolchain::BuildToolchainCmd;
use cargo_zisk::ZISK_VERSION_MESSAGE;
use clap::{Parser, Subcommand};

// Main enum defining cargo subcommands.
#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
pub enum Cargo {
    Sdk(ZiskSdk),
}

// Structure representing the 'sdk' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, args_conflicts_with_subcommands = true, version = ZISK_VERSION_MESSAGE)]
pub struct ZiskSdk {
    #[clap(subcommand)]
    pub command: Option<ZiskSdkCommands>,
}

// Enum defining the available subcommands for `ZiskSdk`.
#[derive(Subcommand)]
pub enum ZiskSdkCommands {
    BuildToolchain(BuildToolchainCmd),
}

fn main() -> Result<()> {
    // Parse command-line arguments and handle errors if they occur.
    let Cargo::Sdk(args) = Cargo::parse();

    // Check if a command was provided and execute the corresponding command.
    if let Some(command) = args.command {
        match command {
            ZiskSdkCommands::BuildToolchain(cmd) => {
                cmd.run()
                    .context("Error executing BuildToolchain command")?;
            }
        }
    } else {
        println!("No command provided");
    }
    Ok(())
}
