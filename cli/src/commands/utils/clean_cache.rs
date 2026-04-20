use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;
use proofman_common::VerboseMode;
use rom_setup::get_elf_data_hash;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

use crate::{
    common::{detect_current_project_elf, get_home_zisk_path},
    ux::{print_banner, print_banner_command},
};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Clean the zisk cache directory for a specific program or the entire cache
pub struct ZiskCleanCache {
    /// Path of the program ELF file to clean cache for. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long, conflicts_with = "all")]
    pub elf: Option<PathBuf>,

    /// Clean cache for all programs (mutually exclusive with `--elf`)
    #[arg(short = 'a', long, conflicts_with = "elf")]
    pub all: bool,
}

impl ZiskCleanCache {
    pub fn run(&mut self) -> Result<()> {
        setup_logger(VerboseMode::Info);

        print_banner();
        print_banner_command("Clean");

        let home_zisk_path = get_home_zisk_path()?;
        let cache_zisk_path = home_zisk_path.join("cache");

        if cache_zisk_path.exists() {
            if self.all {
                tracing::info!("Removing zisk cache path at: {}", cache_zisk_path.display());

                fs::remove_dir_all(&cache_zisk_path).with_context(|| {
                    format!("Failed to remove directory {}", cache_zisk_path.display())
                })?;

                tracing::info!(
                    "{}",
                    format!("Successfully removed {}", cache_zisk_path.display())
                        .bright_green()
                        .bold()
                );
            } else {
                if self.elf.is_none() {
                    self.elf = match detect_current_project_elf()? {
                        Some(elf) => Some(elf),
                        None => {
                            anyhow::bail!(
                                "No ELF file provided, and could not detect a project ELF in the current directory. Please provide an ELF file with --elf."
                            );
                        }
                    };
                }

                tracing::info!(
                    "Cleaning zisk cache for ELF: {}",
                    self.elf.as_ref().unwrap().display()
                );

                let elf_hash = get_elf_data_hash(&std::fs::read(self.elf.as_ref().unwrap())?);

                let mut files_deleted = 0;

                // Delete all files in the cache directory that contain the ELF hash in their name
                for entry in std::fs::read_dir(&cache_zisk_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        let file_name = path
                            .file_name()
                            .with_context(|| {
                                format!(
                                    "Failed to get file name for cache entry {}",
                                    path.display()
                                )
                            })?
                            .to_string_lossy();
                        if file_name.contains(&elf_hash) {
                            std::fs::remove_file(&path)?;
                            files_deleted += 1;
                            tracing::debug!("Removed cache file: {}", file_name);
                        }
                    }
                }

                // Delete hints marker file if it exists
                let elf_file_name = self
                    .elf
                    .as_ref()
                    .context("Error getting ELF path")?
                    .file_name()
                    .context("Error getting ELF file name")?
                    .to_string_lossy();
                let hints_marker = cache_zisk_path.join(format!("{}.assembly_hints", elf_file_name));
                if hints_marker.exists() {
                    std::fs::remove_file(&hints_marker)?;
                    files_deleted += 1;
                    tracing::debug!("Removed hints marker file: {}", hints_marker.display());
                }

                // Show success or info message based on files deleted
                if files_deleted > 0 {
                    tracing::info!(
                        "{}",
                        "Successfully removed files from zisk cache".bright_green().bold()
                    );
                } else {
                    tracing::info!(
                        "{}",
                        "No cache files found for the specified ELF".bright_yellow().bold()
                    );
                }
            }
        } else {
            tracing::info!("No zisk cache directory found at {}", cache_zisk_path.display());
        }

        Ok(())
    }
}
