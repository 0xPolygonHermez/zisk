use std::future::IntoFuture;
use std::time::Duration;

use anyhow::Result;
use tracing::{error, warn};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{JobHandle, ProverClient};

mod aggregate;
mod execute;
mod prove;
mod setup;
mod upload;
mod wrap;

pub(crate) use aggregate::ZiskRemoteAggregate;
pub(crate) use execute::ZiskRemoteExecute;
pub(crate) use prove::ZiskRemoteProve;
pub(crate) use setup::ZiskRemoteSetup;
pub(crate) use upload::ZiskRemoteUpload;
pub(crate) use wrap::ZiskRemoteWrap;

/// Wait for a remote job, and cancel it on Ctrl-C, or once `timeout`, if any, has
/// passed: the job would otherwise go on running on the cluster after the command
/// has given up on it.
pub(crate) async fn await_job<T>(job: JobHandle<T>, timeout: Option<Duration>) -> Result<T>
where
    JobHandle<T>: IntoFuture<Output = zisk_sdk::Result<T>>,
{
    let canceller = job.canceller();
    let expired = async {
        match timeout {
            Some(timeout) => tokio::time::sleep(timeout).await,
            None => std::future::pending().await,
        }
    };
    let why = tokio::select! {
        result = job.into_future() => return Ok(result?),
        _ = tokio::signal::ctrl_c() => "interrupted".to_string(),
        () = expired => format!("timed out after {}s", timeout.unwrap_or_default().as_secs()),
    };
    if let Some(canceller) = canceller {
        match canceller.cancel().await {
            Ok(true) => warn!("Job {why}; cancelled it"),
            Ok(false) => warn!("Job {why}; it had already ended"),
            Err(e) => error!("Job {why}; cancelling it failed: {e}"),
        }
    }
    anyhow::bail!("job {why}")
}

#[derive(clap::Subcommand)]
pub(crate) enum ZiskRemoteCmd {
    /// Upload a guest program to the remote service
    Upload(ZiskRemoteUpload),
    /// Generate the proving setup on the remote service
    Setup(ZiskRemoteSetup),
    /// Generate a proof on the remote service
    Prove(ZiskRemoteProve),
    /// Execute a guest program on the remote service
    Execute(ZiskRemoteExecute),
    /// Wrap a proof on the remote service
    Wrap(ZiskRemoteWrap),
    /// Fold two proofs into one recurser proof on the remote service
    Aggregate(ZiskRemoteAggregate),
}

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Run ZisK operations against a remote prover service
pub(crate) struct RemoteCmd {
    /// Coordinator gRPC URL
    #[arg(
        long,
        default_value = "http://localhost:7000",
        env = "ZISK_COORDINATOR_URL",
        global = true
    )]
    coordinator: String,

    /// Connection timeout in seconds
    #[arg(long, default_value_t = 10, global = true)]
    connect_timeout: u64,

    /// Per-request timeout in seconds
    #[arg(long, default_value_t = 3600, global = true)]
    request_timeout: u64,

    #[command(subcommand)]
    command: ZiskRemoteCmd,
}

impl RemoteCmd {
    pub(crate) fn run(&mut self) -> Result<()> {
        // The remote backend talks gRPC to the coordinator, which is async — there
        // is no synchronous path (unlike the embedded client). Drive everything on a
        // multi-threaded runtime: the coordinator client uses `block_in_place`
        // internally, which requires a multi-threaded runtime context.
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        runtime.block_on(async {
            let client = ProverClient::remote(self.coordinator.clone())
                .connect_timeout(Duration::from_secs(self.connect_timeout))
                .request_timeout(Duration::from_secs(self.request_timeout))
                .build()?;

            match &mut self.command {
                ZiskRemoteCmd::Upload(cmd) => cmd.run(&client).await,
                ZiskRemoteCmd::Setup(cmd) => cmd.run(&client).await,
                ZiskRemoteCmd::Prove(cmd) => cmd.run(&client).await,
                ZiskRemoteCmd::Execute(cmd) => cmd.run(&client).await,
                ZiskRemoteCmd::Wrap(cmd) => cmd.run(&client).await,
                ZiskRemoteCmd::Aggregate(cmd) => cmd.run(&client).await,
            }
        })
    }
}
