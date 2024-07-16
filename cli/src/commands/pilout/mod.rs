pub mod pilout_inspect;

use clap::{Parser, Subcommand};

use self::pilout_inspect::PiloutInspectCmd;

#[derive(Parser)]
pub struct PiloutCmd {
    #[command(subcommand)]
    pub pilout_commands: PiloutSubcommands,
}

#[derive(Subcommand)]
pub enum PiloutSubcommands {
    Inspect(PiloutInspectCmd),
}
