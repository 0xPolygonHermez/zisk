use std::path::{Path, PathBuf};

use crate::{gen_elf_hash, get_rom_blowup_factor_and_arity};
use fields::{Goldilocks, PrimeField};
use std::fs;
use std::fs::File;
use std::io::Write;

pub fn rom_vkey(
    elf: &Path,
    verkey_file: &Option<PathBuf>,
    proving_key: &Path,
) -> Result<Vec<Goldilocks>, anyhow::Error> {
    // Check if the path is a file and not a directory
    if !elf.is_file() {
        tracing::error!("Error: The specified ROM path is not a file: {}", elf.display());
        std::process::exit(1);
    }

    let (blowup_factor, merkle_tree_arity) = get_rom_blowup_factor_and_arity(proving_key);

    let root = gen_elf_hash(elf, &PathBuf::new(), blowup_factor, merkle_tree_arity, false)?;

    let verkey: Vec<u8> =
        root.iter().flat_map(|x| x.as_canonical_biguint().to_bytes_le()).collect();

    if let Some(verkey_file) = verkey_file {
        let parent = Path::new(&verkey_file).parent().unwrap();
        fs::create_dir_all(parent)?;
        let mut file = File::create(verkey_file)?;
        file.write_all(&verkey)?;
        file.flush()?;
    }

    tracing::info!("Root hash: {:?}", root);

    Ok(root)
}
