use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use colored::Colorize;
use proofman_common::initialize_logger;

use crate::{
    commands::{cli_fail_if_macos, get_proving_key, get_zisk_path},
    ux::print_banner,
};

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
        cli_fail_if_macos()?;

        initialize_logger(proofman_common::VerboseMode::Info, None);

        tracing::info!(
            "{}",
            format!("{} Rom Setup", format!("{: >12}", "Command").bright_green().bold())
        );
        tracing::info!("");

        print_banner();

        let proving_key = get_proving_key(self.proving_key.as_ref());
        let zisk_path = get_zisk_path(self.zisk_path.as_ref());

        rom_setup::rom_full_setup(
            &self.elf,
            &proving_key,
            &zisk_path,
            &self.output_dir,
            self.verbose,
        )
    }
}
