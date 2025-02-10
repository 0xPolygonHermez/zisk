use anyhow::{Context, Result};
use cargo_zisk::{
    commands::{ZiskBuild, ZiskRun, ZiskSdk},
    ZISK_VERSION_MESSAGE,
};
use clap::Parser;

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
}

fn main() -> Result<()> {
    // Parse command-line arguments and handle errors if they occur.
    let cargo_args = Cargo::parse();

    match cargo_args {
        Cargo::Sdk(args) => {
            args.command.run().context("Error executing SDK command")?;
        }
        Cargo::Run(args) => {
            args.run().context("Error executing Run command")?;
        }
        Cargo::Build(args) => {
            args.run().context("Error executing Build command")?;
        }
    }

    Ok(())
}
