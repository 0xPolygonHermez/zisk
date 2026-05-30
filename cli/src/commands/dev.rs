use anyhow::Result;
use clap::Parser;
use zisk_build::ZISK_VERSION_MESSAGE;

use super::SharedCmd;

mod check_setup;
mod clean;
mod execute;
mod export_solidity_calldata;
mod program_setup;
mod proofman_setup;
mod prove;
mod stats;
mod verify_constraints;
mod wrap;

pub(crate) use check_setup::*;
pub(crate) use clean::*;
pub(crate) use execute::*;
pub(crate) use export_solidity_calldata::*;
pub(crate) use program_setup::*;
pub(crate) use proofman_setup::*;
pub(crate) use prove::*;
pub(crate) use stats::*;
pub(crate) use verify_constraints::*;
pub(crate) use wrap::*;

/// Parses developer CLI arguments and dispatches to the selected command.
pub fn run_cli_dev() -> Result<()> {
    ZiskCliDevCmd::parse().run()
}

#[derive(Parser)]
#[command(
    name = "cargo-zisk-dev",
    bin_name = "cargo-zisk-dev",
    version = ZISK_VERSION_MESSAGE,
    about = "Developer/advanced CLI for ZisK",
    long_about = "Cargo Zisk Dev is the developer-facing CLI for ZisK. \
                  It exposes the full set of commands and flags, including advanced and internal ones \
                  used to develop, debug, and benchmark ZisK itself. \
                  End-user workflows for building and proving guest programs should use cargo-zisk."
)]
pub(crate) enum ZiskCliDevCmd {
    // Commands shared with cargo-zisk
    #[command(flatten)]
    Shared(SharedCmd),

    // Dev-only commands
    CheckSetup(CheckSetupCmd),
    Clean(CleanCmd),
    Execute(ExecuteCmd),
    ExportSolidityCalldata(ExportSolidityCalldataCmd),
    WrapProof(WrapCmd),
    Prove(ProveCmd),
    ProgramSetup(ProgramSetupCmd),
    ProofmanSetup(ProofmanSetupCmd),
    Stats(StatsCmd),
    VerifyConstraints(VerifyConstraintsCmd),
}

impl ZiskCliDevCmd {
    pub(crate) fn run(self) -> Result<()> {
        match self {
            ZiskCliDevCmd::Shared(cmd) => cmd.run(),
            ZiskCliDevCmd::CheckSetup(cmd) => cmd.run(),
            ZiskCliDevCmd::Clean(cmd) => cmd.run(),
            ZiskCliDevCmd::Prove(mut cmd) => cmd.run(),
            ZiskCliDevCmd::WrapProof(cmd) => cmd.run(),
            ZiskCliDevCmd::ProgramSetup(mut cmd) => cmd.run(),
            ZiskCliDevCmd::ProofmanSetup(mut cmd) => cmd.run(),
            ZiskCliDevCmd::Stats(mut cmd) => cmd.run(),
            ZiskCliDevCmd::Execute(mut cmd) => cmd.run(),
            ZiskCliDevCmd::ExportSolidityCalldata(cmd) => cmd.run(),
            ZiskCliDevCmd::VerifyConstraints(mut cmd) => cmd.run(),
        }
    }
}
