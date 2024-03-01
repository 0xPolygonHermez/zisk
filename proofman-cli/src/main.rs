use clap::{Parser, Subcommand};
mod commands;
use commands::trace::{TraceSubcommands, TraceCmd};
use commands::prove::ProveCmd;
use util::cli::print_banner;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Trace(TraceCmd),
    Prove(ProveCmd),
}

fn main() {
    print_banner(false);

    let cli = Cli::parse();

    match &cli.command {
        Commands::Trace(args) => match &args.trace_commands {
            TraceSubcommands::Setup(args) => {
                args.run().unwrap();
            }
        },
        Commands::Prove(args) => {
            args.run().unwrap();
        }
    };
}
