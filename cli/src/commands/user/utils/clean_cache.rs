use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use colored::Colorize;
use proofman_common::VerboseMode;
use rom_setup::get_elf_data_hash;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::ZiskPaths;
use zisk_prover_backend::setup_logger;

use crate::{
    common::detect_current_project_elf,
    ux::{print_banner, print_banner_command},
};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Clean the zisk cache directory for a specific program or the entire cache
pub(crate) struct ZiskCleanCache {
    /// Path of the program ELF file to clean cache for. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long, conflicts_with = "all")]
    elf: Option<PathBuf>,

    /// Clean cache for all programs (mutually exclusive with `--elf`)
    #[arg(short = 'a', long, conflicts_with = "elf")]
    all: bool,
}

impl ZiskCleanCache {
    pub(crate) fn run(&mut self) -> Result<()> {
        setup_logger(VerboseMode::Info);

        print_banner();
        print_banner_command("Clean");

        let cache_zisk_path = ZiskPaths::global().cache.clone();

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

                let elf_file_name = self
                    .elf
                    .as_ref()
                    .context("Error getting ELF path")?
                    .file_name()
                    .context("Error getting ELF file name")?
                    .to_string_lossy()
                    .into_owned();

                let files_deleted =
                    Self::clean_cache_for_elf(&cache_zisk_path, &elf_hash, &elf_file_name)?;

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

    /// Remove every file in `cache_dir` whose name contains `elf_hash`, plus the
    /// `<elf_file_name>.assembly_hints` marker if present, returning the number
    /// of files removed. Scoped to the given directory (no global state), so it
    /// is exercised against a temp cache dir in tests.
    fn clean_cache_for_elf(cache_dir: &Path, elf_hash: &str, elf_file_name: &str) -> Result<usize> {
        let mut files_deleted = 0;

        for entry in std::fs::read_dir(cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let file_name = path
                    .file_name()
                    .with_context(|| {
                        format!("Failed to get file name for cache entry {}", path.display())
                    })?
                    .to_string_lossy()
                    .into_owned();
                if file_name.contains(elf_hash) {
                    std::fs::remove_file(&path)?;
                    files_deleted += 1;
                    tracing::debug!("Removed cache file: {}", file_name);
                }
            }
        }

        let hints_marker = cache_dir.join(format!("{elf_file_name}.assembly_hints"));
        if hints_marker.exists() {
            std::fs::remove_file(&hints_marker)?;
            files_deleted += 1;
            tracing::debug!("Removed hints marker file: {}", hints_marker.display());
        }

        Ok(files_deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::ZiskCleanCache;
    use tempfile::tempdir;

    #[test]
    fn removes_only_files_matching_the_hash_and_the_marker() {
        let dir = tempdir().unwrap();
        let cache = dir.path();
        let hash = "deadbeef";

        // Matching cache files (hash in name).
        std::fs::write(cache.join(format!("rom-{hash}.bin")), b"x").unwrap();
        std::fs::write(cache.join(format!("{hash}-setup.bin")), b"x").unwrap();
        // Non-matching files must survive.
        std::fs::write(cache.join("other-program.bin"), b"x").unwrap();
        std::fs::write(cache.join("cafef00d-rom.bin"), b"x").unwrap();
        // Hints marker for our ELF.
        std::fs::write(cache.join("guest.elf.assembly_hints"), b"x").unwrap();

        let deleted = ZiskCleanCache::clean_cache_for_elf(cache, hash, "guest.elf").unwrap();

        assert_eq!(deleted, 3, "two hash files + one hints marker");
        assert!(!cache.join(format!("rom-{hash}.bin")).exists());
        assert!(!cache.join(format!("{hash}-setup.bin")).exists());
        assert!(!cache.join("guest.elf.assembly_hints").exists());
        // Unrelated files preserved.
        assert!(cache.join("other-program.bin").exists());
        assert!(cache.join("cafef00d-rom.bin").exists());
    }

    #[test]
    fn returns_zero_when_nothing_matches() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("unrelated.bin"), b"x").unwrap();

        let deleted =
            ZiskCleanCache::clean_cache_for_elf(dir.path(), "nomatch", "guest.elf").unwrap();
        assert_eq!(deleted, 0);
        assert!(dir.path().join("unrelated.bin").exists());
    }

    #[test]
    fn marker_counted_even_without_hash_matches() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("prog.elf.assembly_hints"), b"x").unwrap();

        let deleted =
            ZiskCleanCache::clean_cache_for_elf(dir.path(), "nomatch", "prog.elf").unwrap();
        assert_eq!(deleted, 1);
        assert!(!dir.path().join("prog.elf.assembly_hints").exists());
    }
}
