use crate::ZISK_VERSION_MESSAGE;

use std::fs;
use colored::Colorize;
use anyhow::{Context, Result};
use log::info;
use proofman_common::initialize_logger;
use zisk::common::{get_home_zisk_path, print_banner};

/// Deletes the default zisk setup folder
#[derive(clap::Args)]
#[command(about = "Remove the cache directory", long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
pub struct ZiskClean;

impl ZiskClean {
    pub fn run(&self) -> Result<()> {
        initialize_logger(proofman_common::VerboseMode::Info);

        print_banner();
        println!("{} Clean", format!("{: >12}", "Command").bright_green().bold());

        println!();

        let home_zisk_path = get_home_zisk_path();
        let cache_zisk_path = home_zisk_path.join("cache");

        if home_zisk_path.exists() {
            info!("Removing default zisk path at: {}", cache_zisk_path.display());

            fs::remove_dir_all(&cache_zisk_path).with_context(|| {
                format!("Failed to remove directory {}", cache_zisk_path.display())
            })?;

            info!("{} Successfully removed {}", "[OK]".green().bold(), cache_zisk_path.display());
        } else {
            info!(
                "{} No zisk setup directory found at {}",
                "[WARN]".yellow(),
                cache_zisk_path.display()
            );
        }

        Ok(())
    }
}
