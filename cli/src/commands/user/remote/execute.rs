use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{GuestProgram, RemoteClient, ZiskHints, ZiskStdin};

use crate::common::{reject_quic_hints, resolve_elf};
use crate::ux::print_job_banner;

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Execute a guest program on the remote service
///
/// The program must already be registered and set up (run `remote setup` first).
pub(crate) struct ZiskRemoteExecute {
    /// Path to the guest ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,

    /// Input for the guest. Accepts a file path, `file://path`, or inline data
    /// `inline://[[1,2],[3]]` (a JSON array of u64 arrays, one frame per inner array)
    #[arg(short = 'i', long)]
    inputs: Option<String>,

    /// Precompiles hints URI for the guest (sent inline to the coordinator).
    ///
    /// `file://path` or a plain path is read and sent inline. `quic://` is not
    /// supported from the CLI.
    #[arg(long, conflicts_with = "inputs")]
    hints: Option<String>,

    /// Execute timeout in seconds (0 = no timeout)
    #[arg(long, default_value_t = 0)]
    timeout: u64,
}

impl ZiskRemoteExecute {
    pub(crate) async fn run(&mut self, client: &RemoteClient) -> Result<()> {
        let elf = resolve_elf(self.elf.take())?;
        reject_quic_hints(self.hints.as_deref())?;

        print_job_banner("Remote Execute", &elf, self.inputs.as_deref(), self.hints.as_deref());

        let program = GuestProgram::from_uri(elf.to_str().unwrap())?;
        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;
        let hints = self.hints.as_ref().map(ZiskHints::from_uri).transpose()?;

        let mut request = client.execute(&program, stdin);
        if let Some(hints) = hints {
            request = request.hints(hints);
        }
        if self.timeout != 0 {
            request = request.timeout(Duration::from_secs(self.timeout));
        }
        let result = request.run()?.await?;

        info!("{}", "--- EXECUTE SUMMARY -----------".bright_green().bold());
        info!(
            "Execution completed in {}ms, steps: {}",
            result.get_execution_time(),
            result.get_execution_steps()
        );

        Ok(())
    }
}
