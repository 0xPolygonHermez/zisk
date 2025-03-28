use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use colored::Colorize;
use proofman_common::initialize_logger;

use crate::ux::print_banner;

use super::get_default_proving_key;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskRomSetup {
    /// ELF file path
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Output dir path
    #[clap(short = 'o', long)]
    pub output_dir: Option<PathBuf>,

    #[clap(short = 'v', long, default_value_t = false)]
    pub verbose: bool,
}

impl ZiskRomSetup {
    pub fn run(&self) -> Result<()> {
        println!("{} Rom Setup", format!("{: >12}", "Command").bright_green().bold());

        initialize_logger(proofman_common::VerboseMode::Info);

        print_banner();

        let proving_key = self.get_proving_key();

        rom_setup::rom_full_setup(&self.elf, &proving_key, &self.output_dir, self.verbose)
    }

    /// Gets the proving key file location.
    /// Uses the default one if not specified by user.
    fn get_proving_key(&self) -> PathBuf {
        if self.proving_key.is_none() {
            get_default_proving_key()
        } else {
            self.proving_key.clone().unwrap()
        }
    }
}
