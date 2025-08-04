use std::fs;

use clap::Parser;
use colored::Colorize;

use anyhow::{Context, Result};
use proofman_common::initialize_logger;

use crate::{commands::get_home_zisk_path, ux::print_banner};

/// Deletes the default zisk setup folder
#[derive(Parser, Debug)]
#[command(version, about = "Remove the cache directory", long_about = None)]
pub struct ZiskClean;

impl ZiskClean {
    pub fn run(&self) -> Result<()> {
        initialize_logger(proofman_common::VerboseMode::Info, None);

        print_banner();
        tracing::info!(
            "{}",
            format!("{} Clean", format!("{: >12}", "Command").bright_green().bold())
        );
        tracing::info!("");

        let home_zisk_path = get_home_zisk_path();
        let cache_zisk_path = home_zisk_path.join("cache");

        if home_zisk_path.exists() {
            tracing::info!("Removing default zisk path at: {}", cache_zisk_path.display());

            fs::remove_dir_all(&cache_zisk_path).with_context(|| {
                format!("Failed to remove directory {}", cache_zisk_path.display())
            })?;

            tracing::info!(
                "{} Successfully removed {}",
                "[OK]".green().bold(),
                cache_zisk_path.display()
            );
        } else {
            tracing::info!(
                "{} No zisk setup directory found at {}",
                "[WARN]".yellow(),
                cache_zisk_path.display()
            );
        }

        Ok(())
    }
}
