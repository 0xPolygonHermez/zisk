use clap::Args;
use pilout::pilout_proxy::PilOutProxy;
use proofman_common::VerboseMode;
use std::path::PathBuf;
use colored::Colorize;

#[derive(Args)]
pub struct PiloutInspectCmd {
    /// pilout file path
    #[clap(short, long)]
    pub pilout: PathBuf,

    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl PiloutInspectCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Pilout inspect subcommand", format!("{: >12}", "Command").bright_green().bold());
        println!();

        let filter_level: VerboseMode = self.verbose.into();
        env_logger::builder()
            .format_timestamp(None)
            .format_level(true)
            .format_target(false)
            .filter_level(filter_level.into())
            .init();

        let pilout = PilOutProxy::new(&self.pilout.display().to_string())?;

        pilout.print_pilout_info();

        Ok(())
    }
}
