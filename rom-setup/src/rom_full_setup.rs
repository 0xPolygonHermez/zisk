use std::{
    fs,
    path::{Path, PathBuf},
};

use colored::Colorize;

use crate::{ensure_dir_exists, get_elf_data_hash, DEFAULT_CACHE_PATH};

#[allow(unused_variables)]
pub fn rom_full_setup(
    elf: &Path,
    proving_key: &Path,
    zisk_path: &Path,
    output_dir: &Option<PathBuf>,
    hints: bool,
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

    tracing::info!("Computing merkle root");
    crate::rom_merkle_setup(elf, &elf_hash, output_path.as_path(), proving_key, false)?;
    // Assembly setup is not needed on macOS due to the lack of support for assembly generation.
    #[cfg(not(target_os = "macos"))]
    {
        tracing::info!("Computing assembly setup");
        crate::generate_assembly(elf, &elf_hash, zisk_path, output_path.as_path(), hints, verbose)?;
    }

    println!();
    tracing::info!(
        "{} {}",
        "ROM setup successfully completed at".bright_green().bold(),
        output_path.display()
    );

    Ok(())
}
