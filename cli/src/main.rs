use clap::{Parser, Subcommand};
mod commands;
use commands::new::NewCmd;
use commands::trace::{TraceSubcommands, TraceCmd};
use commands::pilout::{PiloutSubcommands, PiloutCmd};
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
    Pilout(PiloutCmd),
    Trace(TraceCmd),
    New(NewCmd),
}

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    print_banner(false);

    let cli = Cli::parse();

    match &cli.command {
        Commands::Pilout(args) => match &args.pilout_commands {
            PiloutSubcommands::Inspect(args) => {
                args.run().unwrap();
            }
        },
        Commands::Trace(args) => match &args.trace_commands {
            TraceSubcommands::Setup(args) => {
                args.run().unwrap();
            }
        },
        Commands::New(args) => {
            args.run().unwrap();
        }
    };
}
