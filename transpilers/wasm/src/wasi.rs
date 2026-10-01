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
//!
//! Guest pointers are untrusted: every range a stub reads or writes is checked against the current
//! linear-memory size (`REG_MEM_END`) before the stub touches anything, and a bad range fails the
//! whole call with `EFAULT`, so a stub can never reach Zisk RAM outside the guest's pages.

use std::error::Error;

use super::emit::{Code, LabelId};
use super::layout::*;
use super::module::{FuncSig, ValKind, WasmModule};
use zisk_core::{ZiskInstBuilder, INPUT_ADDR, OUTPUT_ADDR, ROM_EXIT, UART_ADDR};

/// WASI errno values we use.
const ERRNO_SUCCESS: u64 = 0;
const ERRNO_BADF: u64 = 8;
const ERRNO_FAULT: u64 = 21;

pub const RNG_SEED: u64 = 0x1234_5678_9abc_def0;
const RNG_WARNING: &str = "WARNING: Using insecure random number generator.\n";

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
const R_END: u64 = 25;

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

    code.push_raw(zib, super::emit::Fixup::None);
    // pubout: a = index, b = last c
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("reg", 12, false);
    zib.src_b("lastc", 0, false);
    zib.op("pubout").unwrap();
    zib.store("none", 0, false, false);
    zib.j(4, 4);

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

    code.push_raw(zib, super::emit::Fixup::None);
}

fn emit_console_str(code: &mut Code, text: &str) {
    code.load_imm_to_reg(R_B, UART_ADDR);
    for &byte in text.as_bytes() {
        code.load_imm_to_reg(R_C, byte as u64);
        code.store_reg_to_mem(R_B, 0, R_C, 1);
    }
}

fn body_random_get(code: &mut Code, fault: LabelId, _done: LabelId) {
    // Warn once per run that the stream is not random (clobbers R_B/R_C: done before the
    // arguments are loaded).
    let warned = code.new_label();
    code.load_abs_to_reg(R_C, WASM_RNG_WARNED_ADDR);
    code.cmp_imm_branch("eq", R_C, 0, warned, false);
    code.load_imm_to_reg(R_C, 1);
    code.store_reg_to_abs(WASM_RNG_WARNED_ADDR, R_C, 8);
    emit_console_str(code, RNG_WARNING);
    code.bind(warned);

    code.load_slot_to_reg(R_A, local_offset(0));
    code.load_slot_to_reg(R_B, local_offset(1));
    code.alu_ri("and", R_B, R_B, 0xFFFF_FFFF);
    checked_linear(code, R_A, Len::Reg(R_B), fault);

    code.load_abs_to_reg(R_C, WASM_RNG_STATE_ADDR);
    let head = code.new_label();
    let tail = code.new_label();
    let end = code.new_label();
    code.bind(head);
    code.cmp_imm_branch("ltu", R_B, 8, tail, true);
    emit_splitmix64(code, R_C, R_D, R_E);
    code.store_reg_to_mem(R_A, 0, R_D, 8);
    code.alu_ri("add", R_A, R_A, 8);
    code.alu_ri("sub", R_B, R_B, 8);
    code.jump(head);
    code.bind(tail);
    code.cmp_imm_branch("eq", R_B, 0, end, true);
    emit_splitmix64(code, R_C, R_D, R_E);
    code.store_reg_to_mem(R_A, 0, R_D, 1);
    code.alu_ri("add", R_A, R_A, 1);
    code.alu_ri("sub", R_B, R_B, 1);
    code.jump(tail);
    code.bind(end);
    code.store_reg_to_abs(WASM_RNG_STATE_ADDR, R_C, 8);
    code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
}

fn emit_splitmix64(code: &mut Code, state: u64, out: u64, tmp: u64) {
    code.alu_ri("add", state, state, 0x9E37_79B9_7F4A_7C15u64 as i64);
    code.alu_ri("srl", tmp, state, 30);
    code.alu_rr("xor", out, state, tmp);
    code.alu_ri("mul", out, out, 0xBF58_476D_1CE4_E5B9u64 as i64);
    code.alu_ri("srl", tmp, out, 27);
    code.alu_rr("xor", out, out, tmp);
    code.alu_ri("mul", out, out, 0x94D0_49BB_1331_11EBu64 as i64);
    code.alu_ri("srl", tmp, out, 31);
    code.alu_rr("xor", out, out, tmp);
}

fn wrap_stub<F: FnOnce(&mut Code, LabelId, LabelId)>(body: F) -> Code {
    let mut code = Code::new();
    // Prologue: save the return address.
    code.store_reg_to_slot(FRAME_RET_PC_OFF, REG_RA);
    let fault = code.new_label();
    let done = code.new_label();
    body(&mut code, fault, done);
    code.jump(done);
    code.bind(fault);
    code.load_imm_to_reg(REG_RET, ERRNO_FAULT);
    // Epilogue: restore caller FP, return to saved address. (REG_RET set by the body.)
    code.bind(done);
    code.load_slot_to_reg(REG_T0, FRAME_RET_PC_OFF);
    code.load_slot_to_reg(REG_T2, FRAME_CALLER_FP_OFF);
    code.mov_reg(REG_FP, REG_T2);
    code.ret_reg(REG_T0);
    code
}

fn require_fd(code: &mut Code, allowed: &[u64], done: LabelId) {
    code.load_slot_to_reg(R_A, local_offset(0));
    code.alu_ri("and", R_A, R_A, 0xFFFF_FFFF);
    let ok = code.new_label();
    for &fd in allowed {
        code.cmp_imm_branch("eq", R_A, fd as i64, ok, true);
    }
    code.load_imm_to_reg(REG_RET, ERRNO_BADF);
    code.jump(done);
    code.bind(ok);
}

#[derive(Clone, Copy)]
enum Len {
    Imm(u64),
    Reg(u64),
}

/// Loads wasm linear-memory absolute address `WASM_MEM_BASE + (reg & 0xffffffff)` into `reg`.
fn to_linear(code: &mut Code, reg: u64) {
    code.alu_ri("and", reg, reg, 0xFFFF_FFFF);
    code.alu_ri("add", reg, reg, WASM_MEM_BASE as i64);
}

fn checked_linear(code: &mut Code, reg: u64, len: Len, fault: LabelId) {
    to_linear(code, reg);
    match len {
        Len::Imm(n) => code.alu_ri("add", R_END, reg, n as i64),
        Len::Reg(r) => code.alu_rr("add", R_END, reg, r),
    }
    code.cmp_reg_branch("ltu", REG_MEM_END, R_END, fault, true); // mem_end < end -> fault
}

fn check_iovecs(code: &mut Code, iovs: u64, count: u64, j: u64, fault: LabelId) {
    code.load_imm_to_reg(j, 0);
    let head = code.new_label();
    let end = code.new_label();
    code.bind(head);
    code.cmp_reg_branch("ltu", j, count, end, false); // while j < count
    code.alu_ri("mul", R_F, j, 8);
    code.alu_rr("add", R_F, iovs, R_F); // &iovec[j]
    code.load_mem_to_reg("copyb", R_G, R_F, 0, 4); // buf ptr
    code.load_mem_to_reg("copyb", R_H, R_F, 4, 4); // len
    checked_linear(code, R_G, Len::Reg(R_H), fault);
    code.alu_ri("add", j, j, 1);
    code.jump(head);
    code.bind(end);
}

/// `fd_write(fd, iovs, iovs_len, nwritten) -> errno`.  Writes every iovec byte to the UART console
/// and, for the standard streams, into the public output region; stores the byte count to
/// `*nwritten` and returns success.
fn body_fd_write(code: &mut Code, fault: LabelId, done: LabelId) {
    // locals: 0=fd, 1=iovs, 2=iovs_len, 3=nwritten
    const R_OUT: u64 = 22; // running absolute pointer into the public output region
    const R_NWRITTEN: u64 = 23; // absolute address of *nwritten
    const R_UART: u64 = 24; // UART base address
    require_fd(code, &[1, 2], done); // stdout, stderr
    code.load_slot_to_reg(R_A, local_offset(1)); // iovs ptr
    code.load_slot_to_reg(R_D, local_offset(2)); // iovs_len
    code.alu_ri("and", R_D, R_D, 0xFFFF_FFFF);
    code.alu_ri("mul", R_B, R_D, 8); // iovec array size
    checked_linear(code, R_A, Len::Reg(R_B), fault);
    code.load_slot_to_reg(R_NWRITTEN, local_offset(3));
    checked_linear(code, R_NWRITTEN, Len::Imm(4), fault);
    check_iovecs(code, R_A, R_D, R_B, fault);

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
    code.store_reg_to_mem(R_NWRITTEN, 0, R_C, 4);
    code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
}

/// `fd_read(fd, iovs, iovs_len, nread) -> errno`.  Reads from the Zisk input region (length-prefixed
/// at `INPUT_ADDR`) into the iovecs, advancing a persistent cursor; returns 0 bytes at EOF.
fn body_fd_read(code: &mut Code, fault: LabelId, done: LabelId) {
    // locals: 0=fd, 1=iovs, 2=iovs_len, 3=nread
    const R_NREAD: u64 = 26; // absolute address of *nread
    let jreg = 22u64;
    require_fd(code, &[0], done); // stdin
    code.load_slot_to_reg(R_D, local_offset(1)); // iovs ptr
    code.load_slot_to_reg(R_E, local_offset(2)); // iovs_len
    code.alu_ri("and", R_E, R_E, 0xFFFF_FFFF);
    code.alu_ri("mul", R_F, R_E, 8); // iovec array size
    checked_linear(code, R_D, Len::Reg(R_F), fault);
    code.load_slot_to_reg(R_NREAD, local_offset(3));
    checked_linear(code, R_NREAD, Len::Imm(4), fault);
    check_iovecs(code, R_D, R_E, jreg, fault);

    // Input layout (ziskos convention): an 8-byte length prefix at INPUT_ADDR+8, data at
    // INPUT_ADDR+16. (The emulator writes a zero "free input" word at INPUT_ADDR itself.)
    // R_A = input length, R_B = cursor, R_C = total read
    code.load_abs_to_reg(R_A, INPUT_ADDR + 8); // input length (u64)
    code.load_abs_to_reg(R_B, WASM_STDIN_POS_ADDR); // cursor
    code.load_imm_to_reg(R_C, 0);

    // Single pass over iovecs; stop at EOF.
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
    code.store_reg_to_mem(R_NREAD, 0, R_C, 4);
    code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
}

/// Stores zero into the two u32 cells pointed to by locals 0 and 1 (the `*_sizes_get` shape);
/// both pointers are validated before either cell is written.
fn store_two_zero_u32(code: &mut Code, fault: LabelId, _done: LabelId) {
    code.load_slot_to_reg(R_A, local_offset(0));
    checked_linear(code, R_A, Len::Imm(4), fault);
    code.load_slot_to_reg(R_B, local_offset(1));
    checked_linear(code, R_B, Len::Imm(4), fault);
    code.load_imm_to_reg(R_C, 0);
    code.store_reg_to_mem(R_A, 0, R_C, 4);
    code.store_reg_to_mem(R_B, 0, R_C, 4);
    code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
}

fn expected_signature(name: &str) -> Option<(&'static [ValKind], &'static [ValKind])> {
    use ValKind::{I32, I64};
    const ERRNO: &[ValKind] = &[I32];
    Some(match name {
        "proc_exit" => (&[I32], &[]),
        "fd_write" | "fd_read" => (&[I32, I32, I32, I32], ERRNO),
        "args_sizes_get" | "args_get" | "environ_sizes_get" | "environ_get" | "random_get"
        | "fd_fdstat_get" | "fd_prestat_get" | "fd_filestat_get" => (&[I32, I32], ERRNO),
        "clock_time_get" => (&[I32, I64, I32], ERRNO),
        "fd_prestat_dir_name" => (&[I32, I32, I32], ERRNO),
        "fd_seek" => (&[I32, I64, I32, I32], ERRNO),
        "fd_close" => (&[I32], ERRNO),
        "sched_yield" => (&[], ERRNO),
        _ => return None,
    })
}

fn check_signature(name: &str, declared: &FuncSig) -> Result<(), Box<dyn Error>> {
    let Some((params, results)) = expected_signature(name) else {
        return Ok(());
    };
    if declared.params != params || declared.results != results {
        return Err(format!(
            "wasm: import wasi_snapshot_preview1::{name} is declared as {:?} -> {:?}, expected \
             {params:?} -> {results:?}",
            declared.params, declared.results
        )
        .into());
    }
    Ok(())
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
    check_signature(name, module.func_sig(import_index as u32)?)?;

    let code = match name.as_str() {
        "proc_exit" => {
            // Terminates the program; does not return.
            let mut code = Code::new();
            emit_pubout_exit(&mut code);
            code
        }
        "fd_write" => wrap_stub(body_fd_write),
        "fd_read" => wrap_stub(body_fd_read),
        "args_sizes_get" | "environ_sizes_get" => wrap_stub(store_two_zero_u32),
        "args_get" | "environ_get" => wrap_stub(|code, _fault, _done| {
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        "random_get" => wrap_stub(body_random_get),
        "clock_time_get" => wrap_stub(|code, fault, _done| {
            // Deterministic monotonic clock derived from the step counter is not addressable here;
            // report 0. locals: 0=id, 1=precision, 2=time_ptr
            code.load_slot_to_reg(R_A, local_offset(2));
            checked_linear(code, R_A, Len::Imm(8), fault);
            code.load_imm_to_reg(R_B, 0);
            code.store_reg_to_mem(R_A, 0, R_B, 8);
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        // The runtime queries stdout/stderr/stdin via fd_fdstat_get to set up buffering; report a
        // character device so writes are accepted (and line-buffered). locals: 0=fd, 1=retptr.
        "fd_fdstat_get" => wrap_stub(|code, fault, _done| {
            // fdstat struct (24 bytes): fs_filetype(u8)=2 (character device), then zeros.
            code.load_slot_to_reg(R_A, local_offset(1));
            checked_linear(code, R_A, Len::Imm(24), fault);
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
            wrap_stub(|code, _fault, _done| {
                code.load_imm_to_reg(REG_RET, ERRNO_BADF);
            })
        }
        "sched_yield" => wrap_stub(|code, _fault, _done| {
            code.load_imm_to_reg(REG_RET, ERRNO_SUCCESS);
        }),
        other => {
            return Err(
                format!("wasm: unsupported wasi import 'wasi_snapshot_preview1::{other}'").into()
            );
        }
    };
    Ok(code)
}
