use anyhow::{Context, Result};
use p3_goldilocks::Goldilocks;
use proofman_common::{write_custom_commit_trace, GlobalInfo, ProofType, StarkInfo};
use sm_rom::RomSM;
use std::fs;
use std::path::{Path, PathBuf};
use zisk_pil::{RomRomHash, RomRomTrace, PILOUT_HASH};

pub const DEFAULT_CACHE_PATH: &str = ".zisk/cache";

pub fn gen_elf_hash(
    rom_path: &Path,
    rom_buffer_str: &str,
    blowup_factor: u64,
    check: bool,
) -> Result<Vec<Goldilocks>, Box<dyn std::error::Error>> {
    let mut custom_rom_trace: RomRomTrace<Goldilocks> = RomRomTrace::new();

    RomSM::compute_custom_trace_rom(rom_path.to_path_buf(), &mut custom_rom_trace);

    write_custom_commit_trace(&mut custom_rom_trace, blowup_factor, rom_buffer_str, check)
}

pub fn get_elf_bin_file_path(
    elf_path: &PathBuf,
    default_cache_path: &Path,
    blowup_factor: u64,
) -> Result<PathBuf> {
    let elf_data =
        fs::read(elf_path).with_context(|| format!("Error reading ELF file: {:?}", elf_path))?;

    let hash = blake3::hash(&elf_data).to_hex().to_string();

    let pilout_hash = PILOUT_HASH;

    let n = RomRomTrace::<usize>::NUM_ROWS;

    let rom_cache_file_name =
        format!("{}_{}_{}_{}.bin", hash, pilout_hash, &n.to_string(), &blowup_factor.to_string());

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
