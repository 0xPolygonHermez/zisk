//! ELF file extraction utilities for separating ELF parsing from ZiskRom population

use elf::{
    abi::{
        PF_R, PF_W, PF_X, SHF_ALLOC, SHF_EXECINSTR, SHF_WRITE, SHT_FINI_ARRAY, SHT_INIT_ARRAY,
        SHT_NOBITS, SHT_PREINIT_ARRAY, SHT_PROGBITS,
    },
    endian::AnyEndian,
    ElfBytes,
};
use std::{collections::HashMap, error::Error, fs, path::Path};

use zisk_core::mem::DataSection;
use zisk_core::{is_elf_file, RAM_ADDR, RAM_SIZE};

const RAM_START_ADDR: u64 = RAM_ADDR;
const RAM_END_ADDR: u64 = RAM_ADDR + RAM_SIZE;
const MAX_ELF_SECTION_SIZE: usize = 1024 * 1024 * 1024; // 1 GiB, arbitrary limit to prevent OOM from malformed ELFs

/// All sections that `ZiskRom` cares about in the ELF file, categorized
#[derive(Debug, Default)]
pub struct ElfPayload {
    /// Entry point address from ELF header
    pub entry_point: u64,
    /// `SHF_ALLOC | SHF_EXECINSTR` - executable code sections
    pub exec: Vec<DataSection>,
    /// `SHF_ALLOC | SHF_WRITE` and inside the RAM window - read-write data
    pub rw: Vec<DataSection>,
    /// `SHF_ALLOC` but not `SHF_WRITE` - read-only data
    pub ro: Vec<DataSection>,
}

/// Extracts the relevant sections from ELF file bytes for `ZiskRom`
pub fn collect_elf_payload_from_bytes(file_data: &[u8]) -> Result<ElfPayload, Box<dyn Error>> {
    // Validate it's an ELF file
    match is_elf_file(file_data) {
        Ok(is_file) => {
            if !is_file {
                return Err("ROM file is not a valid ELF file".into());
            }
        }
        Err(_) => {
            return Err("Error reading ROM file".into());
        }
    }

    // Parse the ELF
    let elf = ElfBytes::<AnyEndian>::minimal_parse(file_data)?;

    let mut out = ElfPayload { entry_point: elf.ehdr.e_entry, ..Default::default() };

    // Process all program headers
    if let Some(phdrs) = elf.segments() {
        for ph in phdrs {
            println!(
                "Program header at 0x{:08x} with size {} bytes (type: {}, flags: 0x{:x}, PF_R: 0x{:x}, PF_W: 0x{:x}, PF_X: 0x{:x})",
                ph.p_vaddr, ph.p_memsz, ph.p_type, ph.p_flags, ph.p_flags & PF_R, ph.p_flags & PF_W, ph.p_flags & PF_X
            );

            if ph.p_type == elf::abi::PT_LOAD {
                let is_exec = (ph.p_flags & PF_X) != 0;
                let is_write = (ph.p_flags & PF_W) != 0;
                let in_ram =
                    ph.p_vaddr >= RAM_START_ADDR && ph.p_vaddr + ph.p_memsz <= RAM_END_ADDR;

                if is_exec {
                    // Executable code section
                    let data = elf.segment_data(&ph)?.to_vec();
                    out.exec.push(DataSection { addr: ph.p_vaddr, data });
                } else if is_write && in_ram {
                    // Read-write data that needs to be copied to RAM
                    let data = elf.segment_data(&ph)?.to_vec();
                    out.rw.push(DataSection { addr: ph.p_vaddr, data });
                } else if is_write {
                    // Writable data outside RAM is an error - it cannot be properly initialized
                    return Err(format!(
                        "ELF contains writable segment at 0x{:08x}-0x{:08x} outside RAM bounds (0x{:08x}-0x{:08x}). \
                        Writable segments must be placed in RAM. Consider adjusting your linker script.",
                        ph.p_vaddr, ph.p_vaddr + ph.p_memsz, RAM_START_ADDR, RAM_END_ADDR
                    ).into());
                } else {
                    // Read-only data (constants, strings, etc.)
                    let data = elf.segment_data(&ph)?.to_vec();
                    out.ro.push(DataSection { addr: ph.p_vaddr, data });
                }
            }
        }
    }

    Ok(out)
}

/// Byte alignment each RO section's length is padded to (the ROM-init row format).
const RO_SECTION_ALIGN: u64 = 32;

/// Merge read-only data sections and pad each to a multiple of `RO_SECTION_ALIGN`.
///
/// This also coalesces sections that the padding would otherwise make overlap (the
/// inter-section gap is zero-filled), so no RO address gets two ROM-init entries —
/// which `rom_data.pil` rejects on an honest run.
pub fn merge_ro_sections(mut sections: Vec<DataSection>) -> Result<Vec<DataSection>, String> {
    if sections.is_empty() {
        return Ok(Vec::new());
    }
    sections.sort_by_key(|s| s.addr);

    let mut iter = sections.into_iter();
    let mut current = iter.next().unwrap();
    let mut merged: Vec<DataSection> = Vec::new();
    for next in iter {
        let end = current.addr + current.data.len() as u64;
        let padded_end =
            current.addr + (current.data.len() as u64).next_multiple_of(RO_SECTION_ALIGN);
        if next.addr < end {
            return Err(format!(
                "overlapping read-only data sections at 0x{:x} and 0x{:x}",
                current.addr, next.addr
            ));
        }
        // Merge when adjacent, or close enough that padding would overlap `next`;
        // resizing to `next`'s offset zero-fills any gap and keeps its address.
        if next.addr == end || next.addr < padded_end {
            current.data.resize((next.addr - current.addr) as usize, 0);
            current.data.extend(next.data);
        } else {
            merged.push(std::mem::replace(&mut current, next));
        }
    }
    merged.push(current);

    for s in &mut merged {
        s.data.resize((s.data.len() as u64).next_multiple_of(RO_SECTION_ALIGN) as usize, 0);
    }
    Ok(merged)
}

/// Get addresses for a list of symbols from an ELF file
pub fn get_symbol_addresses(
    elf_path: &Path,
    symbol_names: &[&str],
) -> Result<HashMap<String, u64>, Box<dyn Error>> {
    let file_data = fs::read(elf_path)?;
    get_symbol_addresses_from_bytes(&file_data, symbol_names)
}

/// Get addresses for a list of symbols from ELF bytes
pub fn get_symbol_addresses_from_bytes(
    file_data: &[u8],
    symbol_names: &[&str],
) -> Result<HashMap<String, u64>, Box<dyn Error>> {
    let elf = ElfBytes::<AnyEndian>::minimal_parse(file_data)?;
    let mut result = HashMap::new();
    let names_set: std::collections::HashSet<&str> = symbol_names.iter().copied().collect();

    if let Some((symtab, strtab)) = elf.symbol_table()? {
        for sym in symtab {
            if let Ok(name) = strtab.get(sym.st_name as usize) {
                if names_set.contains(name) {
                    result.insert(name.to_string(), sym.st_value);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_ro_empty() {
        let result = merge_ro_sections(Vec::new()).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_merge_ro_padding_overlap_is_coalesced() {
        // Two sections within 32 bytes (the real bug): padding the first would
        // overlap the second, so they must coalesce into one with the gap zeroed.
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2, 3, 4, 5, 6, 7, 8] }, // ends 0x1008
            DataSection { addr: 0x1010, data: vec![9, 10, 11, 12, 13, 14, 15, 16] }, // gap of 8
        ];
        let result = merge_ro_sections(sections).unwrap();
        assert_eq!(result.len(), 1, "near sections must be coalesced, not left to overlap");
        assert_eq!(result[0].addr, 0x1000);
        // [sec0][8-byte zero gap][sec1] then padded to a 32-byte multiple.
        assert_eq!(
            result[0].data,
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, // sec0 @ 0x1000
                0, 0, 0, 0, 0, 0, 0, 0, // gap @ 0x1008..0x1010
                9, 10, 11, 12, 13, 14, 15, 16, // sec1 @ 0x1010 (real value preserved)
                0, 0, 0, 0, 0, 0, 0, 0, // padding to 32
            ]
        );
        // The overlapping word keeps sec1's value, NOT the padding zero.
        assert_eq!(result[0].data[(0x1010 - 0x1000) as usize], 9);
    }

    #[test]
    fn test_merge_ro_exact_adjacency_is_merged() {
        // Exact adjacency must still merge (unchanged from the old behaviour).
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2, 3, 4, 5, 6, 7, 8] },
            DataSection { addr: 0x1008, data: vec![9, 10, 11, 12, 13, 14, 15, 16] },
        ];
        let result = merge_ro_sections(sections).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(
            result[0].data,
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0
            ]
        );
    }

    #[test]
    fn test_merge_ro_adjacent_when_first_already_padded() {
        // First section length is already a multiple of 32 (padded_end == end), and
        // the next is exactly adjacent: the `== current_end` branch must still merge.
        let mut first = vec![0u8; 32];
        first[0] = 0xAA;
        let sections = vec![
            DataSection { addr: 0x1000, data: first },
            DataSection { addr: 0x1020, data: vec![0xBB, 0, 0, 0, 0, 0, 0, 0] },
        ];
        let result = merge_ro_sections(sections).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data.len(), 64); // 32 + 8 padded to 32 = 64
        assert_eq!(result[0].data[0], 0xAA);
        assert_eq!(result[0].data[32], 0xBB);
    }

    #[test]
    fn test_merge_ro_far_sections_not_merged() {
        // Sections far enough apart (next starts at/after the padded end) are NOT
        // merged; each is independently padded to a 32-byte multiple. Output is the
        // same as the old merge+pad for non-overlapping ELFs.
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2, 3, 4, 5, 6, 7, 8] },
            DataSection { addr: 0x1020, data: vec![9, 10, 11, 12, 13, 14, 15, 16] },
        ];
        let result = merge_ro_sections(sections).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data.len(), 32);
        assert_eq!(&result[0].data[0..8], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(result[1].addr, 0x1020);
        assert_eq!(result[1].data.len(), 32);
        assert_eq!(&result[1].data[0..8], &[9, 10, 11, 12, 13, 14, 15, 16]);
    }

    #[test]
    fn test_merge_ro_real_overlap_is_rejected() {
        // Two distinct sections claiming the same byte must be rejected, not merged.
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2, 3, 4, 5, 6, 7, 8] }, // ends 0x1008
            DataSection { addr: 0x1004, data: vec![9, 10, 11, 12] },          // overlaps real data
        ];
        assert!(merge_ro_sections(sections).is_err());
    }
}
