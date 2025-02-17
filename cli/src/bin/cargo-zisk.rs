use anyhow::{anyhow, Context, Result};
use cargo_zisk::{
    commands::{ZiskBuild, ZiskProve, ZiskRun, ZiskSdk, ZiskVerifyConstraints},
    ZISK_VERSION_MESSAGE,
};
use clap::Parser;
use proofman_cli::commands::verify_stark::VerifyStark;

// Main enum defining cargo subcommands.
#[derive(Parser)]
#[command(
    name = "cargo-zisk",
    bin_name = "cargo-zisk",
    version = ZISK_VERSION_MESSAGE,
    about = "CLI tool for Zisk",
    long_about = "Cargo Zisk is a command-line tool to manage Zisk projects."
)]
pub enum Cargo {
    Sdk(ZiskSdk),
    Run(ZiskRun),
    Build(ZiskBuild),
    Prove(ZiskProve),
    VerifyConstraints(ZiskVerifyConstraints),
    VerifyStark(VerifyStark),
}

fn main() -> Result<()> {
    // Parse command-line arguments and handle errors if they occur.
    let cargo_args = Cargo::parse();

    match cargo_args {
        Cargo::Sdk(cmd) => {
            cmd.command.run().context("Error executing SDK command")?;
        }
        Cargo::Run(cmd) => {
            cmd.run().context("Error executing Run command")?;
        }
        Cargo::Build(cmd) => {
            cmd.run().context("Error executing Build command")?;
        }
        Cargo::Prove(args) => {
            args.run().context("Error executing Prove command")?;
        }
        Cargo::VerifyConstraints(cmd) => {
            cmd.run().context("Error executing VerifyConstraints command")?;
        }
        Cargo::VerifyStark(cmd) => {
            cmd.run().map_err(|e| anyhow!("Error executing VerifyStark command: {}", e))?;
        }
    }

    Ok(())
}
