use anyhow::{anyhow, Context, Result};
use cargo_zisk::commands::{
    ZiskBuild, ZiskCheckSetup, ZiskClean, ZiskExecute, ZiskProve, ZiskProveSnark, ZiskRomSetup,
    ZiskRun, ZiskSdk, ZiskStats, ZiskVerify, ZiskVerifyConstraints, ZiskVerifySnark,
};
use clap::Parser;
use zisk_build::ZISK_VERSION_MESSAGE;

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
    Build(ZiskBuild),
    CheckSetup(ZiskCheckSetup),
    Clean(ZiskClean),
    Execute(ZiskExecute),
    Prove(ZiskProve),
    ProveSnark(ZiskProveSnark),
    RomSetup(ZiskRomSetup),
    Run(ZiskRun),
    Sdk(ZiskSdk),
    Stats(ZiskStats),
    Verify(ZiskVerify),
    VerifySnark(ZiskVerifySnark),
    VerifyConstraints(ZiskVerifyConstraints),
}

fn main() -> Result<()> {
    // Parse command-line arguments and handle errors if they occur.
    let cargo_args = Cargo::parse();

    match cargo_args {
        Cargo::Build(cmd) => {
            cmd.run().context("Error executing Build command")?;
        }
        Cargo::CheckSetup(cmd) => {
            cmd.run().context("Error executing CheckSetup command")?;
        }
        Cargo::Clean(cmd) => {
            cmd.run().context("Error executing Clean command")?;
        }
        Cargo::Prove(mut cmd) => {
            cmd.run().context("Error executing Prove command")?;
        }
        Cargo::ProveSnark(cmd) => {
            cmd.run().context("Error executing ProveSnark command")?;
        }
        Cargo::RomSetup(cmd) => {
            cmd.run().context("Error executing RomSetup command")?;
        }
        Cargo::Run(cmd) => {
            cmd.run().context("Error executing Run command")?;
        }
        Cargo::Stats(mut cmd) => {
            cmd.run().context("Error executing Stats command")?;
        }
        Cargo::Execute(mut cmd) => {
            cmd.run().context("Error executing Execute command")?;
        }
        Cargo::Sdk(cmd) => {
            cmd.command.run().context("Error executing SDK command")?;
        }
        Cargo::Verify(cmd) => {
            cmd.run().map_err(|e| anyhow!("Error executing Verify command: {}", e))?;
        }
        Cargo::VerifySnark(cmd) => {
            cmd.run().context("Error executing VerifySnark command")?;
        }
        Cargo::VerifyConstraints(mut cmd) => {
            cmd.run().context("Error executing VerifyConstraints command")?;
        }
    }

    Ok(())
}
