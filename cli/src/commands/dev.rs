use anyhow::Result;
use clap::Parser;
use zisk_build::ZISK_VERSION_MESSAGE;

use super::{ZiskBuild, ZiskNew, ZiskRun, ZiskToolchain, ZiskUtils, ZiskVerify};

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
pub enum ZiskDevCmd {
    // Shared commands with cargo-zisk
    New(ZiskNew),
    Build(ZiskBuild),
    Run(ZiskRun),
    Verify(ZiskVerify),
    Utils(ZiskUtils),
    Toolchain(ZiskToolchain),

    // Dev-only commands
    CheckSetup(ZiskCheckSetup),
    Clean(ZiskClean),
    Execute(ZiskExecute),
    ExportSolidityCalldata(ZiskExportSolidityCalldata),
    WrapProof(ZiskWrap),
    Prove(ZiskProve),
    ProgramSetup(ZiskProgramSetup),
    ProofmanSetup(ZiskProofmanSetup),
    Stats(ZiskStats),
    VerifyConstraints(ZiskVerifyConstraints),
}

impl ZiskDevCmd {
    pub fn run(self) -> Result<()> {
        match self {
            ZiskDevCmd::Build(cmd) => cmd.run(),
            ZiskDevCmd::CheckSetup(cmd) => cmd.run(),
            ZiskDevCmd::Clean(cmd) => cmd.run(),
            ZiskDevCmd::New(cmd) => cmd.run(),
            ZiskDevCmd::Prove(mut cmd) => cmd.run(),
            ZiskDevCmd::WrapProof(cmd) => cmd.run(),
            ZiskDevCmd::ProgramSetup(mut cmd) => cmd.run(),
            ZiskDevCmd::ProofmanSetup(mut cmd) => cmd.run(),
            ZiskDevCmd::Run(cmd) => cmd.run(),
            ZiskDevCmd::Stats(mut cmd) => cmd.run(),
            ZiskDevCmd::Toolchain(mut cmd) => cmd.run(),
            ZiskDevCmd::Utils(mut cmd) => cmd.run(),
            ZiskDevCmd::Execute(mut cmd) => cmd.run(),
            ZiskDevCmd::ExportSolidityCalldata(cmd) => cmd.run(),
            ZiskDevCmd::Verify(cmd) => cmd.run(),
            ZiskDevCmd::VerifyConstraints(mut cmd) => cmd.run(),
        }
    }
}
