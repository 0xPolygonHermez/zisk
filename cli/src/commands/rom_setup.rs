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

    /// Assembly file path, by default it will be the same as the ELF file with a .asm extension
    #[clap(short = 's', long)]
    pub asm: Option<PathBuf>,
}

impl ZiskRomSetup {
    pub fn run(&self) -> Result<()> {
        println!("{} Rom Setup", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(proofman_common::VerboseMode::Info);

        print_banner();

        rom_setup::assembly_setup(&self.elf, self.asm.as_ref())?;

        rom_setup::rom_merkle_setup(&self.elf, &self.get_proving_key(), true)?;

        Ok(())
    }

    /// Gets the proving key file location.
    /// Uses the default one if not specified by user.
    pub fn get_proving_key(&self) -> PathBuf {
        if self.proving_key.is_none() {
            get_default_proving_key()
        } else {
            self.proving_key.clone().unwrap()
        }
    }
}
