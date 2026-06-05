use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;

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

#[cfg(test)]
mod tests {
    use super::validate_asm_hints;

    #[test]
    fn hints_without_asm_is_rejected() {
        assert!(validate_asm_hints(false, Some("file:///tmp/h.bin")).is_err());
    }

    #[test]
    fn no_hints_no_asm_is_ok() {
        assert!(validate_asm_hints(false, None).is_ok());
    }

    // The macOS guard makes `--asm` itself fail there, so the asm-enabled
    // assertions only hold off macOS.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn asm_without_hints_is_ok() {
        assert!(validate_asm_hints(true, None).is_ok());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn asm_with_file_hints_is_ok_but_quic_is_rejected() {
        assert!(validate_asm_hints(true, Some("file:///tmp/h.bin")).is_ok());
        assert!(validate_asm_hints(true, Some("quic://host:1234")).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn asm_is_rejected_on_macos() {
        assert!(validate_asm_hints(true, None).is_err());
    }
}
