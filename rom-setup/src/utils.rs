use anyhow::{Context, Result};
use fields::{Goldilocks, PrimeField64};
use proofman_common::{write_custom_commit_trace, ProofCtx, ProofmanError, ProofmanResult};
use sm_rom::CustomRom;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use zisk_common::{ZiskPaths, PROGRAM_VK_LEN};
use zisk_pil::{RomRomTrace, PILOUT_HASH};

pub const ROM_MERKLE_TREE_ARITY: u64 = 4;
pub const ROM_BLOWUP_FACTOR: u64 = 2;

pub fn get_output_path(output_dir: &Option<PathBuf>) -> Result<PathBuf> {
    let output_path = if output_dir.is_none() {
        let cache_path = ZiskPaths::global().cache.clone();
        ensure_dir_exists(&cache_path);
        cache_path
    } else {
        ensure_dir_exists(output_dir.as_ref().unwrap());
        output_dir.clone().unwrap()
    };

    let output_path = fs::canonicalize(&output_path)
        .unwrap_or_else(|_| panic!("Failed to get absolute path for {output_path:?}"));

    Ok(output_path)
}

pub fn gen_elf_hash<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf: &[u8],
    rom_buffer_path: &Path,
    blowup_factor: u64,
    merkle_tree_arity: u64,
) -> ProofmanResult<Vec<F>> {
    let mut custom_rom_trace =
        CustomRom::build::<F>(elf).map_err(|e| ProofmanError::InvalidParameters(e.to_string()))?;

    write_custom_commit_trace(
        pctx,
        &mut custom_rom_trace,
        blowup_factor,
        merkle_tree_arity,
        rom_buffer_path,
    )
}

pub fn get_elf_vk(verkey_path: &Path) -> Result<Option<Vec<u64>>> {
    if !verkey_path.exists() {
        return Ok(None);
    }

    let mut file = File::open(verkey_path)?;
    let mut vk = vec![0u64; PROGRAM_VK_LEN];
    for word in vk.iter_mut() {
        let mut buf = [0u8; 8];
        file.read_exact(&mut buf)?;
        *word = u64::from_le_bytes(buf);
    }
    Ok(Some(vk))
}

pub fn get_elf_data_hash_from_path(elf_path: &Path) -> Result<String> {
    let elf_data =
        fs::read(elf_path).with_context(|| format!("Error reading ELF file: {elf_path:?}"))?;

    let hash = blake3::hash(&elf_data).to_hex().to_string();

    Ok(hash)
}

pub fn get_elf_data_hash(elf: &[u8]) -> String {
    blake3::hash(elf).to_hex().to_string()
}

/// Hash of the ROM inputs (transpiler + embedded float ELF), baked by
/// `build.rs`. Folded into the ROM/verkey filenames so a transpiler or float
/// change invalidates them — otherwise a warm cache returns a verkey stale
/// w.r.t. the current `zisk-core`, i.e. the wrong program vk.
const ROM_INPUTS_HASH: &str = env!("ZISK_ROM_INPUTS_HASH");

pub fn get_elf_bin_file_path_with_hash(
    hash: &str,
    default_cache_path: &Path,
    gpu: bool,
) -> Result<PathBuf> {
    let pilout_hash = PILOUT_HASH;

    let n = RomRomTrace::<Goldilocks>::NUM_ROWS;

    let gpu_suffix = if gpu { "_gpu" } else { "" };
    let rom_cache_file_name = format!(
        "{}-d{}_{}_{}_{}_{}{}.bin",
        hash,
        ROM_INPUTS_HASH,
        pilout_hash,
        &n.to_string(),
        &ROM_BLOWUP_FACTOR.to_string(),
        &ROM_MERKLE_TREE_ARITY.to_string(),
        gpu_suffix
    );

    Ok(default_cache_path.join(rom_cache_file_name))
}

pub fn get_elf_bin_verkey_file_path_with_hash(
    hash: &str,
    default_cache_path: &Path,
) -> Result<PathBuf> {
    let pilout_hash = PILOUT_HASH;

    let n = RomRomTrace::<Goldilocks>::NUM_ROWS;

    let rom_cache_file_name = format!(
        "{}-d{}_{}_{}_{}_{}.verkey.bin",
        hash,
        ROM_INPUTS_HASH,
        pilout_hash,
        &n.to_string(),
        &ROM_BLOWUP_FACTOR.to_string(),
        &ROM_MERKLE_TREE_ARITY.to_string(),
    );

    Ok(default_cache_path.join(rom_cache_file_name))
}

pub struct RomInfo {
    pub blowup_factor: u64,
    pub merkle_tree_arity: u64,
}

pub fn ensure_dir_exists(path: &PathBuf) {
    if let Err(e) = std::fs::create_dir_all(path) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Failed to create cache directory {path:?}: {e}");
        }
    }
}
