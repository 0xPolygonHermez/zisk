//! wasm32 → Zisk transpiler frontend.
//!
//! [`wasm2rom`] is the wasm counterpart of `elf2rom`: it turns a `wasm32-wasip1` binary
//! into a [`ZiskRom`] that the (architecture-neutral) emulator and prover consume unchanged.  The
//! lowering is described in the submodules:
//!
//! * [`module`] — structural scan / validation of the wasm binary.
//! * [`layout`] — the Zisk address-space and register assignment for the wasm machine.
//! * [`emit`] — low-level instruction emitter with symbolic jump fixups.
//! * [`lowering`] — per-function lowering of the integer wasm subset.
//! * [`float`] — f32/f64 operators, lowered onto the RISC-V soft-float library (`float` feature).
//! * [`wasi`] — minimal `wasi_snapshot_preview1` runtime.

pub mod emit;
#[cfg(feature = "float")]
pub mod float;
pub mod layout;
pub mod lowering;
pub mod module;
pub mod wasi;

use std::error::Error;

use emit::{Code, Fixup};
use layout::*;
use module::{parse_module, WasmModule};
use zisk_core::mem::DataSection;
use zisk_core::zisk_rom::DataSection64;
use zisk_core::{
    ZiskInstBuilder, ZiskRom, ARCH_ID_CSR_ADDR, ARCH_ID_ZISK, MAX_ZISK_OS_ROM_ADDR, ROM_ADDR,
    ROM_ENTRY,
};
use zisk_riscv::add_end_and_lib;

/// One past the last program ROM address generated functions may occupy.  With the `float`
/// feature the top of the program ROM window holds the soft-float library (linked by
/// `float::link_float_lib`), so the program must stop below it.
#[cfg(feature = "float")]
const PROGRAM_ROM_END: u64 = zisk_core::FLOAT_LIB_ROM_ADDR;
#[cfg(not(feature = "float"))]
const PROGRAM_ROM_END: u64 = zisk_core::ROM_ADDR_MAX + 1;
use zisk_transpiler_common::elf2rom::normalize_rw_data_sections;

/// Reserve below `WASM_STACK_TOP` for the synthetic entry "frame" that calls `_start`.
const ENTRY_FRAME_RESERVE: i64 = 64;

/// Transpiles a wasm module into a Zisk ROM.
pub fn wasm2rom(bytes: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    let module = parse_module(bytes)?;
    reject_unsupported_layout(&module)?;

    // Resolve the program entry point.
    let command = module.exported_func("_start");
    if module.start_func.is_none() && command.is_none() {
        return Err("wasm: module has neither a start section nor an exported '_start'".into());
    }
    if let Some(index) = command {
        let sig = module.func_sig(index)?;
        if !sig.params.is_empty() || !sig.results.is_empty() {
            return Err("wasm: the exported '_start' must have type () -> ()".into());
        }
    }
    let entry_calls: Vec<u32> = module.start_func.into_iter().chain(command).collect();

    let mut rom: ZiskRom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

    // Reuse the RISC-V BIOS prologue: it installs the end instruction (at ROM_EXIT), the float
    // handler (with the `float` feature), and the initial jump that lands at the first post-BIOS
    // instruction (our entry routine).
    add_end_and_lib(&mut rom);

    // The soft-float library the float handler dispatches to lives in its own reserved ROM/RAM
    // windows (see `zisk_core::mem`), so it never collides with the wasm machine's areas.
    #[cfg(feature = "float")]
    float::link_float_lib(&mut rom)?;

    // Active data segments become ROM-initialized RAM, exactly like an ELF's writable segments:
    // the emulator/prover populate them before the first instruction, so no init code runs.
    // The spec lets segments overlap (later ones win); the merge below assumes they do not, so
    // overlaps are rejected rather than resolved.
    reject_overlapping_segments(&module.data)?;
    let segments = module
        .data
        .iter()
        .map(|seg| DataSection { addr: WASM_MEM_BASE + seg.offset, data: seg.bytes.clone() })
        .collect();
    for section in normalize_rw_data_sections(segments) {
        let data = section.data.chunks(8).map(|c| u64::from_le_bytes(c.try_into().unwrap()));
        rom.rw_data_64.push(DataSection64 { addr: section.addr, data: data.collect() });
    }

    // -- lower every function (imports become WASI stubs) --------------------
    let n_funcs = module.func_count() as usize;
    let n_imports = module.func_import_count as usize;
    let mut codes: Vec<Code> = Vec::with_capacity(n_funcs);
    for i in 0..n_imports {
        codes.push(wasi::build_wasi_stub(&module, i)?);
    }
    for d in 0..module.defined.len() {
        let func_index = (n_imports + d) as u32;
        codes.push(lowering::lower_function(&module, func_index)?);
    }

    // -- lay out functions in the program ROM area ---------------------------
    let mut func_addr = vec![0u64; n_funcs];
    let mut addr = ROM_ADDR;
    for (i, code) in codes.iter().enumerate() {
        func_addr[i] = addr;
        addr += 4 * code.len() as u64;
        if addr > PROGRAM_ROM_END {
            return Err(format!(
                "wasm: program too large ({addr:#x} exceeds the program ROM end \
                 {PROGRAM_ROM_END:#x})"
            )
            .into());
        }
    }

    // -- resolve symbolic jumps/calls and insert into the ROM ----------------
    for (i, code) in codes.iter().enumerate() {
        resolve_and_insert(&mut rom, code, func_addr[i], &func_addr);
    }

    // -- emit the entry routine (init data + call _start + finalize) ---------
    let entry = build_entry_routine(&module, &func_addr, &entry_calls);
    let entry_base = rom.next_init_inst_addr;
    let entry_end = entry_base + 4 * entry.len() as u64;
    if entry_end > MAX_ZISK_OS_ROM_ADDR + 1 {
        return Err(format!(
            "wasm: entry routine too large ({entry_end:#x} exceeds the BIOS ROM end {:#x})",
            MAX_ZISK_OS_ROM_ADDR + 1
        )
        .into());
    }
    resolve_and_insert(&mut rom, &entry, entry_base, &func_addr);
    rom.next_init_inst_addr = entry_end;

    rom.optimize_instruction_lookup()?;

    Ok(rom)
}

/// Fails if the module does not fit the machine's fixed runtime areas (see `layout.rs`):
///
/// * the linear memory's declared initial size must fit the window below `WASM_MEM_LIMIT` (it is
///   what `memory.size` reports and what `memory.grow` extends), and every active data segment
///   must lie inside that initial memory, as instantiation requires;
/// * the globals must fit `WASM_MAX_GLOBALS`;
/// * table 0's declared size must fit `WASM_MAX_TABLE_ENTRIES`, and every active element segment
///   must lie inside it.
///
/// Everything is checked in wasm index/address space, before any Zisk base address is added, so
/// an oversized module cannot initialize one area over the next.
fn reject_unsupported_layout(module: &WasmModule) -> Result<(), Box<dyn Error>> {
    if module.globals.len() as u64 > WASM_MAX_GLOBALS {
        return Err(format!(
            "wasm: {} globals exceed the {} this machine supports",
            module.globals.len(),
            WASM_MAX_GLOBALS
        )
        .into());
    }
    if module.table_initial > WASM_MAX_TABLE_ENTRIES {
        return Err(format!(
            "wasm: table of {} entries exceeds the {} this machine supports",
            module.table_initial, WASM_MAX_TABLE_ENTRIES
        )
        .into());
    }
    for seg in &module.elems {
        let fits = (seg.table_offset as u64)
            .checked_add(seg.func_indices.len() as u64)
            .is_some_and(|end| end <= module.table_initial);
        if !fits {
            return Err(format!(
                "wasm: element segment at table index {} ({} entries) does not fit the table of \
                 {} entries",
                seg.table_offset,
                seg.func_indices.len(),
                module.table_initial
            )
            .into());
        }
    }
    if module.mem_initial_pages > WASM_MAX_PAGES {
        return Err(format!(
            "wasm: initial memory of {} pages exceeds the {} pages this machine supports",
            module.mem_initial_pages, WASM_MAX_PAGES
        )
        .into());
    }
    let mem_bytes = module.mem_initial_pages * WASM_PAGE_SIZE;
    for seg in &module.data {
        let fits =
            seg.offset.checked_add(seg.bytes.len() as u64).is_some_and(|end| end <= mem_bytes);
        if !fits {
            return Err(format!(
                "wasm: data segment at offset {:#x} ({} bytes) does not fit the initial memory \
                 of {} bytes",
                seg.offset,
                seg.bytes.len(),
                mem_bytes
            )
            .into());
        }
    }
    Ok(())
}

/// Fails if any two active data segments cover a common linear-memory byte.
fn reject_overlapping_segments(data: &[module::DataSeg]) -> Result<(), Box<dyn Error>> {
    let mut ranges: Vec<(u64, u64)> = data
        .iter()
        .filter(|seg| !seg.bytes.is_empty())
        .map(|seg| (seg.offset, seg.offset + seg.bytes.len() as u64))
        .collect();
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        let ((prev_start, prev_end), (start, end)) = (pair[0], pair[1]);
        if start < prev_end {
            return Err(format!(
                "wasm: overlapping data segments [{prev_start:#x}, {prev_end:#x}) and \
                 [{start:#x}, {end:#x}) are not supported"
            )
            .into());
        }
    }
    Ok(())
}

/// Resolves the symbolic fixups of a [`Code`] laid out at `base` and inserts every instruction into
/// `rom.insts` at its final address.
fn resolve_and_insert(rom: &mut ZiskRom, code: &Code, base: u64, func_addr: &[u64]) {
    for (j, pending) in code.insts.iter().enumerate() {
        let inst_addr = base + 4 * j as u64;
        let mut zib = pending.zib.clone();
        zib.i.paddr = inst_addr;
        match &pending.fixup {
            Fixup::None => {}
            Fixup::Jump(label) => {
                let target = base + 4 * code.label_target(*label) as u64;
                let off = target as i64 - inst_addr as i64;
                zib.j(off, off);
            }
            Fixup::JumpIfFlag(label) => {
                let target = base + 4 * code.label_target(*label) as u64;
                let off = target as i64 - inst_addr as i64;
                zib.j(off, 4);
            }
            Fixup::JumpIfNotFlag(label) => {
                let target = base + 4 * code.label_target(*label) as u64;
                let off = target as i64 - inst_addr as i64;
                zib.j(4, off);
            }
            Fixup::FuncAddr(index) => {
                zib.src_b("imm", func_addr[*index as usize], false);
            }
            Fixup::LabelAddr(label) => {
                zib.src_b("imm", base + 4 * code.label_target(*label) as u64, false);
            }
        }
        zib.build(rom);
    }
}

/// Builds the BIOS entry routine: initialize the runtime, call each of `entry_calls` in order
/// (the start-section function, then `_start`), then publish output and halt.  Returned as a
/// [`Code`] so its calls and internal jumps resolve through the same fixup machinery as ordinary
/// functions.
fn build_entry_routine(module: &WasmModule, func_addr: &[u64], entry_calls: &[u32]) -> Code {
    let mut code = Code::new();

    // marchid = Zisk (parity with the RISC-V path).
    store_const_to_abs(&mut code, ARCH_ID_CSR_ADDR, ARCH_ID_ZISK);

    // Control cells.
    store_const_to_abs(&mut code, WASM_MEM_PAGES_ADDR, module.mem_initial_pages);
    store_const_to_abs(&mut code, WASM_STDIN_POS_ADDR, 0);
    store_const_to_abs(&mut code, WASM_STDOUT_LEN_ADDR, 0);
    store_const_to_abs(&mut code, WASM_RNG_STATE_ADDR, wasi::RNG_SEED);
    store_const_to_abs(&mut code, WASM_RNG_WARNED_ADDR, 0);

    // Globals.
    for (i, g) in module.globals.iter().enumerate() {
        store_const_to_abs(&mut code, global_addr(i as u32), g.init as u64);
    }

    // Indirect-call table: { zisk_pc, type_index } per entry.
    for seg in &module.elems {
        for (k, &func_index) in seg.func_indices.iter().enumerate() {
            let entry = seg.table_offset + k as u32;
            let canonical = module.canonical_type(func_type_index(module, func_index));
            store_const_to_abs(&mut code, table_entry_addr(entry), func_addr[func_index as usize]);
            store_const_to_abs(&mut code, table_entry_addr(entry) + 8, canonical as u64);
        }
    }

    // Linear-memory bound for the access checks (kept in a register, see `REG_MEM_END`).
    code.load_imm_to_reg(REG_MEM_END, WASM_MEM_BASE + module.mem_initial_pages * WASM_PAGE_SIZE);

    code.load_imm_to_reg(REG_FP, WASM_STACK_TOP);
    for &func_index in entry_calls {
        emit_entry_call(&mut code, func_index);
    }

    wasi::emit_pubout_exit(&mut code);

    code
}

fn emit_entry_call(code: &mut Code, func_index: u32) {
    code.load_imm_to_reg(REG_T2, (WASM_STACK_TOP as i64 - ENTRY_FRAME_RESERVE) as u64); // newFP
                                                                                        // mem[newFP - 16] = caller FP
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("reg", REG_T2, false);
    zib.src_b("reg", REG_FP, false);
    zib.op("copyb").unwrap();
    zib.ind_width(8);
    zib.store("ind", FRAME_CALLER_FP_OFF, false, false);
    zib.j(4, 4);

    code.push_raw(zib, Fixup::None);
    code.mov_reg(REG_FP, REG_T2);
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.store_pc("reg", REG_RA as i64, false);
    zib.j(0, 4);

    // On return: publish output and halt.
    code.push_raw(zib, Fixup::FuncAddr(func_index));
}

fn func_type_index(module: &WasmModule, func_index: u32) -> u32 {
    if func_index < module.func_import_count {
        module.imports[func_index as usize].2
    } else {
        module.defined[(func_index - module.func_import_count) as usize].type_index
    }
}

/// Stores a 64-bit constant to an absolute system address (two instructions).
fn store_const_to_abs(code: &mut Code, addr: u64, value: u64) {
    code.load_imm_to_reg(REG_T0, value);
    code.store_reg_to_abs(addr, REG_T0, 8);
}
