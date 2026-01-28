use fields::PrimeField;
use std::path::{Path, PathBuf};

use crate::{
    gen_elf_hash, get_elf_bin_file_path_with_hash, get_elf_data_hash, get_output_path, get_rom_info,
};

pub fn rom_merkle_setup(
    elf: &Path,
    output_dir: &Option<PathBuf>,
    proving_key: &Path,
    mut check: bool,
) -> Result<(PathBuf, Vec<u8>), anyhow::Error> {
    // Check if the path is a file and not a directory
    if !elf.is_file() {
        tracing::error!("Error: The specified ROM path is not a file: {}", elf.display());
        std::process::exit(1);
    }

    let output_path = get_output_path(output_dir)?;

    let elf_hash = get_elf_data_hash(elf)?;

    let rom_info = get_rom_info(proving_key)?;

    let elf_bin_path = get_elf_bin_file_path_with_hash(
        elf,
        &elf_hash,
        &output_path,
        rom_info.blowup_factor,
        rom_info.merkle_tree_arity,
    )?;

    if !elf_bin_path.exists() {
        check = false;
    }

    let root = gen_elf_hash(
        elf,
        elf_bin_path.as_path(),
        rom_info.blowup_factor,
        rom_info.merkle_tree_arity,
        check,
    )?;

    tracing::info!("Root hash: {:?}", root);

    let verkey: Vec<u8> =
        root.iter().flat_map(|x| x.as_canonical_biguint().to_bytes_le()).collect();

    Ok((elf_bin_path, verkey))
}
