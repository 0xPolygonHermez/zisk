use anyhow::{anyhow, Context, Result};
use cargo_zisk::{
    commands::{
        ZiskBuild, ZiskCheckSetup, ZiskClean, ZiskExecute, ZiskProve, ZiskProveClient,
        ZiskRomSetup, ZiskRun, ZiskSdk, ZiskServer, ZiskStats, ZiskVerify, ZiskVerifyConstraints,
    },
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
    Build(ZiskBuild),
    CheckSetup(ZiskCheckSetup),
    Clean(ZiskClean),
    Execute(ZiskExecute),
    ProveClient(ZiskProveClient),
    Prove(ZiskProve),
    RomSetup(ZiskRomSetup),
    Run(ZiskRun),
    Sdk(ZiskSdk),
    Server(ZiskServer),
    Stats(ZiskStats),
    Verify(ZiskVerify),
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
        Cargo::ProveClient(cmd) => {
            cmd.run().context("Error executing ProveClient command")?;
        }
        Cargo::Prove(mut cmd) => {
            cmd.run().context("Error executing Prove command")?;
        }
        Cargo::RomSetup(cmd) => {
            cmd.run().context("Error executing RomSetup command")?;
        }
        Cargo::Run(cmd) => {
            cmd.run().context("Error executing Run command")?;
        }
        Cargo::Stats(mut cmd) => {
            cmd.run().context("Error executing SDK command")?;
        }
        Cargo::Execute(mut cmd) => {
            cmd.run().context("Error executing Execute command")?;
        }
        Cargo::Sdk(cmd) => {
            cmd.command.run().context("Error executing SDK command")?;
        }
        Cargo::Server(mut cmd) => {
            cmd.run().context("Error executing Server command")?;
        }
        Cargo::Verify(cmd) => {
            cmd.run().map_err(|e| anyhow!("Error executing Verify command: {}", e))?;
        }
        Cargo::VerifyConstraints(mut cmd) => {
            cmd.run().context("Error executing VerifyConstraints command")?;
        }
    }

    Ok(())
}
