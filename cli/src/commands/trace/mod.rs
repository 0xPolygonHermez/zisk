pub mod trace_setup;

use clap::{Parser, Subcommand};
use trace_setup::TraceSetupCmd;

#[derive(Parser)]
pub struct TraceCmd {
    #[command(subcommand)]
    pub trace_commands: TraceSubcommands,
}

#[derive(Subcommand)]
pub enum TraceSubcommands {
    Setup(TraceSetupCmd),
}
