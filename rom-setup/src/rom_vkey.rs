use std::path::{Path, PathBuf};

use crate::{gen_elf_hash, get_rom_info};
use fields::PrimeField;
use std::fs;
use std::fs::File;
use std::io::Write;

pub fn rom_vkey(
    elf: &Path,
    verkey_file: &Option<PathBuf>,
    proving_key: &Path,
) -> Result<(Vec<u8>, u64), anyhow::Error> {
    // Check if the path is a file and not a directory
    if !elf.is_file() {
        tracing::error!("Error: The specified ROM path is not a file: {}", elf.display());
        std::process::exit(1);
    }

    let rom_info = get_rom_info(proving_key)?;

    let root = gen_elf_hash(
        elf,
        &PathBuf::new(),
        rom_info.blowup_factor,
        rom_info.merkle_tree_arity,
        false,
    )?;

    tracing::info!("Root hash: {:?}", root);

    let verkey: Vec<u8> =
        root.iter().flat_map(|x| x.as_canonical_biguint().to_bytes_le()).collect();

    if let Some(verkey_file) = verkey_file {
        let parent = Path::new(&verkey_file).parent().unwrap();
        fs::create_dir_all(parent)?;
        let mut file = File::create(verkey_file)?;
        file.write_all(&verkey)?;
        file.flush()?;
    }

    Ok((verkey, rom_info.starting_pos_publics_program_vk))
}

/// Verify that the program VK publics match the proof's public values.
///
/// `program_vk` contains 4 publics, each is a u64 (8 bytes), so 32 bytes total.
/// This function compares `program_vk` with the bytes in `public_values` starting at `starting_pos`.
pub fn verify_program_vk_publics(
    program_vk: &[u8],
    starting_pos: u64,
    public_values: &[u8],
) -> Result<(), anyhow::Error> {
    let end = starting_pos as usize + program_vk.len();

    if public_values.len() < end {
        return Err(anyhow::anyhow!(
            "Proof public values too short: expected at least {} bytes, got {}",
            end,
            public_values.len()
        ));
    }

    let proof_publics = &public_values[starting_pos as usize..end];

    if program_vk != proof_publics {
        return Err(anyhow::anyhow!("Program VK publics do not match proof public values"));
    }

    Ok(())
}
