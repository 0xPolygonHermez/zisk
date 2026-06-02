use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{EmbeddedClient, GuestProgram};

use crate::common::reject_quic_hints;

mod execute;
mod prove;
mod wrap;

pub(crate) use execute::ZiskEmbeddedExecute;
pub(crate) use prove::ZiskEmbeddedProve;
pub(crate) use wrap::ZiskEmbeddedWrap;

/// Reject `--asm`/`--hints` combinations the embedded CLI cannot serve.
fn validate_asm_hints(asm: bool, hints: Option<&str>) -> Result<()> {
    // The ASM backend is not supported on macOS.
    if asm && cfg!(target_os = "macos") {
        anyhow::bail!("--asm is not supported on macOS; the ASM backend is Linux-only.");
    }
    // Hints are streamed to the ASM backend only.
    if hints.is_some() && !asm {
        anyhow::bail!("--hints requires the ASM backend; re-run with --asm.");
    }
    reject_quic_hints(hints)
}

/// Run ROM setup before execution/proving, selecting hints or emulator-only mode.
///
/// The embedded SDK exposes a synchronous path (`run_sync`), so no async runtime
/// is needed here.
fn run_embedded_setup(
    client: &EmbeddedClient,
    program: &GuestProgram,
    asm: bool,
    has_hints: bool,
) -> Result<()> {
    let mut setup = client.setup(program);
    if !asm {
        setup = setup.emulator_only();
    } else if has_hints {
        setup = setup.with_hints();
    }
    setup.run_sync()?;
    Ok(())
}

#[derive(clap::Subcommand)]
pub(crate) enum ZiskEmbeddedCmd {
    /// Generate a proof locally
    Prove(ZiskEmbeddedProve),
    /// Execute a guest program locally
    Execute(ZiskEmbeddedExecute),
    /// Wrap a proof locally
    Wrap(ZiskEmbeddedWrap),
}

impl ZiskEmbeddedCmd {
    pub(crate) fn run(&mut self) -> Result<()> {
        match self {
            ZiskEmbeddedCmd::Prove(cmd) => cmd.run(),
            ZiskEmbeddedCmd::Execute(cmd) => cmd.run(),
            ZiskEmbeddedCmd::Wrap(cmd) => cmd.run(),
        }
    }
}

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Run ZisK operations locally using the embedded prover
pub(crate) struct EmbeddedCmd {
    #[command(subcommand)]
    command: ZiskEmbeddedCmd,
}

impl EmbeddedCmd {
    pub(crate) fn run(&mut self) -> Result<()> {
        self.command.run()
    }
}
