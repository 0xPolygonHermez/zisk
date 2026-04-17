use anyhow::{anyhow, Context, Result};
use cargo_zisk::commands::{
    ZiskBuild, ZiskBuildToolchain, ZiskCheckSetup, ZiskClean, ZiskConvertInput, ZiskExecute,
    ZiskInstallToolchain, ZiskNew, ZiskProgramSetup, ZiskProve, ZiskRun, ZiskStats, ZiskUtils,
    ZiskVerify, ZiskVerifyConstraints, ZiskWrap,
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
    BuildToolchain(ZiskBuildToolchain),
    ConvertInput(ZiskConvertInput),
    CheckSetup(ZiskCheckSetup),
    Clean(ZiskClean),
    InstallToolchain(ZiskInstallToolchain),
    Execute(ZiskExecute),
    New(ZiskNew),
    WrapProof(ZiskWrap),
    Prove(ZiskProve),
    ProgramSetup(ZiskProgramSetup),
    Run(ZiskRun),
    Stats(ZiskStats),
    Utils(ZiskUtils),
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
        Cargo::BuildToolchain(cmd) => {
            cmd.run().context("Error executing BuildToolchain command")?;
        }
        Cargo::ConvertInput(cmd) => {
            cmd.run().context("Error executing ConvertInput command")?;
        }
        Cargo::CheckSetup(cmd) => {
            cmd.run().context("Error executing CheckSetup command")?;
        }
        Cargo::Clean(cmd) => {
            cmd.run().context("Error executing Clean command")?;
        }
        Cargo::InstallToolchain(cmd) => {
            cmd.run().context("Error executing InstallToolchain command")?;
        }
        Cargo::New(cmd) => {
            cmd.run().context("Error executing New command")?;
        }
        Cargo::Prove(mut cmd) => {
            cmd.run().context("Error executing Prove command")?;
        }
        Cargo::WrapProof(cmd) => {
            cmd.run().context("Error executing WrapProof command")?;
        }
        Cargo::ProgramSetup(mut cmd) => {
            cmd.run().context("Error executing RomSetup command")?;
        }
        Cargo::Run(cmd) => {
            cmd.run().context("Error executing Run command")?;
        }
        Cargo::Stats(mut cmd) => {
            cmd.run().context("Error executing Stats command")?;
        }
        Cargo::Utils(mut cmd) => {
            cmd.run().context("Error executing Utils command")?;
        }
        Cargo::Execute(mut cmd) => {
            cmd.run().context("Error executing Execute command")?;
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
