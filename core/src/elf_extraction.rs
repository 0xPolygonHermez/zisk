//! ELF file extraction utilities for separating ELF parsing from ZiskRom population

use elf::{
    abi::{SHF_ALLOC, SHF_EXECINSTR, SHF_WRITE, SHT_NOBITS, SHT_PROGBITS},
    endian::AnyEndian,
    ElfBytes,
};
use std::{collections::HashMap, error::Error, fs, path::Path};

use crate::{is_elf_file, RAM_ADDR, RAM_SIZE};

const RAM_START_ADDR: u64 = RAM_ADDR;
const RAM_END_ADDR: u64 = RAM_ADDR + RAM_SIZE;
const MAX_ELF_SECTION_SIZE: usize = 1024 * 1024 * 1024; // 1 GiB, arbitrary limit to prevent OOM from malformed ELFs

/// Raw bytes of `data` that will live at `addr` once the ROM has booted.
#[derive(Debug, Clone)]
pub struct DataSection {
    pub addr: u64,
    pub data: Vec<u8>,
}

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

    // Process all section headers
    if let Some(shdrs) = elf.section_headers() {
        for sh in shdrs {
            // Must be allocated at runtime
            //
            // Essentially all sections that we need to load into memory when the program is loaded.
            //
            // Example of things this skips are the .debug_* related sections.
            if (sh.sh_flags & SHF_ALLOC as u64) == 0 {
                continue;
            }

            // Spec says ignore if address is 0
            if sh.sh_addr == 0 {
                continue;
            }

            // Handle different section types
            let data = if sh.sh_type == SHT_PROGBITS {
                let (raw, _) = elf.section_data(&sh)?;
                let mut data = raw.to_vec();
                // Word-align by padding with zeros (trimming would remove valid data)
                while data.len() % 4 != 0 {
                    data.push(0);
                }
                data
            } else if sh.sh_type == SHT_NOBITS {
                // BSS sections - uninitialized data, should be zero-filled
                // Create a zero-filled vector of the appropriate size
                let size = sh.sh_size as usize;
                if size > MAX_ELF_SECTION_SIZE {
                    return Err(format!(
                        "ELF section at 0x{:08x} has size {} which exceeds the maximum allowed size of {} bytes.",
                        sh.sh_addr, size, MAX_ELF_SECTION_SIZE
                    ).into());
                }
                // Align size to 4 bytes
                let aligned_size = (size + 3) & !3;
                vec![0u8; aligned_size]
            } else {
                // Skip other section types (notes, etc.)
                continue;
            };

            // Categorize the section based on its flags
            let is_exec = (sh.sh_flags & SHF_EXECINSTR as u64) != 0;
            let is_write = (sh.sh_flags & SHF_WRITE as u64) != 0;
            let in_ram =
                sh.sh_addr >= RAM_START_ADDR && sh.sh_addr + data.len() as u64 <= RAM_END_ADDR;

            if is_exec {
                // Executable code section
                out.exec.push(DataSection { addr: sh.sh_addr, data });
            } else if is_write && in_ram {
                // Read-write data that needs to be copied to RAM
                out.rw.push(DataSection { addr: sh.sh_addr, data });
            } else if is_write {
                // Writable data outside RAM is an error - it cannot be properly initialized
                let section_type = if sh.sh_type == SHT_NOBITS { "BSS" } else { "data" };
                let end_addr = sh.sh_addr + data.len() as u64;
                return Err(format!(
                    "ELF contains writable {} section at 0x{:08x}-0x{:08x} outside RAM bounds (0x{:08x}-0x{:08x}). \
                    Writable sections must be placed in RAM. Consider adjusting your linker script.",
                    section_type, sh.sh_addr, end_addr, RAM_START_ADDR, RAM_END_ADDR
                ).into());
            } else {
                // Read-only data (constants, strings, etc.)
                out.ro.push(DataSection { addr: sh.sh_addr, data });
            }
        }
    }

    Ok(out)
}

/// Helper function to merge adjacent data sections
///
///   Example: If you have:
///  - Section A: addr=0x1000, data=[1,2,3,4] (ends at 0x1004)
///  - Section B: addr=0x1004, data=[5,6,7,8] (starts at 0x1004)
///
///  They merge into:
///  - Single section: addr=0x1000, data=[1,2,3,4,5,6,7,8]
pub fn merge_adjacent_data_sections(sections: &[DataSection]) -> Vec<DataSection> {
    if sections.is_empty() {
        return Vec::new();
    }

    let mut merged = Vec::new();
    let mut sections = sections.to_vec();

    // Sort by address
    sections.sort_by_key(|s| s.addr);

    let mut current = sections[0].clone();

    for section in sections.into_iter().skip(1) {
        // Check if this section is adjacent to the current one
        if current.addr + current.data.len() as u64 == section.addr {
            // Merge by extending the data
            current.data.extend(section.data);
        } else {
            // Not adjacent, save current and start a new one
            merged.push(current);
            current = section;
        }
    }

    merged.push(current);

    merged
}

/// Merge read-only data sections and pad each to a multiple of `align` bytes.
///
/// Unlike `merge_adjacent_data_sections`, this also coalesces sections that the
/// padding would otherwise make overlap (the inter-section gap is zero-filled),
/// so no RO address gets two ROM-init entries — which `rom_data.pil` rejects on an
/// honest run. `align` must be a multiple of 8; section addresses are 8-aligned.
pub fn merge_ro_sections(sections: &[DataSection], align: u64) -> Result<Vec<DataSection>, String> {
    if sections.is_empty() {
        return Ok(Vec::new());
    }
    let mut sections = sections.to_vec();
    sections.sort_by_key(|s| s.addr);

    let mut merged: Vec<DataSection> = Vec::new();
    let mut current = sections[0].clone();
    for next in sections.into_iter().skip(1) {
        let end = current.addr + current.data.len() as u64;
        let padded_end = current.addr + (current.data.len() as u64).next_multiple_of(align);
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
            current.data.extend_from_slice(&next.data);
        } else {
            merged.push(current);
            current = next;
        }
    }
    merged.push(current);

    for s in &mut merged {
        s.data.resize((s.data.len() as u64).next_multiple_of(align) as usize, 0);
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
    fn test_merge_adjacent_empty() {
        let sections = vec![];
        let result = merge_adjacent_data_sections(&sections);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_merge_adjacent_single_section() {
        let sections = vec![DataSection { addr: 0x1000, data: vec![1, 2, 3, 4] }];
        let result = merge_adjacent_data_sections(&sections);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_merge_adjacent_two_adjacent_sections() {
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2, 3, 4] },
            DataSection { addr: 0x1004, data: vec![5, 6, 7, 8] },
        ];
        let result = merge_adjacent_data_sections(&sections);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_merge_adjacent_two_non_adjacent_sections() {
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2, 3, 4] },
            DataSection { addr: 0x2000, data: vec![5, 6, 7, 8] },
        ];
        let result = merge_adjacent_data_sections(&sections);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data, vec![1, 2, 3, 4]);
        assert_eq!(result[1].addr, 0x2000);
        assert_eq!(result[1].data, vec![5, 6, 7, 8]);
    }

    #[test]
    fn test_merge_adjacent_multiple_adjacent_sections() {
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2] },
            DataSection { addr: 0x1002, data: vec![3, 4] },
            DataSection { addr: 0x1004, data: vec![5, 6] },
        ];
        let result = merge_adjacent_data_sections(&sections);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_merge_adjacent_out_of_order_sections() {
        // Sections provided out of order should still merge correctly
        let sections = vec![
            DataSection { addr: 0x1004, data: vec![5, 6, 7, 8] },
            DataSection { addr: 0x1000, data: vec![1, 2, 3, 4] },
        ];
        let result = merge_adjacent_data_sections(&sections);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_merge_adjacent_mixed_adjacent_and_gaps() {
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2] },
            DataSection { addr: 0x1002, data: vec![3, 4] },
            DataSection { addr: 0x2000, data: vec![5, 6] },
            DataSection { addr: 0x2002, data: vec![7, 8] },
            DataSection { addr: 0x3000, data: vec![9, 10] },
        ];
        let result = merge_adjacent_data_sections(&sections);
        assert_eq!(result.len(), 3);
        // First merged group
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data, vec![1, 2, 3, 4]);
        // Second merged group
        assert_eq!(result[1].addr, 0x2000);
        assert_eq!(result[1].data, vec![5, 6, 7, 8]);
        // Third standalone section
        assert_eq!(result[2].addr, 0x3000);
        assert_eq!(result[2].data, vec![9, 10]);
    }

    #[test]
    fn test_merge_adjacent_with_gap_of_one_byte() {
        // Sections with even 1 byte gap should NOT merge
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2, 3, 4] },
            DataSection {
                addr: 0x1005, // Gap of 1 byte
                data: vec![5, 6, 7, 8],
            },
        ];
        let result = merge_adjacent_data_sections(&sections);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data, vec![1, 2, 3, 4]);
        assert_eq!(result[1].addr, 0x1005);
        assert_eq!(result[1].data, vec![5, 6, 7, 8]);
    }

    #[test]
    fn test_merge_adjacent_overlapping_sections() {
        // Overlapping sections should NOT merge (they stay separate)
        // TODO: Should not be possible, but this test explicitly
        // TODO states the behaviour for if it did happen.
        let sections = vec![
            DataSection { addr: 0x1000, data: vec![1, 2, 3, 4] },
            DataSection {
                addr: 0x1003, // Overlaps by 1 byte
                data: vec![5, 6, 7, 8],
            },
        ];
        let result = merge_adjacent_data_sections(&sections);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(result[0].data, vec![1, 2, 3, 4]);
        assert_eq!(result[1].addr, 0x1003);
        assert_eq!(result[1].data, vec![5, 6, 7, 8]);
    }

    #[test]
    fn test_merge_ro_empty() {
        let result = merge_ro_sections(&[], 32).unwrap();
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
        let result = merge_ro_sections(&sections, 32).unwrap();
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
        let result = merge_ro_sections(&sections, 32).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addr, 0x1000);
        assert_eq!(
            result[0].data,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
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
        let result = merge_ro_sections(&sections, 32).unwrap();
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
        let result = merge_ro_sections(&sections, 32).unwrap();
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
            DataSection { addr: 0x1004, data: vec![9, 10, 11, 12] }, // overlaps real data
        ];
        assert!(merge_ro_sections(&sections, 32).is_err());
    }
}
