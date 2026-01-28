use anyhow::{Context, Result};
use fields::{Field, Goldilocks};
use proofman_common::{
    write_custom_commit_trace, GlobalInfo, ProofType, ProofmanResult, StarkInfo,
};
use sm_rom::RomSM;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
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

pub fn gen_elf_hash(
    rom_path: &Path,
    rom_buffer_path: &Path,
    blowup_factor: u64,
    merkle_tree_arity: u64,
    check: bool,
) -> ProofmanResult<Vec<Goldilocks>> {
    let buffer = vec![
        Goldilocks::ZERO;
        RomRomTrace::<Goldilocks>::NUM_ROWS * RomRomTrace::<Goldilocks>::ROW_SIZE
    ];
    let mut custom_rom_trace: RomRomTrace<Goldilocks> = RomRomTrace::new_from_vec(buffer)?;

    RomSM::compute_custom_trace_rom(rom_path.to_path_buf(), &mut custom_rom_trace);

    write_custom_commit_trace(
        &mut custom_rom_trace,
        blowup_factor,
        merkle_tree_arity,
        rom_buffer_path,
        check,
    )
}

pub fn get_elf_data_hash(elf_path: &Path) -> Result<String> {
    let elf_data =
        fs::read(elf_path).with_context(|| format!("Error reading ELF file: {elf_path:?}"))?;

    let hash = blake3::hash(&elf_data).to_hex().to_string();

    Ok(hash)
}

pub fn get_elf_bin_file_path(
    elf_path: &Path,
    default_cache_path: &Path,
    blowup_factor: u64,
    arity: u64,
) -> Result<PathBuf> {
    let elf_data =
        fs::read(elf_path).with_context(|| format!("Error reading ELF file: {elf_path:?}"))?;

    let hash = blake3::hash(&elf_data).to_hex().to_string();

    get_elf_bin_file_path_with_hash(elf_path, &hash, default_cache_path, blowup_factor, arity)
}

pub fn get_elf_bin_file_path_with_hash(
    elf_path: &Path,
    hash: &str,
    default_cache_path: &Path,
    blowup_factor: u64,
    arity: u64,
) -> Result<PathBuf> {
    if !elf_path.is_file() {
        return Err(anyhow::anyhow!(
            "Error: The specified ROM path is not a file: {}",
            elf_path.display()
        ));
    }
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

pub struct RomInfo {
    pub blowup_factor: u64,
    pub merkle_tree_arity: u64,
    pub starting_pos_publics_program_vk: u64,
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
    let publics_pos = global_info.get_public_starting_pos("rom_root")?;
    Ok(RomInfo {
        blowup_factor: 1 << (stark_info.stark_struct.n_bits_ext - stark_info.stark_struct.n_bits),
        merkle_tree_arity: stark_info.stark_struct.merkle_tree_arity,
        starting_pos_publics_program_vk: publics_pos as u64,
    })
}

pub fn ensure_dir_exists(path: &PathBuf) {
    if let Err(e) = std::fs::create_dir_all(path) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Failed to create cache directory {path:?}: {e}");
        }
    }
}
