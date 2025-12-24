use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use colored::Colorize;
use proofman_common::initialize_logger;

use crate::{commands::get_proving_key, ux::print_banner};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskRomVkey {
    /// ELF file path
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// VKey file
    #[clap(short = 'o', long)]
    pub vkey_file: Option<PathBuf>,

    #[clap(short = 'v', long, default_value_t = false)]
    pub verbose: bool,
}

impl ZiskRomVkey {
    pub fn run(&self) -> Result<()> {
        initialize_logger(proofman_common::VerboseMode::Info, None);

        tracing::info!(
            "{}",
            format!("{} Rom VKey", format!("{: >12}", "Command").bright_green().bold())
        );
        tracing::info!("");

        print_banner();

        let proving_key = get_proving_key(self.proving_key.as_ref());

        rom_setup::rom_vkey(&self.elf, &self.vkey_file, &proving_key)?;

        Ok(())
    }
}
