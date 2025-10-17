use anyhow::{Context, Result};
use fields::{Field, Goldilocks};
use proofman_common::{write_custom_commit_trace, GlobalInfo, ProofType, StarkInfo};
use sm_rom::RomSM;
use std::fs;
use std::path::{Path, PathBuf};
use zisk_pil::{RomRomTrace, PILOUT_HASH};

pub const DEFAULT_CACHE_PATH: &str = ".zisk/cache";

pub fn gen_elf_hash(
    rom_path: &Path,
    rom_buffer_path: &Path,
    blowup_factor: u64,
    check: bool,
) -> Result<Vec<Goldilocks>, anyhow::Error> {
    let buffer = vec![
        Goldilocks::ZERO;
        RomRomTrace::<Goldilocks>::NUM_ROWS * RomRomTrace::<Goldilocks>::ROW_SIZE
    ];
    let mut custom_rom_trace: RomRomTrace<Goldilocks> = RomRomTrace::new_from_vec(buffer);

    RomSM::compute_custom_trace_rom(rom_path.to_path_buf(), &mut custom_rom_trace);

    let result =
        write_custom_commit_trace(&mut custom_rom_trace, blowup_factor, rom_buffer_path, check)
            .map_err(|e| anyhow::anyhow!("Error writing custom commit trace: {}", e))?;

    Ok(result)
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
) -> Result<PathBuf> {
    let elf_data =
        fs::read(elf_path).with_context(|| format!("Error reading ELF file: {elf_path:?}"))?;

    let hash = blake3::hash(&elf_data).to_hex().to_string();

    get_elf_bin_file_path_with_hash(elf_path, &hash, default_cache_path, blowup_factor)
}

pub fn get_elf_bin_file_path_with_hash(
    elf_path: &Path,
    hash: &str,
    default_cache_path: &Path,
    blowup_factor: u64,
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
        "{}_{}_{}_{}{}.bin",
        hash,
        pilout_hash,
        &n.to_string(),
        &blowup_factor.to_string(),
        gpu
    );

    Ok(default_cache_path.join(rom_cache_file_name))
}

pub fn get_rom_blowup_factor(proving_key_path: &Path) -> u64 {
    let global_info = GlobalInfo::new(proving_key_path);
    let (airgroup_id, air_id) = global_info.get_air_id("Zisk", "Rom");
    let setup_path = global_info.get_air_setup_path(airgroup_id, air_id, &ProofType::Basic);
    let stark_info_path = setup_path.display().to_string() + ".starkinfo.json";
    let stark_info_json = std::fs::read_to_string(&stark_info_path)
        .unwrap_or_else(|_| panic!("Failed to read file {}", &stark_info_path));
    let stark_info = StarkInfo::from_json(&stark_info_json);

    1 << (stark_info.stark_struct.n_bits_ext - stark_info.stark_struct.n_bits)
}
