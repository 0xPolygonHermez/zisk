use std::path::Path;

use crate::{gen_elf_hash, get_elf_bin_file_path_with_hash, get_rom_blowup_factor};

pub fn rom_merkle_setup(
    elf: &Path,
    elf_hash: &str,
    output_path: &Path,
    proving_key: &Path,
    mut check: bool,
) -> Result<(), anyhow::Error> {
    // Check if the path is a file and not a directory
    if !elf.is_file() {
        tracing::error!("Error: The specified ROM path is not a file: {}", elf.display());
        std::process::exit(1);
    }

    let blowup_factor = get_rom_blowup_factor(proving_key);

    let elf_bin_path = get_elf_bin_file_path_with_hash(elf, elf_hash, output_path, blowup_factor)?;

    if !elf_bin_path.exists() {
        check = false;
    }

    let root = gen_elf_hash(elf, elf_bin_path.as_path(), blowup_factor, check)?;

    tracing::info!("Root hash: {:?}", root);

    Ok(())
}
