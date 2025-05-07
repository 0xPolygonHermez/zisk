use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use colored::Colorize;
use proofman_common::initialize_logger;

use zisk::common::print_banner;
use zisk::common::{get_default_proving_key, get_default_zisk_path};

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
        println!("{} Rom Setup", format!("{: >12}", "Command").bright_green().bold());

        initialize_logger(proofman_common::VerboseMode::Info);

        print_banner();

        let proving_key = self.get_proving_key();
        let zisk_path = self.get_zisk_path();

        rom_setup::rom_full_setup(
            &self.elf,
            &proving_key,
            &zisk_path,
            &self.output_dir,
            self.verbose,
        )
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

    /// Gets the proving key file location.
    /// Uses the default one if not specified by user.
    fn get_zisk_path(&self) -> PathBuf {
        if self.zisk_path.is_none() {
            get_default_zisk_path()
        } else {
            self.zisk_path.clone().unwrap()
        }
    }
}
