use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use colored::Colorize;
use proofman_common::initialize_logger;

use crate::{commands::get_proving_key, ux::print_banner};
use rom_setup::gen_assembly;
use rom_setup::rom_merkle_setup;

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

    /// Setup folder path
    #[clap(short = 'z', long)]
    pub zisk_path: Option<PathBuf>,

    /// Output dir path
    #[clap(short = 'o', long)]
    pub output_dir: Option<PathBuf>,

    #[clap(short = 'v', long, default_value_t = false)]
    pub verbose: bool,
}

impl ZiskRomSetup {
    pub fn run(&self) -> Result<()> {
        initialize_logger(proofman_common::VerboseMode::Info, None);

        tracing::info!(
            "{}",
            format!("{} Rom Setup", format!("{: >12}", "Command").bright_green().bold())
        );
        tracing::info!("");

        print_banner();

        let proving_key = get_proving_key(self.proving_key.as_ref());

        tracing::info!("Computing setup for ROM {}", self.elf.display());

        tracing::info!("Computing merkle root");
        rom_merkle_setup(&self.elf, &self.output_dir, &proving_key, false)?;

        gen_assembly(&self.elf, &self.zisk_path, &self.output_dir, self.verbose)?;

        println!();
        tracing::info!("{}", "ROM setup successfully completed".bright_green().bold());
        Ok(())
    }
}
