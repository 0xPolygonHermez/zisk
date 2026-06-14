//! wasm32 → Zisk transpiler frontend.
//!
//! [`wasm2rom`] is the wasm counterpart of [`crate::elf2rom`]: it turns a `wasm32-wasip1` binary
//! into a [`ZiskRom`] that the (architecture-neutral) emulator and prover consume unchanged.  The
//! lowering is described in the submodules:
//!
//! * [`module`] — structural scan / validation of the wasm binary.
//! * [`layout`] — the Zisk address-space and register assignment for the wasm machine.
//! * [`emit`] — low-level instruction emitter with symbolic jump fixups.
//! * [`lowering`] — per-function lowering of the integer wasm subset.
//! * [`wasi`] — minimal `wasi_snapshot_preview1` runtime.

pub mod emit;
pub mod layout;
pub mod lowering;
pub mod module;
pub mod wasi;

use std::error::Error;

use crate::{
    add_end_and_lib, ZiskInstBuilder, ZiskRom, ARCH_ID_CSR_ADDR, ARCH_ID_ZISK, ROM_ADDR,
    ROM_ADDR_MAX, ROM_ENTRY,
};
use emit::{Code, Fixup};
use layout::*;
use module::{parse_module, WasmModule};

/// Reserve below `WASM_STACK_TOP` for the synthetic entry "frame" that calls `_start`.
const ENTRY_FRAME_RESERVE: i64 = 64;

/// Transpiles a wasm module into a Zisk ROM.
pub fn wasm2rom(bytes: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    let module = parse_module(bytes)?;

    // Resolve the program entry point.
    let start_index = module
        .start_func
        .or_else(|| module.exported_func("_start"))
        .ok_or("wasm: module has neither a start section nor an exported '_start'")?;

    let mut rom: ZiskRom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

    // Reuse the RISC-V BIOS prologue: it installs the end instruction (at ROM_EXIT) and the float
    // handler, and the initial jump that lands at the first post-BIOS instruction (our entry
    // routine).  The float handler is never reached by wasm code.
    add_end_and_lib(&mut rom);

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
        if addr > ROM_ADDR_MAX {
            return Err(format!(
                "wasm: program too large ({addr:#x} exceeds ROM_ADDR_MAX {ROM_ADDR_MAX:#x})"
            )
            .into());
        }
    }

    // -- resolve symbolic jumps/calls and insert into the ROM ----------------
    for (i, code) in codes.iter().enumerate() {
        resolve_and_insert(&mut rom, code, func_addr[i], &func_addr);
    }

    // -- emit the entry routine (init data + call _start + finalize) ---------
    let entry = build_entry_routine(&module, &func_addr, start_index);
    let entry_base = rom.next_init_inst_addr;
    resolve_and_insert(&mut rom, &entry, entry_base, &func_addr);
    rom.next_init_inst_addr = entry_base + 4 * entry.len() as u64;

    crate::elf2rom::optimize_instruction_lookup(&mut rom)?;

    Ok(rom)
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
        }
        rom.insts.insert(inst_addr, zib);
    }
}

/// Builds the BIOS entry routine: initialize the runtime, call `_start`, then publish output and
/// halt.  Returned as a [`Code`] so its `_start` call and internal jumps resolve through the same
/// fixup machinery as ordinary functions.
fn build_entry_routine(module: &WasmModule, func_addr: &[u64], start_index: u32) -> Code {
    let mut code = Code::new();

    // marchid = Zisk (parity with the RISC-V path).
    store_const_to_abs(&mut code, ARCH_ID_CSR_ADDR, ARCH_ID_ZISK);

    // Control cells.
    store_const_to_abs(&mut code, WASM_MEM_PAGES_ADDR, module.mem_initial_pages);
    store_const_to_abs(&mut code, WASM_STDIN_POS_ADDR, 0);
    store_const_to_abs(&mut code, WASM_STDOUT_LEN_ADDR, 0);

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

    // Active data segments -> linear memory.
    for seg in &module.data {
        emit_init_bytes(&mut code, WASM_MEM_BASE + seg.offset, &seg.bytes);
    }

    // Set up the first frame and call _start.
    code.load_imm_to_reg(REG_FP, WASM_STACK_TOP);
    code.load_imm_to_reg(REG_T2, (WASM_STACK_TOP as i64 - ENTRY_FRAME_RESERVE) as u64); // newFP
    // mem[newFP - 16] = caller FP
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("reg", REG_T2, false);
    zib.src_b("reg", REG_FP, false);
    zib.op("copyb").unwrap();
    zib.ind_width(8);
    zib.store("ind", FRAME_CALLER_FP_OFF, false, false);
    zib.j(4, 4);
    zib.build();
    code.push_raw(zib, Fixup::None);
    code.mov_reg(REG_FP, REG_T2);
    // CALL _start
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", 0, false); // patched to _start address
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.store_pc("reg", REG_RA as i64, false);
    zib.j(0, 4);
    zib.build();
    code.push_raw(zib, Fixup::FuncAddr(start_index));

    // On return: publish output and halt.
    wasi::emit_pubout_exit(&mut code);

    code
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

/// Stores `bytes` into linear memory starting at absolute `addr`, 8 bytes at a time.
fn emit_init_bytes(code: &mut Code, addr: u64, bytes: &[u8]) {
    let mut off = 0usize;
    while off + 8 <= bytes.len() {
        let v = u64::from_le_bytes(bytes[off..off + 8].try_into().unwrap());
        code.load_imm_to_reg(REG_T0, v);
        code.load_imm_to_reg(REG_T1, addr + off as u64);
        code.store_reg_to_mem(REG_T1, 0, REG_T0, 8);
        off += 8;
    }
    // Tail bytes, one at a time.
    while off < bytes.len() {
        code.load_imm_to_reg(REG_T0, bytes[off] as u64);
        code.load_imm_to_reg(REG_T1, addr + off as u64);
        code.store_reg_to_mem(REG_T1, 0, REG_T0, 1);
        off += 1;
    }
}
