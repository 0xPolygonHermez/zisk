use clap::Args;
use pilout::pilout_proxy::PilOutProxy;
use std::path::PathBuf;
use colored::Colorize;

#[derive(Args)]
pub struct PiloutInspectCmd {
    /// pilout file path
    #[clap(short, long)]
    pub pilout: PathBuf,
}

impl PiloutInspectCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} {}", format!("{: >12}", "Command").bright_green().bold(), "Pilout inspect subcommand");
        println!("");

        let pilout = PilOutProxy::new(&self.pilout.display().to_string(), false)?;

        pilout.print_pilout_info();

        Ok(())
    }
}
