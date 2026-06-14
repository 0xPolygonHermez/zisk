//! Minimal `wasi_snapshot_preview1` support, generated as Zisk routines.
//!
//! Each recognized WASI import becomes a callee that follows the same frame convention as a
//! lowered wasm function (see `lowering.rs`): parameters arrive as locals, the result errno is
//! returned in `REG_RET`, and the routine restores the caller frame and returns.  `proc_exit` is
//! special — it terminates the program instead of returning.
//!
//! The surface is deliberately small: enough for a stock `wasm32-wasip1` Rust/C program that reads
//! stdin and writes stdout to run.  stdin is mapped to the Zisk input region and stdout/stderr to
//! the public output region and the UART console.

use std::error::Error;

use super::emit::Code;
use super::layout::*;
use super::module::WasmModule;
use crate::{ZiskInstBuilder, INPUT_ADDR, OUTPUT_ADDR, ROM_EXIT, UART_ADDR};

/// WASI errno values we use.
const ERRNO_SUCCESS: u64 = 0;
const ERRNO_BADF: u64 = 8;

// Scratch registers used by the longer stubs (must avoid REG_FP and the temporaries used by the
// epilogue, REG_T0/REG_T2).
const R_A: u64 = 14;
const R_B: u64 = 15;
const R_C: u64 = 16;
const R_D: u64 = 17;
const R_E: u64 = 18;
const R_F: u64 = 19;
const R_G: u64 = 20;
const R_H: u64 = 21;

/// Emits the public-output publication loop followed by a jump to `ROM_EXIT`.  Terminates the
/// program.  Mirrors the finalization sequence of the RISC-V entry/exit code.
pub fn emit_pubout_exit(code: &mut Code) {
    code.load_imm_to_reg(11, 32); // output length, in 8-byte words
    code.load_imm_to_reg(12, 0); // index
    code.load_imm_to_reg(13, OUTPUT_ADDR); // data pointer
    let head = code.new_label();
    let end = code.new_label();
    code.bind(head);
    code.cmp_reg_branch("eq", 11, 12, end, true); // index == length -> done
    // c = mem[reg13] (load the chunk into last-c)
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("reg", 13, false);
    zib.src_b("ind", 0, false);
    zib.ind_width(8);
    zib.op("copyb").unwrap();
    zib.store("none", 0, false, false);
    zib.j(4, 4);
    zib.build();
    code.push_raw(zib, super::emit::Fixup::None);
    // pubout: a = index, b = last c
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("reg", 12, false);
    zib.src_b("lastc", 0, false);
    zib.op("pubout").unwrap();
    zib.store("none", 0, false, false);
    zib.j(4, 4);
    zib.build();
    code.push_raw(zib, super::emit::Fixup::None);
    code.alu_ri("add", 13, 13, 8);
    code.alu_ri("add", 12, 12, 1);
    code.jump(head);
    code.bind(end);
    // Jump to ROM_EXIT (the final end instruction).
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("imm", 0, false);
    zib.src_b("imm", ROM_EXIT, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.j(0, 0);
    zib.build();
    code.push_raw(zib, super::emit::Fixup::None);
}

/// Wraps a stub body with the standard callee prologue/epilogue.
fn wrap_stub<F: FnOnce(&mut Code)>(body: F) -> Code {
    let mut code = Code::new();
    // Prologue: save the return address.
    code.store_reg_to_slot(FRAME_RET_PC_OFF, REG_RA);
    body(&mut code);
    // Epilogue: restore caller FP, return to saved address. (REG_RET set by the body.)
    code.load_slot_to_reg(REG_T0, FRAME_RET_PC_OFF);
    code.load_slot_to_reg(REG_T2, FRAME_CALLER_FP_OFF);
    code.mov_reg(REG_FP, REG_T2);
    code.ret_reg(REG_T0);
    code
}

/// Loads wasm linear-memory absolute address `WASM_MEM_BASE + (reg & 0xffffffff)` into `reg`.
fn to_linear(code: &mut Code, reg: u64) {
    code.alu_ri("and", reg, reg, 0xFFFF_FFFF);
    code.alu_ri("add", reg, reg, WASM_MEM_BASE as i64);
}

/// `fd_write(fd, iovs, iovs_len, nwritten) -> errno`.  Writes every iovec byte to the UART console
/// and, for the standard streams, into the public output region; stores the byte count to
/// `*nwritten` and returns success.
fn body_fd_write(code: &mut Code) {
    // locals: 0=fd, 1=iovs, 2=iovs_len, 3=nwritten
    const R_OUT: u64 = 22; // running absolute pointer into the public output region
    const R_UART: u64 = 24; // UART base address
    code.load_slot_to_reg(R_A, local_offset(1)); // iovs ptr
    to_linear(code, R_A);
    code.load_slot_to_reg(R_D, local_offset(2)); // iovs_len
    code.alu_ri("and", R_D, R_D, 0xFFFF_FFFF);
    code.load_imm_to_reg(R_B, 0); // j
    code.load_imm_to_reg(R_C, 0); // total bytes
    // output pointer = OUTPUT_ADDR + current stdout length
    code.load_abs_to_reg(R_OUT, WASM_STDOUT_LEN_ADDR);
    code.alu_ri("add", R_OUT, R_OUT, OUTPUT_ADDR as i64);
    // UART base address (a width-1 store here streams a byte to the console).
    code.load_imm_to_reg(R_UART, UART_ADDR);

    let outer = code.new_label();
    let outer_end = code.new_label();
    code.bind(outer);
    code.cmp_reg_branch("ltu", R_B, R_D, outer_end, false); // while j < iovs_len (jump out when !(<))
    // iovec is { u32 buf, u32 len } = 8 bytes.
    code.mov_reg(R_E, R_B);
    code.alu_ri("mul", R_E, R_E, 8);
    code.alu_rr("add", R_E, R_A, R_E); // &iovec[j]
    code.load_mem_to_reg("copyb", R_F, R_E, 0, 4); // buf ptr
    to_linear(code, R_F);
    code.load_mem_to_reg("copyb", R_G, R_E, 4, 4); // len
    code.alu_rr("add", R_C, R_C, R_G); // total += len

    let inner = code.new_label();
    let inner_end = code.new_label();
    code.bind(inner);
    code.cmp_imm_branch("eq", R_G, 0, inner_end, true); // len == 0 -> done
    code.load_mem_to_reg("copyb", R_H, R_F, 0, 1); // byte
    code.store_reg_to_mem(R_UART, 0, R_H, 1); // stream to the console (width-1 store to UART)
    code.store_reg_to_mem(R_OUT, 0, R_H, 1); // mirror into public output
    code.alu_ri("add", R_OUT, R_OUT, 1);
    code.alu_ri("add", R_F, R_F, 1);
    code.alu_ri("sub", R_G, R_G, 1);
    code.jump(inner);
    code.bind(inner_end);

    code.alu_ri("add", R_B, R_B, 1);
    code.jump(outer);
    code.bind(outer_end);

    // persist the new stdout length: R_OUT - OUTPUT_ADDR
    code.alu_ri("sub", R_OUT, R_OUT, OUTPUT_ADDR as i64);
    code.store_reg_to_abs(WASM_STDOUT_LEN_ADDR, R_OUT, 8);
    // *nwritten = total
    code.load_slot_to_reg(R_E, local_offset(3));
    to_linear(code, R_E);
    code.store_reg_to_mem(R_E, 0, R_C, 4);
    code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
}

/// `fd_read(fd, iovs, iovs_len, nread) -> errno`.  Reads from the Zisk input region (length-prefixed
/// at `INPUT_ADDR`) into the iovecs, advancing a persistent cursor; returns 0 bytes at EOF.
fn body_fd_read(code: &mut Code) {
    // locals: 0=fd, 1=iovs, 2=iovs_len, 3=nread
    // Input layout (ziskos convention): an 8-byte length prefix at INPUT_ADDR+8, data at
    // INPUT_ADDR+16. (The emulator writes a zero "free input" word at INPUT_ADDR itself.)
    // R_A = input length, R_B = cursor, R_C = total read
    code.load_abs_to_reg(R_A, INPUT_ADDR + 8); // input length (u64)
    code.load_abs_to_reg(R_B, WASM_STDIN_POS_ADDR); // cursor
    code.load_imm_to_reg(R_C, 0);
    code.load_slot_to_reg(R_D, local_offset(1)); // iovs ptr
    to_linear(code, R_D);
    code.load_slot_to_reg(R_E, local_offset(2)); // iovs_len
    code.alu_ri("and", R_E, R_E, 0xFFFF_FFFF);

    // Single pass over iovecs; stop at EOF.
    let jreg = 22u64;
    code.load_imm_to_reg(jreg, 0);
    let outer = code.new_label();
    let outer_end = code.new_label();
    code.bind(outer);
    code.cmp_reg_branch("ltu", jreg, R_E, outer_end, false);
    // &iovec[j]
    code.mov_reg(R_F, jreg);
    code.alu_ri("mul", R_F, R_F, 8);
    code.alu_rr("add", R_F, R_D, R_F);
    code.load_mem_to_reg("copyb", R_G, R_F, 0, 4); // buf ptr
    to_linear(code, R_G);
    code.load_mem_to_reg("copyb", R_H, R_F, 4, 4); // len

    let inner = code.new_label();
    let inner_end = code.new_label();
    code.bind(inner);
    code.cmp_imm_branch("eq", R_H, 0, inner_end, true); // len == 0
    code.cmp_reg_branch("ltu", R_B, R_A, inner_end, false); // cursor < input_len else EOF
    // byte = mem[INPUT_ADDR + 16 + cursor]
    let breg = 23u64;
    code.mov_reg(breg, R_B);
    code.alu_ri("add", breg, breg, (INPUT_ADDR + 16) as i64);
    let tmp = 24u64;
    code.load_mem_to_reg("copyb", tmp, breg, 0, 1);
    code.store_reg_to_mem(R_G, 0, tmp, 1);
    code.alu_ri("add", R_G, R_G, 1);
    code.alu_ri("add", R_B, R_B, 1);
    code.alu_ri("add", R_C, R_C, 1);
    code.alu_ri("sub", R_H, R_H, 1);
    code.jump(inner);
    code.bind(inner_end);
    code.alu_ri("add", jreg, jreg, 1);
    code.jump(outer);
    code.bind(outer_end);

    // persist cursor; *nread = total
    code.store_reg_to_abs(WASM_STDIN_POS_ADDR, R_B, 8);
    code.load_slot_to_reg(R_F, local_offset(3));
    to_linear(code, R_F);
    code.store_reg_to_mem(R_F, 0, R_C, 4);
    code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
}

/// Stores `value` into `*ptr` where `ptr` is wasm-linear and held in local `local_idx`.
fn store_u32_to_local_ptr(code: &mut Code, local_idx: u32, value: u64) {
    code.load_slot_to_reg(R_A, local_offset(local_idx));
    to_linear(code, R_A);
    code.load_imm_to_reg(R_B, value);
    code.store_reg_to_mem(R_A, 0, R_B, 4);
}

/// Builds the Zisk routine implementing imported function `import_index`.
pub fn build_wasi_stub(module: &WasmModule, import_index: usize) -> Result<Code, Box<dyn Error>> {
    let (mod_name, name, _ty) = &module.imports[import_index];
    if mod_name != "wasi_snapshot_preview1" {
        return Err(format!(
            "wasm: unsupported import module '{mod_name}' (only wasi_snapshot_preview1 is supported)"
        )
        .into());
    }

    let code = match name.as_str() {
        "proc_exit" => {
            // Terminates the program; does not return.
            let mut code = Code::new();
            emit_pubout_exit(&mut code);
            code
        }
        "fd_write" => wrap_stub(body_fd_write),
        "fd_read" => wrap_stub(body_fd_read),
        "args_sizes_get" => wrap_stub(|code| {
            // *argc = 0, *argv_buf_size = 0
            store_u32_to_local_ptr(code, 0, 0);
            store_u32_to_local_ptr(code, 1, 0);
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        "args_get" => wrap_stub(|code| {
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        "environ_sizes_get" => wrap_stub(|code| {
            store_u32_to_local_ptr(code, 0, 0);
            store_u32_to_local_ptr(code, 1, 0);
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        "environ_get" => wrap_stub(|code| {
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        "random_get" => wrap_stub(|code| {
            // Deterministic: zero-fill the requested buffer. locals: 0=buf, 1=len
            code.load_slot_to_reg(R_A, local_offset(0));
            to_linear(code, R_A);
            code.load_slot_to_reg(R_B, local_offset(1));
            code.alu_ri("and", R_B, R_B, 0xFFFF_FFFF);
            code.load_imm_to_reg(R_C, 0);
            let head = code.new_label();
            let end = code.new_label();
            code.bind(head);
            code.cmp_imm_branch("eq", R_B, 0, end, true);
            code.store_reg_to_mem(R_A, 0, R_C, 1);
            code.alu_ri("add", R_A, R_A, 1);
            code.alu_ri("sub", R_B, R_B, 1);
            code.jump(head);
            code.bind(end);
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        "clock_time_get" => wrap_stub(|code| {
            // Deterministic monotonic clock derived from the step counter is not addressable here;
            // report 0. locals: 0=id, 1=precision, 2=time_ptr
            code.load_slot_to_reg(R_A, local_offset(2));
            to_linear(code, R_A);
            code.load_imm_to_reg(R_B, 0);
            code.store_reg_to_mem(R_A, 0, R_B, 8);
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        // The runtime queries stdout/stderr/stdin via fd_fdstat_get to set up buffering; report a
        // character device so writes are accepted (and line-buffered). locals: 0=fd, 1=retptr.
        "fd_fdstat_get" => wrap_stub(|code| {
            // fdstat struct (24 bytes): fs_filetype(u8)=2 (character device), then zeros.
            code.load_slot_to_reg(R_A, local_offset(1));
            to_linear(code, R_A);
            code.load_imm_to_reg(R_B, 2); // CHARACTER_DEVICE
            code.store_reg_to_mem(R_A, 0, R_B, 1);
            code.load_imm_to_reg(R_B, 0);
            code.store_reg_to_mem(R_A, 2, R_B, 2); // fs_flags
            code.store_reg_to_mem(R_A, 8, R_B, 8); // fs_rights_base
            code.store_reg_to_mem(R_A, 16, R_B, 8); // fs_rights_inheriting
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        // File-descriptor probing used by the Rust runtime to enumerate preopens: report EBADF so
        // it concludes there are none.
        "fd_prestat_get" | "fd_prestat_dir_name" | "fd_seek" | "fd_close" | "fd_filestat_get" => {
            wrap_stub(|code| {
                code.load_imm_to_reg(REG_RET, ERRNO_BADF);
            })
        }
        "sched_yield" => wrap_stub(|code| {
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        other => {
            return Err(format!(
                "wasm: unsupported wasi import 'wasi_snapshot_preview1::{other}'"
            )
            .into());
        }
    };
    Ok(code)
}
