use std::path::{Path, PathBuf};

use crate::{gen_elf_hash, get_elf_bin_file_path, get_rom_blowup_factor, DEFAULT_CACHE_PATH};

pub fn rom_merkle_setup(elf: &Path, proving_key: &Path, check: bool) -> Result<(), anyhow::Error> {
    let default_cache_path =
        std::env::var("HOME").ok().map(PathBuf::from).unwrap().join(DEFAULT_CACHE_PATH);

    // Check if the path exists
    if !elf.exists() {
        log::error!("Error: The specified ROM file does not exist: {}", elf.display());
        std::process::exit(1);
    }

    // Check if the path is a file and not a directory
    if !elf.is_file() {
        log::error!("Error: The specified ROM path is not a file: {}", elf.display());
        std::process::exit(1);
    }

    let blowup_factor = get_rom_blowup_factor(proving_key);

    let elf_bin_path = get_elf_bin_file_path(elf, &default_cache_path, blowup_factor)?;

    let root = gen_elf_hash(elf, elf_bin_path.as_path(), blowup_factor, check)?;

    println!("Root hash: {:?}", root);
    println!("ROM hash computed successfully");

    Ok(())
}
