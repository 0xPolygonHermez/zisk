use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::{commands::get_proving_key, ux::print_banner};
use colored::Colorize;
use fields::Goldilocks;
use proofman_common::initialize_logger;
use rom_setup::gen_assembly;
use rom_setup::rom_merkle_setup;
use std::fs;
use zisk_common::ElfBinaryOwned;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskRomSetup {
    /// ELF file path
    #[clap(short = 'e', long)]
    pub elf_path: PathBuf,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Setup folder path
    #[clap(short = 'z', long)]
    pub zisk_path: Option<PathBuf>,

    /// Output dir path
    #[clap(short = 'o', long)]
    pub output_dir: Option<PathBuf>,

    /// Enable precompile hints in assembly generation
    #[clap(short = 'n', long, default_value_t = false)]
    pub hints: bool,

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

        tracing::info!("Computing setup for ROM {}", self.elf_path.display());

        tracing::info!("Computing merkle root");
        let elf_bin = fs::read(&self.elf_path).map_err(|e| {
            anyhow::anyhow!("Error reading ELF file {}: {}", self.elf_path.display(), e)
        })?;
        let elf = ElfBinaryOwned::new(
            elf_bin,
            self.elf_path.file_stem().unwrap().to_str().unwrap().to_string(),
            self.hints,
        );
        rom_merkle_setup::<Goldilocks>(&elf, &self.output_dir, &proving_key)?;

        gen_assembly(&self.elf_path, &self.zisk_path, &self.output_dir, self.verbose, self.hints)?;

        println!();
        tracing::info!("{}", "ROM setup successfully completed".bright_green().bold());
        Ok(())
    }
}
