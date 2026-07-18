//! ELF file extraction utilities for separating ELF parsing from ZiskRom population

use elf::{
    abi::{EM_RISCV, ET_EXEC, PF_R, PF_W, PF_X, PT_LOAD},
    endian::LittleEndian,
    file::Class,
    ElfBytes,
};
use std::{collections::HashMap, error::Error, fs, path::Path};

use zisk_core::is_elf_file;
use zisk_core::mem::{DataSection, RAM_ADDR, RAM_SIZE, ROM_ADDR, ROM_SIZE};

const RAM_START_ADDR: u64 = RAM_ADDR;
const RAM_END_ADDR: u64 = RAM_ADDR + RAM_SIZE;
const ROM_START_ADDR: u64 = ROM_ADDR;
const ROM_END_ADDR: u64 = ROM_ADDR + ROM_SIZE;
/// Minimum alignment required for a loadable segment's virtual address and for the
/// entry point. ZisK decodes the RISC-V C (compressed) extension, whose instructions
/// are 2-byte units, so the minimum instruction alignment is 2 bytes — a 4-byte
/// requirement would reject valid entry points that land on a compressed instruction.
const INSTRUCTION_ALIGN: u64 = 2;

/// All sections that `ZiskRom` cares about in the ELF file, categorized
#[derive(Debug, Default)]
pub struct ElfPayload {
    /// Entry point address from ELF header
    pub entry_point: u64,
    /// Executable `PT_LOAD` segments (`PF_X`)
    pub exec: Vec<DataSection>,
    /// Writable `PT_LOAD` segments (`PF_W`), inside the RAM window
    pub rw: Vec<DataSection>,
    /// Read-only `PT_LOAD` segments (neither `PF_W` nor `PF_X`)
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

    // Parse the ELF as little-endian. The target encoding is fixed to ELFDATA2LSB
    // by the RISC-V target standard; parsing with a fixed `LittleEndian` rejects a
    // big-endian file at parse time rather than adapting to it, as the standard
    // requires ("the loader must not infer endianness from, or adapt endianness
    // to, the file").
    let elf = ElfBytes::<LittleEndian>::minimal_parse(file_data)?;

    // --- Header validation ---
    // Reject the file before constructing any state if a header field is wrong.
    if elf.ehdr.class != Class::ELF64 {
        return Err("ELF is not 64-bit (ELFCLASS64 required)".into());
    }
    if elf.ehdr.e_machine != EM_RISCV {
        return Err(format!(
            "ELF machine is 0x{:x}, expected EM_RISCV (0x{:x})",
            elf.ehdr.e_machine, EM_RISCV
        )
        .into());
    }
    if elf.ehdr.e_type != ET_EXEC {
        return Err(format!(
            "ELF type is 0x{:x}, expected ET_EXEC (statically linked executable)",
            elf.ehdr.e_type
        )
        .into());
    }

    let mut out = ElfPayload { entry_point: elf.ehdr.e_entry, ..Default::default() };

    // Loaded segment ranges [start, end), for cross-segment overlap detection.
    let mut loaded: Vec<(u64, u64)> = Vec::new();

    // --- Loadable image from program headers (PT_LOAD only) ---
    // The image is built exclusively from PT_LOAD segments; section headers are
    // never consulted for loading (they may be absent or stripped).
    let segments = elf.segments().ok_or("ELF has no program header table")?;
    for ph in segments {
        if ph.p_type != PT_LOAD {
            continue;
        }

        let is_exec = (ph.p_flags & PF_X) != 0;
        let is_write = (ph.p_flags & PF_W) != 0;
        let is_read = (ph.p_flags & PF_R) != 0;

        // println!(
        //     "PHDR: type=0x{:x} flags=0x{:x} vaddr=0x{:x} memsz=0x{:x} filesz=0x{:x} PT_EXEC={} PT_WRITE={} PT_READ={}",
        //     ph.p_type, ph.p_flags, ph.p_vaddr, ph.p_memsz, ph.p_filesz, is_exec, is_write, is_read
        // );

        let seg_start = ph.p_vaddr;
        // p_memsz is the full memory footprint, including any zero-filled (.bss)
        // tail beyond p_filesz.
        let seg_end = ph.p_vaddr.checked_add(ph.p_memsz).ok_or_else(|| {
            format!("PT_LOAD segment at 0x{seg_start:x} overflows the address space")
        })?;

        // W^X: a loadable segment must never be both writable and executable.
        if is_write && is_exec {
            return Err(format!(
                "PT_LOAD segment at 0x{seg_start:x} is both writable and executable (W^X violation)"
            )
            .into());
        }

        // ZisK is an execute-and-read zkVM: every executable segment must be
        // PF_X. A PF_X | PF_R (execute-and-read) segment is rejected.
        if is_exec && is_read {
            return Err(format!(
                "executable PT_LOAD segment at 0x{seg_start:x} is readable; \
                 ZisK requires PF_X (execute-and-not-read) executable segments"
            )
            .into());
        }

        // Alignment: p_vaddr must be at least {INSTRUCTION_ALIGN}-byte (instruction) aligned.
        if seg_start % INSTRUCTION_ALIGN != 0 {
            return Err(format!(
                "PT_LOAD segment virtual address 0x{seg_start:x} is not {INSTRUCTION_ALIGN}-byte aligned"
            )
            .into());
        }

        // Addressable space: the whole segment [p_vaddr, p_vaddr + p_memsz) must
        // fit within the ROM window (code / read-only data) or the RAM window
        // (read-write data).
        let in_rom = seg_start >= ROM_START_ADDR && seg_end <= ROM_END_ADDR;
        let in_ram = seg_start >= RAM_START_ADDR && seg_end <= RAM_END_ADDR;
        if !in_rom && !in_ram {
            return Err(format!(
                "PT_LOAD segment 0x{seg_start:x}-0x{seg_end:x} is outside ZisK addressable space \
                 (ROM 0x{ROM_START_ADDR:x}-0x{ROM_END_ADDR:x}, RAM 0x{RAM_START_ADDR:x}-0x{RAM_END_ADDR:x})"
            )
            .into());
        }

        // No overlap: loadable segments must be disjoint in virtual address space.
        for &(o_start, o_end) in &loaded {
            if seg_start < o_end && o_start < seg_end {
                return Err(format!(
                    "overlapping PT_LOAD segments: 0x{o_start:x}-0x{o_end:x} and 0x{seg_start:x}-0x{seg_end:x}"
                )
                .into());
            }
        }
        loaded.push((seg_start, seg_end));

        // A segment's file image can never be larger than its memory image.
        if ph.p_filesz > ph.p_memsz {
            return Err(format!(
                "PT_LOAD segment at 0x{seg_start:x} has p_filesz (0x{:x}) greater than p_memsz (0x{:x})",
                ph.p_filesz, ph.p_memsz
            )
            .into());
        }

        // Materialize the segment: the p_filesz file bytes, then a zero-filled
        // p_memsz > p_filesz tail (.bss and other zero-initialized regions).
        // `p_memsz` is already bounded by the addressable-space check above; convert
        // it with a checked cast anyway so a malformed value can never wrap `usize`.
        let mem_size = usize::try_from(ph.p_memsz).map_err(|_| {
            format!("PT_LOAD segment at 0x{seg_start:x} has p_memsz (0x{:x}) too large for this platform", ph.p_memsz)
        })?;
        let file_bytes = elf.segment_data(&ph)?;

        if is_exec {
            // Executable code must live in the ROM window. Downstream the transpiler
            // and emulator assume instruction addresses are in ROM (jump targets are
            // asserted within ROM_ADDR..=ROM_ADDR_MAX, and PC lookups panic outside
            // the ROM range), so an exec segment merely inside the RAM window would
            // later panic or produce an invalid ROM.
            if !in_rom {
                return Err(format!(
                    "executable PT_LOAD segment at 0x{seg_start:x}-0x{seg_end:x} is outside the ROM window \
                     (0x{ROM_START_ADDR:x}-0x{ROM_END_ADDR:x}); executable code must be placed in ROM."
                )
                .into());
            }

            // Executable segments carry instructions only: they must be fully
            // file-backed (no zero-fill tail) and have an even byte length, since
            // each 16-bit half-word is decoded as (part of) an instruction.
            // Rejecting here turns malformed code into a structured error instead of
            // a later panic in `convert_vector` (which requires a multiple of 2).
            if ph.p_memsz != ph.p_filesz {
                return Err(format!(
                    "executable PT_LOAD segment at 0x{seg_start:x} has a zero-fill tail \
                     (p_memsz 0x{:x} != p_filesz 0x{:x}); code segments must be fully file-backed",
                    ph.p_memsz, ph.p_filesz
                )
                .into());
            }
            if file_bytes.len() % 2 != 0 {
                return Err(format!(
                    "executable PT_LOAD segment at 0x{seg_start:x} has an odd byte length ({}); \
                     RISC-V instructions are 2- or 4-byte units",
                    file_bytes.len()
                )
                .into());
            }
            // Keep exactly the file bytes so no spurious zero words are transpiled.
            out.exec.push(DataSection { addr: seg_start, data: file_bytes.to_vec() });
        } else if is_write {
            // Writable data must live in RAM so it can be initialized there.
            if !in_ram {
                return Err(format!(
                    "writable PT_LOAD segment at 0x{seg_start:x}-0x{seg_end:x} is outside RAM \
                     (0x{RAM_START_ADDR:x}-0x{RAM_END_ADDR:x}); writable segments must be placed in RAM. \
                     Consider adjusting your linker script."
                )
                .into());
            }
            let mut data = file_bytes.to_vec();
            if mem_size > RAM_SIZE.try_into().unwrap() {
                return Err(format!(
                    "writable PT_LOAD segment at 0x{seg_start:x} has p_memsz (0x{:x}) larger than RAM_SIZE (0x{:x})",
                    ph.p_memsz, RAM_SIZE
                )
                .into());
            }
            data.resize(mem_size, 0);
            out.rw.push(DataSection { addr: seg_start, data });
        } else {
            // Read-only data (constants, strings, etc.).
            let mut data = file_bytes.to_vec();
            if mem_size > ROM_SIZE.try_into().unwrap() {
                return Err(format!(
                    "read-only PT_LOAD segment at 0x{seg_start:x} has p_memsz (0x{:x}) larger than ROM_SIZE (0x{:x})",
                    ph.p_memsz, ROM_SIZE
                )
                .into());
            }
            data.resize(mem_size, 0);
            out.ro.push(DataSection { addr: seg_start, data });
        }
    }

    Ok(out)
}

/// Validates the guest entry point against the payload's loaded executable segments.
///
/// ZisK reads `e_entry` (it does not boot from a fixed address), so the entry must
/// be instruction-aligned (2 bytes, since ZisK decodes compressed instructions) and
/// fall inside a loaded executable (`PF_X`) segment. This is applied only to the
/// guest payload — a helper payload (e.g. the embedded float library) whose entry
/// ZisK never jumps to is exempt.
pub fn validate_entry_point(payload: &ElfPayload) -> Result<(), Box<dyn Error>> {
    let entry = payload.entry_point;
    if entry % INSTRUCTION_ALIGN != 0 {
        return Err(format!(
            "entry point 0x{entry:x} is not {INSTRUCTION_ALIGN}-byte (instruction) aligned"
        )
        .into());
    }
    let in_exec = payload.exec.iter().any(|s| {
        let len = u64::try_from(s.data.len()).unwrap_or(u64::MAX);
        let end = s.addr.saturating_add(len);
        entry >= s.addr && entry < end
    });
    if !in_exec {
        return Err(format!(
            "entry point 0x{entry:x} does not fall within any loaded executable (PF_X) segment"
        )
        .into());
    }
    Ok(())
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
    let elf = ElfBytes::<LittleEndian>::minimal_parse(file_data)?;
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

    /// Build a payload with the given entry point and executable segment ranges
    /// (`(addr, byte_len)`), leaving ro/rw empty.
    fn payload_with_exec(entry: u64, exec_ranges: &[(u64, usize)]) -> ElfPayload {
        ElfPayload {
            entry_point: entry,
            exec: exec_ranges
                .iter()
                .map(|&(addr, len)| DataSection { addr, data: vec![0u8; len] })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_entry_point_inside_exec_is_ok() {
        // Entry in the interior of an executable segment.
        let p = payload_with_exec(0x8000_0010, &[(0x8000_0000, 0x100)]);
        assert!(validate_entry_point(&p).is_ok());
    }

    #[test]
    fn test_entry_point_at_segment_start_is_ok() {
        let p = payload_with_exec(0x8000_0000, &[(0x8000_0000, 0x100)]);
        assert!(validate_entry_point(&p).is_ok());
    }

    #[test]
    fn test_entry_point_at_last_instruction_is_ok() {
        // Last addressable 2-byte slot in the segment [start, start+len).
        let p = payload_with_exec(0x8000_00fe, &[(0x8000_0000, 0x100)]);
        assert!(validate_entry_point(&p).is_ok());
    }

    #[test]
    fn test_entry_point_at_segment_end_is_rejected() {
        // start + len is one past the segment; the range is half-open, so this is out.
        let p = payload_with_exec(0x8000_0100, &[(0x8000_0000, 0x100)]);
        assert!(validate_entry_point(&p).is_err());
    }

    #[test]
    fn test_entry_point_misaligned_is_rejected() {
        // Odd address: not 2-byte aligned.
        let p = payload_with_exec(0x8000_0011, &[(0x8000_0000, 0x100)]);
        assert!(validate_entry_point(&p).is_err());
    }

    #[test]
    fn test_entry_point_two_byte_aligned_is_ok() {
        // A compressed-instruction entry (2-byte, not 4-byte aligned) must be accepted.
        let p = payload_with_exec(0x8000_000a, &[(0x8000_0000, 0x100)]);
        assert!(validate_entry_point(&p).is_ok());
    }

    #[test]
    fn test_entry_point_outside_any_exec_segment_is_rejected() {
        // Aligned, but not within any executable segment (e.g. points into RO/RW).
        let p = payload_with_exec(0x9000_0000, &[(0x8000_0000, 0x100)]);
        assert!(validate_entry_point(&p).is_err());
    }

    #[test]
    fn test_entry_point_no_exec_segments_is_rejected() {
        let p = payload_with_exec(0x8000_0000, &[]);
        assert!(validate_entry_point(&p).is_err());
    }
}
