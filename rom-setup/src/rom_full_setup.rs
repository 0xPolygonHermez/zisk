use std::{
    fs,
    path::{Path, PathBuf},
};

use colored::Colorize;

use crate::{get_elf_data_hash, DEFAULT_CACHE_PATH};

pub fn rom_full_setup(
    elf: &PathBuf,
    proving_key: &Path,
    zisk_path: &Path,
    output_dir: &Option<PathBuf>,
    verbose: bool,
) -> std::result::Result<(), anyhow::Error> {
    let output_path = if output_dir.is_none() {
        let cache_path = std::env::var("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(DEFAULT_CACHE_PATH))
            .unwrap_or_else(|_| panic!("$HOME environment variable is not set"));

        ensure_dir_exists(&cache_path);
        cache_path
    } else {
        ensure_dir_exists(output_dir.as_ref().unwrap());
        output_dir.clone().unwrap()
    };

    let output_path = fs::canonicalize(&output_path)
        .unwrap_or_else(|_| panic!("Failed to get absolute path for {output_path:?}"));

    println!();

    tracing::info!("Computing setup for ROM {}", elf.display());

    tracing::info!("Computing ELF hash");
    let elf_hash = get_elf_data_hash(elf)?;

    tracing::info!("Computing assembly setup");
    crate::generate_assembly(elf, &elf_hash, zisk_path, output_path.as_path(), verbose)?;

    tracing::info!("Computing merkle root");
    crate::rom_merkle_setup(elf, &elf_hash, output_path.as_path(), proving_key, false)?;

    tracing::info!("Computing Verification key");

    crate::rom_vkey()?;

    println!();
    tracing::info!(
        "{} {}",
        "ROM setup successfully completed at".bright_green().bold(),
        output_path.display()
    );

    Ok(())
}

fn ensure_dir_exists(path: &PathBuf) {
    if let Err(e) = std::fs::create_dir_all(path) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Failed to create cache directory {path:?}: {e}");
        }
    }
}
