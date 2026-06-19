//! Command-line surface for the two ZisK binaries: `cargo-zisk` and `cargo-zisk-dev`.

mod dev;
mod shared;
mod user;

pub(crate) use dev::*;
pub(crate) use shared::*;
pub(crate) use user::*;

use anyhow::Result;
use clap::Parser;

/// Parses developer CLI arguments and dispatches to the selected command.
pub fn run_cli_dev() -> Result<()> {
    ZiskCliDevCmd::parse().run()
}

/// Parses the user-facing CLI arguments and dispatches to the selected command.
pub fn run_cli() -> Result<()> {
    ZiskCliCmd::parse().run()
}

#[cfg(test)]
mod tests {
    use super::{ZiskCliCmd, ZiskCliDevCmd};
    use clap::{CommandFactory, Parser};

    /// `debug_assert` walks the whole clap model and panics on a malformed
    /// configuration (duplicate args, bad defaults, conflicting IDs). This is a
    /// cheap guard that catches command-definition regressions for both binaries.
    #[test]
    fn user_cli_model_is_valid() {
        ZiskCliCmd::command().debug_assert();
    }

    #[test]
    fn dev_cli_model_is_valid() {
        ZiskCliDevCmd::command().debug_assert();
    }

    #[test]
    fn user_cli_parses_shared_and_embedded_commands() {
        // `build` comes from the flattened SharedCmd.
        assert!(ZiskCliCmd::try_parse_from(["cargo-zisk", "build", "--release"]).is_ok());
        // The embedded prover commands are exposed directly at the top level.
        assert!(ZiskCliCmd::try_parse_from(["cargo-zisk", "prove", "--elf", "g.elf"]).is_ok());
        assert!(ZiskCliCmd::try_parse_from(["cargo-zisk", "setup", "--elf", "g.elf"]).is_ok());
    }

    #[test]
    fn user_cli_rejects_conflicting_proof_flags() {
        // --minimal and --plonk are mutually exclusive.
        assert!(
            ZiskCliCmd::try_parse_from(["cargo-zisk", "prove", "--minimal", "--plonk"]).is_err()
        );
    }

    #[test]
    fn user_cli_rejects_unknown_command() {
        assert!(ZiskCliCmd::try_parse_from(["cargo-zisk", "not-a-command"]).is_err());
    }

    #[test]
    fn remote_coordinator_has_default_and_env() {
        // Parses without --coordinator (relies on the default value).
        assert!(ZiskCliCmd::try_parse_from(["cargo-zisk", "remote", "upload", "--elf", "g.elf"])
            .is_ok());
    }

    #[test]
    fn dev_cli_standalone_conflicts_with_proving_key() {
        assert!(
            ZiskCliDevCmd::try_parse_from(["cargo-zisk-dev", "execute", "--standalone"]).is_ok()
        );
        assert!(ZiskCliDevCmd::try_parse_from([
            "cargo-zisk-dev",
            "execute",
            "--standalone",
            "--proving-key",
            "k",
        ])
        .is_err());
    }
}
