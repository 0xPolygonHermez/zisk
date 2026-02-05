use anyhow::{Context, Result};
use fields::{Goldilocks, PrimeField64};
use proofman_common::{
    write_custom_commit_trace, GlobalInfo, ProofCtx, ProofType, ProofmanResult, StarkInfo,
};
use sm_rom::RomSM;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use zisk_common::ElfBinaryLike;
use zisk_pil::{RomRomTrace, PILOUT_HASH};

pub const DEFAULT_CACHE_PATH: &str = ".zisk/cache";

/// Gets the user's home directory as specified by the HOME environment variable.
pub fn get_home_dir() -> String {
    env::var("HOME").expect("get_home_dir() failed to get HOME environment variable")
}

/// Gets the default zisk folder location in the home installation directory.
pub fn get_default_zisk_path() -> PathBuf {
    let zisk_path = format!("{}/.zisk/zisk", get_home_dir());
    PathBuf::from(zisk_path)
}

/// Gets the zisk folder.
/// Uses the default one if not specified by user.
pub fn get_zisk_path(zisk_path: Option<&PathBuf>) -> PathBuf {
    zisk_path.cloned().unwrap_or_else(get_default_zisk_path)
}

pub fn get_output_path(output_dir: &Option<PathBuf>) -> Result<PathBuf> {
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

    Ok(output_path)
}

pub fn gen_elf_hash<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf: &[u8],
    rom_buffer_path: &Path,
    blowup_factor: u64,
    merkle_tree_arity: u64,
) -> ProofmanResult<Vec<F>> {
    let buffer = vec![F::ZERO; RomRomTrace::<F>::NUM_ROWS * RomRomTrace::<F>::ROW_SIZE];
    let mut custom_rom_trace: RomRomTrace<F> = RomRomTrace::new_from_vec(buffer)?;

    RomSM::compute_custom_trace_rom(elf, &mut custom_rom_trace);

    write_custom_commit_trace(
        pctx,
        &mut custom_rom_trace,
        blowup_factor,
        merkle_tree_arity,
        rom_buffer_path,
    )
}

pub fn get_elf_vk(verkey_path: &Path) -> Result<Option<Vec<u8>>> {
    if !verkey_path.exists() {
        return Ok(None);
    }

    let mut file = File::open(verkey_path)?;
    let mut root_bytes = [0u8; 32];
    file.read_exact(&mut root_bytes)?;
    Ok(Some(root_bytes.to_vec()))
}

pub fn get_elf_data_hash_from_path(elf_path: &Path) -> Result<String> {
    let elf_data =
        fs::read(elf_path).with_context(|| format!("Error reading ELF file: {elf_path:?}"))?;

    let hash = blake3::hash(&elf_data).to_hex().to_string();

    Ok(hash)
}

pub fn get_elf_data_hash(elf: &impl ElfBinaryLike) -> Result<String> {
    let hash = blake3::hash(elf.elf()).to_hex().to_string();
    Ok(hash)
}

pub fn get_elf_bin_file_path_with_hash(
    hash: &str,
    default_cache_path: &Path,
    blowup_factor: u64,
    arity: u64,
) -> Result<PathBuf> {
    let pilout_hash = PILOUT_HASH;

    let n = RomRomTrace::<Goldilocks>::NUM_ROWS;

    let gpu = if cfg!(feature = "gpu") { "_gpu" } else { "" };
    let rom_cache_file_name = format!(
        "{}_{}_{}_{}_{}{}.bin",
        hash,
        pilout_hash,
        &n.to_string(),
        &blowup_factor.to_string(),
        &arity.to_string(),
        gpu
    );

    Ok(default_cache_path.join(rom_cache_file_name))
}

pub fn get_elf_bin_verkey_file_path_with_hash(
    hash: &str,
    default_cache_path: &Path,
    blowup_factor: u64,
    arity: u64,
) -> Result<PathBuf> {
    let pilout_hash = PILOUT_HASH;

    let n = RomRomTrace::<Goldilocks>::NUM_ROWS;

    let gpu = if cfg!(feature = "gpu") { "_gpu" } else { "" };
    let rom_cache_file_name = format!(
        "{}_{}_{}_{}_{}{}.verkey.bin",
        hash,
        pilout_hash,
        &n.to_string(),
        &blowup_factor.to_string(),
        &arity.to_string(),
        gpu
    );

    Ok(default_cache_path.join(rom_cache_file_name))
}

pub struct RomInfo {
    pub blowup_factor: u64,
    pub merkle_tree_arity: u64,
}

pub fn get_rom_info(proving_key_path: &Path) -> ProofmanResult<RomInfo> {
    let global_info =
        GlobalInfo::new(proving_key_path).expect("Failed to load global info from proving key");
    let (airgroup_id, air_id) = global_info.get_air_id("Zisk", "Rom");
    let setup_path = global_info.get_air_setup_path(airgroup_id, air_id, &ProofType::Basic);
    let stark_info_path = setup_path.display().to_string() + ".starkinfo.json";
    let stark_info_json = std::fs::read_to_string(&stark_info_path)
        .unwrap_or_else(|_| panic!("Failed to read file {}", &stark_info_path));
    let stark_info = StarkInfo::from_json(&stark_info_json);
    Ok(RomInfo {
        blowup_factor: 1 << (stark_info.stark_struct.n_bits_ext - stark_info.stark_struct.n_bits),
        merkle_tree_arity: stark_info.stark_struct.merkle_tree_arity,
    })
}

pub fn ensure_dir_exists(path: &PathBuf) {
    if let Err(e) = std::fs::create_dir_all(path) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Failed to create cache directory {path:?}: {e}");
        }
    }
}
