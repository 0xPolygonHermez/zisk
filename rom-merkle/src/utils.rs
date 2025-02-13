use p3_goldilocks::Goldilocks;
use proofman_common::write_custom_commit_trace;
use sm_rom::RomSM;
use std::path::Path;
use zisk_pil::RomRomTrace;

pub fn gen_elf_hash(
    elf_path: &Path,
    buffer_file: &str,
    blowup_factor: u64,
    check: bool,
) -> Result<Vec<Goldilocks>, Box<dyn std::error::Error>> {
    let mut custom_rom_trace: RomRomTrace<Goldilocks> = RomRomTrace::new();

    RomSM::compute_custom_trace_rom(elf_path.to_path_buf(), &mut custom_rom_trace);

    write_custom_commit_trace(&mut custom_rom_trace, blowup_factor, buffer_file, check)
}
