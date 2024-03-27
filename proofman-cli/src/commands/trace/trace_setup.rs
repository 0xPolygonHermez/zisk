use clap::Args;
use proofman::command_handlers::trace_setup_handler::trace_setup_handler;
use std::path::PathBuf;
use colored::Colorize;
use pilout::pilout_proxy::PilOutProxy;
#[derive(Args)]
pub struct TraceSetupCmd {
    /// pilout file path
    #[clap(short, long)]
    pub pilout: PathBuf,

    /// destination folder path
    #[clap(short, long, default_value = ".")]
    pub dest: PathBuf,
}

impl TraceSetupCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} {}", format!("{: >12}", "Command").bright_green().bold(), "Trace setup subcommand");
        println!("");

        let pilout = PilOutProxy::new(&self.pilout.display().to_string())?;
        for (subproof_id, _subproof) in pilout.subproofs.iter().enumerate() {
            let output = match trace_setup_handler(&pilout, subproof_id) {
                Ok(output) => output,
                Err(e) => return Err(e),
            };

            // TODO write to file
            println!("{}", output);
        }

        Ok(())
    }
}
