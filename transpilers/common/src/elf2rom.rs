//! Reads RISC-V data from and ELF file and converts it to a ZiskRom

use crate::elf_extraction::{
    collect_elf_payload_from_bytes, get_symbol_addresses_from_bytes, merge_ro_sections,
    validate_entry_point, ElfPayload,
};
use std::{error::Error, path::Path};
use zisk_core::mem::DataSection;
use zisk_core::mem::{RAM_ADDR, RAM_SIZE, ROM_ADDR, ROM_ENTRY, ROM_SIZE};
use zisk_core::zisk_rom::{DataSection64, ZiskRom};
use zisk_core::zisk_rom_2_asm::{AsmGenerationMethod, ZiskRom2Asm};
use zisk_core::{FLOAT_LIB_RAM_ADDR, FLOAT_LIB_ROM_ADDR};
use zisk_riscv::riscv2zisk_context::{add_end_and_lib, add_entry_exit_jmp, add_zisk_code};

/// Executes the ROM transpilation process: from ELF to Zisk
pub fn elf2rom(elf: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    // Load the embedded float library (enabled with the `float` feature).
    #[cfg(feature = "float")]
    const FLOAT_LIB_DATA: &[u8] = include_bytes!("../../../lib-float/c/lib/ziskfloat.elf");

    // Extract all relevant sections from the ELF file
    #[cfg(feature = "float")]
    let payloads: Vec<ElfPayload> =
        vec![collect_elf_payload_from_bytes(FLOAT_LIB_DATA)?, collect_elf_payload_from_bytes(elf)?];
    #[cfg(not(feature = "float"))]
    let payloads: Vec<ElfPayload> = vec![collect_elf_payload_from_bytes(elf)?];

    // Record the ELF file index
    #[cfg(feature = "float")]
    let elf_index = 1;
    #[cfg(not(feature = "float"))]
    let elf_index = 0;

    // Without `ziskos::entrypoint!(main);` the linker can't resolve `_start`
    // to the ziskos boot thunk and emits `e_entry = 0`, which would crash the
    // emulator at PC=0 with a confusing out-of-rom error. Looking up a `main`
    // symbol is not a reliable signal: release-mode LTO inlines `main` into
    // `_zisk_main` and strips it from the symbol table.
    if payloads[elf_index].entry_point == 0 {
        return Err("Guest ELF has no entry point (e_entry=0x0). \
                    Declare `#![no_main]` and `ziskos::entrypoint!(main);` \
                    at the guest program root."
            .into());
    }

    // Validate the guest entry point: instruction-aligned (2 bytes, ZisK decodes
    // compressed instructions) and inside a loaded executable segment (ZisK reads
    // e_entry rather than booting from a fixed address).
    validate_entry_point(&payloads[elf_index])?;

    // Get DMA function addresses: (memcpy, memcmp, memset, memmove)
    let dma_addrs = get_dma_symbol_addresses(elf);

    // Create an empty ZiskRom instance
    let mut rom: ZiskRom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

    // Add the end instruction, jumping over it
    add_end_and_lib(&mut rom);

    // Store RO and RW data sections separately, as they will be treated differently when generating the ROM instructions
    let mut ro_data: Vec<DataSection> = Vec::new();
    let mut rw_data: Vec<DataSection> = Vec::new();

    for (i, payload) in payloads.into_iter().enumerate() {
        let ElfPayload { entry_point, exec, ro, rw } = payload;

        // Add executable code sections
        for section in &exec {
            add_zisk_code(&mut rom, section.addr, &section.data, dma_addrs);
        }

        // Add read-only data sections.  They will be stored in ROM, but there can be some RAM
        // regions marked as read-only as well, e.g. the output region
        for section in ro {
            if section.addr >= ROM_ADDR
                && (section.addr + section.data.len() as u64) <= (ROM_ADDR + ROM_SIZE)
            {
                // If this is a program section, it should not overlap with the float library region
                // If this is a float library section, it should not overlap with the program region
                if i == elf_index {
                    if section.addr + section.data.len() as u64 >= FLOAT_LIB_ROM_ADDR {
                        return Err(format!(
                            "ROM program data section at address 0x{:x} with size {} overlaps with the ZisK float library region",
                            section.addr,
                            section.data.len()
                        )
                        .into());
                    }
                } else if section.addr < FLOAT_LIB_ROM_ADDR {
                    return Err(format!(
                        "ROM float library data section at address 0x{:x} with size {} overlaps with the ZisK program region",
                        section.addr,
                        section.data.len()
                    )
                    .into());
                }
                ro_data.push(section);
            } else if section.addr >= RAM_ADDR
                && (section.addr + section.data.len() as u64) <= (RAM_ADDR + RAM_SIZE)
            {
                // If this is a program section, it should not overlap with the float library region
                // If this is a float library section, it should not overlap with the program region
                if i == elf_index {
                    if section.addr + section.data.len() as u64 >= FLOAT_LIB_RAM_ADDR {
                        return Err(format!(
                            "RAM program data section at address 0x{:x} with size {} overlaps with the ZisK float library region",
                            section.addr,
                            section.data.len()
                        )
                        .into());
                    }
                } else if section.addr < FLOAT_LIB_RAM_ADDR {
                    return Err(format!(
                        "RAM float library data section at address 0x{:x} with size {} overlaps with the ZisK program region",
                        section.addr,
                        section.data.len()
                    )
                    .into());
                }
                rw_data.push(section);
            } else {
                return Err(format!(
                    "Data section at address 0x{:x} with size {} is out of ROM and RAM bounds",
                    section.addr,
                    section.data.len()
                )
                .into());
            }
        }

        // Add read-write data sections (will be stored in RAM)
        rw_data.extend(rw);

        // Add entry and exit jump instructions, only for the guest ELF payload
        // (i.e. `payloads[elf_index]`)
        if i == elf_index {
            add_entry_exit_jmp(&mut rom, entry_point);
        }
    }

    // Merge and pad RO sections to a 32-byte multiple, coalescing any sections
    // that the padding would otherwise make overlap (see merge_ro_sections).
    ro_data = merge_ro_sections(ro_data)?;

    // Ensure every data section address is aligned to 8 bytes, and data length as well
    for section in &mut ro_data {
        if section.addr % 8 != 0 {
            return Err(format!(
                "RO data section at address 0x{:x} is not aligned to 8 bytes",
                section.addr
            )
            .into());
        }
        if section.data.len() % 8 != 0 {
            return Err(format!(
                "RO data section at address 0x{:x} has size {} which is not a multiple of 8 bytes",
                section.addr,
                section.data.len()
            )
            .into());
        }
    }

    // Normalize RW data sections for ROM-data initialization: trim zero padding,
    // merge sections that collide once expanded to 32-byte blocks, and carve out
    // every internal all-zero region >= 1 KiB (see normalize_rw_data_sections).
    rw_data = normalize_rw_data_sections(rw_data);

    for section in &mut rw_data {
        if section.addr % 8 != 0 {
            return Err(format!(
                "RW data section at address 0x{:x} is not aligned to 8 bytes",
                section.addr
            )
            .into());
        }
        if section.data.len() % 8 != 0 {
            return Err(format!(
                "RW data section at address 0x{:x} has size {} which is not a multiple of 8 bytes",
                section.addr,
                section.data.len()
            )
            .into());
        }
    }

    // Convert RO data sections to 64-bit data sections, and store them in the ROM
    rom.ro_data_64 = ro_data
        .into_iter()
        .map(|section| {
            let mut data = Vec::new();
            for chunk in section.data.chunks(8) {
                data.push(u64::from_le_bytes(chunk.try_into().unwrap()));
            }
            DataSection64 { addr: section.addr, data }
        })
        .collect();

    // Convert RW data sections to 64-bit data sections, and store them in the ROM
    rom.rw_data_64 = rw_data
        .into_iter()
        .map(|section| {
            let mut data = Vec::new();
            for chunk in section.data.chunks(8) {
                data.push(u64::from_le_bytes(chunk.try_into().unwrap()));
            }
            DataSection64 { addr: section.addr, data }
        })
        .collect();

    // Preprocess the ROM
    // Split the ROM instructions based on their address to improve performance when
    // searching for the instruction corresponding to the program counter (PC) address
    rom.optimize_instruction_lookup()?;

    Ok(rom)
}

/// A ROM data row initializes 32 bytes (4 u64), so RW sections must be sized in
/// whole 32-byte blocks.
const ROM_DATA_BLOCK: usize = 32;
/// Minimum internal all-zero region worth carving out of a section (1 KiB = 32
/// blocks). RAM reads as zero, so carved regions are simply left uninitialized.
const MIN_CARVE_ZEROS: usize = 1024;

/// Compile-time guarantee that RAM starts on a 32-byte block boundary. The section
/// normalization clamps the downward expansion to `RAM_ADDR`, which only preserves
/// block alignment (and the length-multiple-of-32 invariant) if `RAM_ADDR` is itself
/// 32-byte aligned. Fails the build if someone ever changes `RAM_ADDR` otherwise.
const _: () = assert!(
    RAM_ADDR % ROM_DATA_BLOCK as u64 == 0,
    "RAM_ADDR must be aligned to the 32-byte ROM data block size"
);

/// Appends the block range `[start_block, end_block)` of `section` as its own
/// `DataSection` (copied once), skipping empty ranges. Block indices are 32-byte
/// units, so the resulting address and length stay 32-byte aligned.
fn push_block_range(
    out: &mut Vec<DataSection>,
    section: &DataSection,
    start_block: usize,
    end_block: usize,
) {
    if end_block > start_block {
        let bytes = &section.data[start_block * ROM_DATA_BLOCK..end_block * ROM_DATA_BLOCK];
        out.push(DataSection {
            addr: section.addr + (start_block * ROM_DATA_BLOCK) as u64,
            data: bytes.to_vec(),
        });
    }
}

/// Normalizes RW data sections for ROM-data initialization. In one pass it:
/// 1. trims leading/trailing zeros and expands every section to whole 32-byte blocks
///    aligned on absolute 32-byte boundaries (dropping fully-zero sections),
/// 2. merges sections whose expanded ranges overlap or touch, so no address is
///    initialized twice (which the memory argument forbids), and
/// 3. carves out every internal all-zero region of >= 1 KiB (block-aligned),
///    splitting the sections around them so those zeros are not initialized.
///
/// Output invariants: every section is non-empty, both its address and length are
/// multiples of 32 bytes, and no two sections overlap. Sections are returned sorted
/// by address.
///
/// Aligning to absolute 32-byte boundaries (rather than to each section's own start)
/// keeps every section on the same block grid, so merging two sections always yields
/// a length that is still a multiple of 32.
///
/// The downward expansion is clamped to `RAM_ADDR` (the lowest writable RAM address):
/// it must never cross below it, since that region is the stack guard. `RAM_ADDR` is
/// 32-byte aligned (checked at compile time above), so clamping preserves alignment.
fn normalize_rw_data_sections(sections: Vec<DataSection>) -> Vec<DataSection> {
    const BLOCK: u64 = ROM_DATA_BLOCK as u64;

    // ---- Phase 1: trim + expand to absolute 32-byte block bounds -----------
    let mut expanded: Vec<DataSection> = sections
        .into_iter()
        .filter_map(|section| {
            let data = section.data;
            let len = data.len() as u64;

            // Locate the first and last non-zero bytes; drop fully-zero sections.
            let first = data.iter().position(|&b| b != 0)? as u64;
            let last = data.iter().rposition(|&b| b != 0).unwrap() as u64;

            // Expand [first_addr, last_addr] outwards to absolute 32-byte blocks.
            // `end_addr` is exclusive; both bounds are block-aligned so the length is
            // a multiple of 32. `start_addr` may fall below `section.addr` and
            // `end_addr` above its end when the section is not block-aligned; the
            // out-of-section bytes are zero-filled.
            let first_addr = section.addr + first;
            let last_addr = section.addr + last;
            // Clamp the downward expansion so it never crosses below RAM start.
            let start_addr = (first_addr & !(BLOCK - 1)).max(RAM_ADDR);
            let end_addr = (last_addr & !(BLOCK - 1)) + BLOCK;
            // A whole section below RAM start would be clamped to nothing; drop it
            // (such sections are already rejected by the RAM-bounds check upstream).
            if start_addr >= end_addr {
                return None;
            }

            let mut out = vec![0u8; (end_addr - start_addr) as usize];
            // Copy the bytes that actually exist ([section.addr, section.addr+len))
            // into their place within the expanded block range.
            let copy_from = start_addr.max(section.addr);
            let copy_to = end_addr.min(section.addr + len);
            let dst = (copy_from - start_addr) as usize;
            let src = (copy_from - section.addr) as usize;
            let n = (copy_to - copy_from) as usize;
            out[dst..dst + n].copy_from_slice(&data[src..src + n]);

            Some(DataSection { addr: start_addr, data: out })
        })
        .collect();

    // ---- Phase 2: merge sections that overlap or touch once expanded -------
    expanded.sort_by_key(|s| s.addr);
    let mut merged: Vec<DataSection> = Vec::with_capacity(expanded.len());
    for section in expanded {
        if let Some(prev) = merged.last_mut() {
            let prev_end = prev.addr + prev.data.len() as u64;
            if section.addr <= prev_end {
                // Overlap or touch: fold `section` into `prev`. Both are block-aligned
                // so the combined length stays a multiple of 32.
                let prev_orig_len = prev.data.len();
                let sec_end = section.addr + section.data.len() as u64;
                if sec_end > prev_end {
                    prev.data.resize((sec_end - prev.addr) as usize, 0);
                }
                let off = (section.addr - prev.addr) as usize;
                for (k, &b) in section.data.iter().enumerate() {
                    let idx = off + k;
                    if idx < prev_orig_len {
                        // In the overlap keep whatever is non-zero: real data never
                        // overlaps across sections, so at most one side is non-zero.
                        if b != 0 {
                            prev.data[idx] = b;
                        }
                    } else {
                        prev.data[idx] = b;
                    }
                }
                continue;
            }
        }
        merged.push(section);
    }

    // ---- Phase 3: carve out internal all-zero regions of >= MIN_CARVE_ZEROS ----
    // RAM reads as zero, so a large internal zero run needs no initialization: split
    // the section around it. Runs are internal by construction (phase 1 makes the
    // first and last block non-zero), so every emitted piece is non-empty.
    let min_blocks = MIN_CARVE_ZEROS / ROM_DATA_BLOCK;
    let block_is_zero = |data: &[u8], b: usize| {
        data[b * ROM_DATA_BLOCK..(b + 1) * ROM_DATA_BLOCK].iter().all(|&x| x == 0)
    };

    let mut result: Vec<DataSection> = Vec::with_capacity(merged.len());
    for section in merged {
        let nblocks = section.data.len() / ROM_DATA_BLOCK;
        let mut piece_start = 0usize; // first block of the current kept piece
        let mut carved = false;
        let mut b = 0;
        while b < nblocks {
            if !block_is_zero(&section.data, b) {
                b += 1;
                continue;
            }
            let run_start = b;
            while b < nblocks && block_is_zero(&section.data, b) {
                b += 1;
            }
            // Carve only large enough runs; smaller ones stay inside the piece.
            if b - run_start >= min_blocks {
                push_block_range(&mut result, &section, piece_start, run_start);
                piece_start = b;
                carved = true;
            }
        }
        if carved {
            push_block_range(&mut result, &section, piece_start, nblocks);
        } else {
            result.push(section); // untouched: no copy
        }
    }

    result
}

/// Get DMA function addresses from ELF data
/// Returns (memcpy, memcmp, memset, memmove), with 0 for missing symbols
fn get_dma_symbol_addresses(elf_data: &[u8]) -> (u64, u64, u64, u64) {
    let symbols = ["memcpy", "memcmp", "memset", "memmove"];
    match get_symbol_addresses_from_bytes(elf_data, &symbols) {
        Ok(addrs) => (
            addrs.get("memcpy").copied().unwrap_or(0),
            addrs.get("memcmp").copied().unwrap_or(0),
            addrs.get("memset").copied().unwrap_or(0),
            addrs.get("memmove").copied().unwrap_or(0),
        ),
        Err(_) => (0, 0, 0, 0),
    }
}

/// Executes the ELF file data transpilation process into a Zisk ROM, and saves the result into a
/// file.  The file format can be JSON, PIL-based or binary.
pub fn elf2romfile(
    elf: &[u8],
    asm_file: &Path,
    generation_method: AsmGenerationMethod,
    log_output: bool,
    comments: bool,
    hints: bool,
) -> Result<(), Box<dyn Error>> {
    let rom = elf2rom(elf)?;
    ZiskRom2Asm::save_to_asm_file(&rom, asm_file, generation_method, log_output, comments, hints);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures live inside real RAM so the RAM_ADDR clamp is exercised realistically.
    const BASE: u64 = RAM_ADDR;

    fn ds(addr: u64, data: Vec<u8>) -> DataSection {
        DataSection { addr, data }
    }

    /// Every output section must satisfy the normalization invariants.
    fn assert_invariants(out: &[DataSection]) {
        for s in out {
            assert!(!s.data.is_empty(), "empty section at 0x{:x}", s.addr);
            assert_eq!(s.addr % 32, 0, "section addr 0x{:x} not 32-aligned", s.addr);
            assert_eq!(s.data.len() % 32, 0, "section len {} not multiple of 32", s.data.len());
        }
        // No overlaps (output is sorted by address).
        for w in out.windows(2) {
            assert!(
                w[0].addr + w[0].data.len() as u64 <= w[1].addr,
                "sections overlap: 0x{:x}+{} > 0x{:x}",
                w[0].addr,
                w[0].data.len(),
                w[1].addr
            );
        }
    }

    #[test]
    fn trims_and_expands_to_absolute_32_blocks() {
        // Single non-zero byte at offset 40 in a 64-byte section at a 32-aligned base.
        let mut data = vec![0u8; 64];
        data[40] = 0xAB;
        let out = normalize_rw_data_sections(vec![ds(BASE, data)]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1);
        // Byte 40 lives in the block [BASE+0x20, BASE+0x40).
        assert_eq!(out[0].addr, BASE + 0x20);
        assert_eq!(out[0].data.len(), 32);
        assert_eq!(out[0].data[8], 0xAB);
    }

    #[test]
    fn drops_fully_zero_sections() {
        let out = normalize_rw_data_sections(vec![ds(BASE, vec![0u8; 128])]);
        assert!(out.is_empty());
    }

    #[test]
    fn aligns_section_with_unaligned_base() {
        // Base not 32-aligned; the output must still be 32-aligned.
        let out = normalize_rw_data_sections(vec![ds(BASE + 8, vec![0x11])]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[0].data[8], 0x11); // byte at BASE+8 -> offset 8
    }

    #[test]
    fn merges_sections_that_collide_after_expansion() {
        // Two 1-byte sections 16 bytes apart both expand to [BASE, BASE+0x20).
        let a = ds(BASE, vec![0x01]);
        let b = ds(BASE + 0x10, vec![0x02]);
        let out = normalize_rw_data_sections(vec![a, b]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1, "colliding sections must merge");
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[0].data.len(), 32);
        assert_eq!(out[0].data[0], 0x01); // preserved from A
        assert_eq!(out[0].data[16], 0x02); // preserved from B
    }

    #[test]
    fn does_not_merge_far_sections() {
        let a = ds(BASE, vec![0x01]);
        let b = ds(BASE + 0x1000, vec![0x02]);
        let out = normalize_rw_data_sections(vec![a, b]);
        assert_invariants(&out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[1].addr, BASE + 0x1000);
    }

    #[test]
    fn carves_large_internal_zero_run() {
        // block0 non-zero, then exactly 32 zero blocks (1 KiB), then a non-zero block.
        let mut data = vec![0u8; 32 + 1024 + 32];
        data[0] = 0x01;
        data[1056] = 0x02;
        let out = normalize_rw_data_sections(vec![ds(BASE, data)]);
        assert_invariants(&out);
        assert_eq!(out.len(), 2, "the 1 KiB zero hole must be carved out");
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[0].data.len(), 32);
        assert_eq!(out[0].data[0], 0x01);
        assert_eq!(out[1].addr, BASE + 1056);
        assert_eq!(out[1].data.len(), 32);
        assert_eq!(out[1].data[0], 0x02);
    }

    #[test]
    fn keeps_small_internal_zero_run() {
        // Only 16 zero blocks (512 bytes) between the ends: below the 1 KiB threshold.
        let mut data = vec![0u8; 32 + 512 + 32];
        data[0] = 0x01;
        data[544] = 0x02;
        let out = normalize_rw_data_sections(vec![ds(BASE, data)]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1, "sub-1 KiB zero holes stay initialized");
        assert_eq!(out[0].data.len(), 32 + 512 + 32);
    }

    #[test]
    fn never_expands_below_ram_start() {
        // A section based just above RAM start with a non-32-aligned base: the
        // downward expansion floors to RAM_ADDR and never below it.
        let out = normalize_rw_data_sections(vec![ds(BASE + 0x10, vec![0x22])]);
        assert_invariants(&out);
        assert_eq!(out.len(), 1);
        assert!(out[0].addr >= RAM_ADDR, "must not expand below RAM start");
        assert_eq!(out[0].addr, BASE);
        assert_eq!(out[0].data[0x10], 0x22);

        // A section entirely below RAM start is dropped rather than initialized.
        let out = normalize_rw_data_sections(vec![ds(RAM_ADDR - 0x1000, vec![0x33])]);
        assert!(out.is_empty());
    }

    #[test]
    fn carves_all_large_zero_runs() {
        // One section with 20 large (1 KiB) zero runs separated by non-zero blocks:
        // 21 non-zero blocks, 20 runs. Every run is carved out.
        let run = 1024usize;
        let nz = 32usize;
        let runs = 20usize;
        let mut data = vec![0u8; nz + runs * (run + nz)];
        // mark each non-zero block's first byte
        for i in 0..=runs {
            data[i * (run + nz)] = 1;
        }
        let out = normalize_rw_data_sections(vec![ds(BASE, data)]);
        assert_invariants(&out);
        // 20 carves split the section into 21 single-block pieces.
        assert_eq!(out.len(), 21);
        for piece in &out {
            assert_eq!(piece.data.len(), 32);
        }
    }
}
