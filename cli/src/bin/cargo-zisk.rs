use anyhow::{anyhow, Context, Result};
use cargo_zisk::commands::{
    ZiskBuild, ZiskCheckSetup, ZiskClean, ZiskExecute, ZiskExportSolidityCalldata, ZiskNew,
    ZiskProgramSetup, ZiskProofmanSetup, ZiskProve, ZiskProveRecurserAggregator, ZiskRun,
    ZiskSetupRecurserAggregator, ZiskStats, ZiskToolchain, ZiskUtils, ZiskVerify,
    ZiskVerifyConstraints, ZiskWrap,
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
    #[command(hide = true)]
    CheckSetup(ZiskCheckSetup),
    Clean(ZiskClean),
    Execute(ZiskExecute),
    ExportSolidityCalldata(ZiskExportSolidityCalldata),
    New(ZiskNew),
    WrapProof(ZiskWrap),
    Prove(ZiskProve),
    ProveRecurserAggregator(ZiskProveRecurserAggregator),
    ProgramSetup(ZiskProgramSetup),
    ProofmanSetup(ZiskProofmanSetup),
    Run(ZiskRun),
    SetupRecurserAggregator(ZiskSetupRecurserAggregator),
    #[command(hide = true)]
    Stats(ZiskStats),
    Toolchain(ZiskToolchain),
    Utils(ZiskUtils),
    Verify(ZiskVerify),
    #[command(hide = true)]
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
        Cargo::New(cmd) => {
            cmd.run().context("Error executing New command")?;
        }
        Cargo::Prove(mut cmd) => {
            cmd.run().context("Error executing Prove command")?;
        }
        Cargo::ProveRecurserAggregator(cmd) => {
            cmd.run().context("Error executing ProveRecurserAggregator command")?;
        }
        Cargo::WrapProof(cmd) => {
            cmd.run().context("Error executing WrapProof command")?;
        }
        Cargo::ProgramSetup(mut cmd) => {
            cmd.run().context("Error executing RomSetup command")?;
        }
        Cargo::ProofmanSetup(mut cmd) => {
            cmd.run().context("Error executing ProofmanSetup command")?;
        }
        Cargo::Run(cmd) => {
            cmd.run().context("Error executing Run command")?;
        }
        Cargo::SetupRecurserAggregator(cmd) => {
            cmd.run().context("Error executing SetupRecurserAggregator command")?;
        }
        Cargo::Stats(mut cmd) => {
            cmd.run().context("Error executing Stats command")?;
        }
        Cargo::Toolchain(mut cmd) => {
            cmd.run().context("Error executing Toolchain command")?;
        }
        Cargo::Utils(mut cmd) => {
            cmd.run().context("Error executing Utils command")?;
        }
        Cargo::Execute(mut cmd) => {
            cmd.run().context("Error executing Execute command")?;
        }
        Cargo::ExportSolidityCalldata(cmd) => {
            cmd.run().context("Error executing ExportSolidityCalldata command")?;
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
