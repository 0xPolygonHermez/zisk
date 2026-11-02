//! The EF zkVM accelerator C interface (`zkvm_accelerators.h`) as wasm imports.
//!
//! A guest compiled to wasm against the standard header (C with clang, TinyGo through cgo, Rust
//! through an `extern "C"` block) leaves each accelerator undefined, and the linker turns it into
//! an import from module `env`, with the header's C signature lowered by the wasm32 C ABI:
//! pointers, `size_t`, `uint32_t`, `uint8_t` and the `zkvm_status` result are all `i32`.  For
//! example `zkvm_keccak256(const uint8_t*, size_t, zkvm_keccak256_hash*)` becomes
//! `env.zkvm_keccak256: (i32, i32, i32) -> i32`.
//!
//! Each such import becomes a stub with the frame convention of a WASI stub (see `wasi.rs`) that
//! calls the ZisK library routine a RISC-V guest reaches through the matching zkvmcall:
//!
//! 1. every argument is loaded into `a0..a6` (`r10..r16`), zero-extended from 32 bits, and every
//!    pointer is rebased onto `WASM_MEM_BASE`;
//! 2. every range a pointer covers (a fixed size, or a length or a count taken from another
//!    argument) is checked against the current linear memory, and an out-of-range one traps, like
//!    an out-of-bounds load: the routines read and write absolute addresses, so an unchecked
//!    pointer could reach the wasm machine's own state;
//! 3. a call into the routine, which follows the RISC-V calling convention: it preserves `r2`
//!    and `r9` (`REG_MEM_END`), and returns the status in `r10` (`REG_RET`).  `r2` is both the
//!    wasm frame pointer and the RISC-V `sp` the routines push return addresses on, so for the
//!    call it points below the stub's frame, at stack no frame is using.
//!
//! The routines need no alignment: they accept unaligned buffers, as the C and Rust bindings may
//! pass any byte pointer.  `wasm2rom` merges the library into the ROM (see `lib.rs`) only when a
//! module imports one of these functions.

use std::error::Error;

use super::emit::{Code, Fixup};
use super::layout::*;
use super::lowering::callee_frame_size;
use super::module::{ValKind, WasmModule};
use super::wasi::{emit_console_str, wrap_stub};
use zisk_core::ZiskInstBuilder;

/// The import module the wasm32 C ABI puts undefined functions in.
pub const IMPORT_MODULE: &str = "env";

/// How many bytes a pointer argument covers.
#[derive(Clone, Copy)]
enum Size {
    /// A fixed-size type, e.g. `zkvm_bytes_32`.
    Bytes(u64),
    /// As many bytes as the `size_t` argument at this index.
    LenArg(usize),
    /// An array of `elem`-byte structs, as many as the `size_t` argument at `count`.
    Array { elem: u64, count: usize },
}

/// One argument of an accelerator, as the header declares it.
#[derive(Clone, Copy)]
enum Arg {
    Ptr(Size),
    /// `size_t` or `uint32_t`.
    U32,
    /// `uint8_t`: the wasm32 C ABI passes it in an `i32` whose upper bits are unspecified.
    U8,
}

use Arg::{Ptr, U32, U8};
use Size::{Array, Bytes, LenArg};

/// An accelerator of `zkvm_accelerators.h` and the ZisK library routine that implements it.
struct Accelerator {
    name: &'static str,
    routine: &'static str,
    args: &'static [Arg],
}

const fn acc(name: &'static str, routine: &'static str, args: &'static [Arg]) -> Accelerator {
    Accelerator { name, routine, args }
}

/// Every function of `zkvm_accelerators.h`, in header order.  The routine is the target of the
/// zkvmcall of the same name (`zisk_definitions::ZKVMCALLS`), except for `zkvm_keccak_f1600`,
/// which RISC-V guests inline as the `keccak` precompile and so has no zkvmcall.
const ACCELERATORS: &[Accelerator] = &[
    acc("zkvm_keccak256", "ziskasm_zkvm_keccak256", &[Ptr(LenArg(1)), U32, Ptr(Bytes(32))]),
    acc("zkvm_keccak_f1600", "ziskasm_zkvm_keccak_f1600", &[Ptr(Bytes(200))]),
    acc("zkvm_sha256", "ziskasm_zkvm_sha256", &[Ptr(LenArg(1)), U32, Ptr(Bytes(32))]),
    acc("zkvm_ripemd160", "ziskasm_zkvm_ripemd160", &[Ptr(LenArg(1)), U32, Ptr(Bytes(32))]),
    acc(
        "zkvm_secp256k1_verify",
        "ziskasm_zkvm_secp256k1_verify",
        &[Ptr(Bytes(32)), Ptr(Bytes(64)), Ptr(Bytes(64)), Ptr(Bytes(1))],
    ),
    acc(
        "zkvm_secp256k1_ecrecover",
        "ziskasm_zkvm_secp256k1_ecrecover",
        &[Ptr(Bytes(32)), Ptr(Bytes(64)), U8, Ptr(Bytes(64))],
    ),
    acc(
        "zkvm_secp256r1_verify",
        "ziskasm_zkvm_secp256r1_verify",
        &[Ptr(Bytes(32)), Ptr(Bytes(64)), Ptr(Bytes(64)), Ptr(Bytes(1))],
    ),
    acc(
        "zkvm_modexp",
        "ziskasm_zkvm_modexp",
        &[Ptr(LenArg(1)), U32, Ptr(LenArg(3)), U32, Ptr(LenArg(5)), U32, Ptr(LenArg(5))],
    ),
    acc(
        "zkvm_bn254_g1_add",
        "ziskasm_zkvm_bn254_g1_add",
        &[Ptr(Bytes(64)), Ptr(Bytes(64)), Ptr(Bytes(64))],
    ),
    acc(
        "zkvm_bn254_g1_mul",
        "ziskasm_zkvm_bn254_g1_mul",
        &[Ptr(Bytes(64)), Ptr(Bytes(32)), Ptr(Bytes(64))],
    ),
    acc(
        "zkvm_bn254_pairing",
        "ziskasm_zkvm_bn254_pairing",
        &[Ptr(Array { elem: 192, count: 1 }), U32, Ptr(Bytes(1))],
    ),
    acc(
        "zkvm_blake2f",
        "ziskasm_zkvm_blake2f",
        &[U32, Ptr(Bytes(64)), Ptr(Bytes(128)), Ptr(Bytes(16)), U8],
    ),
    acc(
        "zkvm_kzg_point_eval",
        "ziskasm_zkvm_kzg_point_eval",
        &[Ptr(Bytes(48)), Ptr(Bytes(32)), Ptr(Bytes(32)), Ptr(Bytes(48)), Ptr(Bytes(1))],
    ),
    acc(
        "zkvm_bls12_g1_add",
        "ziskasm_zkvm_bls12_g1_add",
        &[Ptr(Bytes(96)), Ptr(Bytes(96)), Ptr(Bytes(96))],
    ),
    acc(
        "zkvm_bls12_g1_msm",
        "ziskasm_zkvm_bls12_g1_msm",
        &[Ptr(Array { elem: 128, count: 1 }), U32, Ptr(Bytes(96))],
    ),
    acc(
        "zkvm_bls12_g2_add",
        "ziskasm_zkvm_bls12_g2_add",
        &[Ptr(Bytes(192)), Ptr(Bytes(192)), Ptr(Bytes(192))],
    ),
    acc(
        "zkvm_bls12_g2_msm",
        "ziskasm_zkvm_bls12_g2_msm",
        &[Ptr(Array { elem: 224, count: 1 }), U32, Ptr(Bytes(192))],
    ),
    acc(
        "zkvm_bls12_pairing",
        "ziskasm_zkvm_bls12_pairing",
        &[Ptr(Array { elem: 288, count: 1 }), U32, Ptr(Bytes(1))],
    ),
    acc(
        "zkvm_bls12_map_fp_to_g1",
        "ziskasm_zkvm_bls12_map_fp_to_g1",
        &[Ptr(Bytes(48)), Ptr(Bytes(96))],
    ),
    acc(
        "zkvm_bls12_map_fp2_to_g2",
        "ziskasm_zkvm_bls12_map_fp2_to_g2",
        &[Ptr(Bytes(96)), Ptr(Bytes(192))],
    ),
];

/// Stack the library routines may use below the stub's frame: 512 nested calls, far more than
/// any routine makes. A call that would take it below `WASM_STACK_LIMIT` traps instead.
const LIBRARY_STACK_BYTES: i64 = 4096;

/// Holds the byte length of the range being checked.
const R_LEN: u64 = 24;
/// Holds the end of the range being checked.
const R_END: u64 = 25;

fn accelerator(name: &str) -> Option<&'static Accelerator> {
    ACCELERATORS.iter().find(|a| a.name == name)
}

/// Whether import `(module, name)` is an accelerator this module implements.
pub fn is_accelerator_import(module: &str, name: &str) -> bool {
    module == IMPORT_MODULE && accelerator(name).is_some()
}

/// The ZisK library routine behind accelerator `name`.
pub fn routine(name: &str) -> Option<&'static str> {
    accelerator(name).map(|a| a.routine)
}

/// The zkvmcall ID of accelerator `name`, if it has one (all but `zkvm_keccak_f1600`).
pub fn zkvmcall_id(name: &str) -> Option<u16> {
    zisk_definitions::ZKVMCALLS.iter().find(|c| c.name == name).map(|c| c.id)
}

/// Builds the stub for accelerator import `import_index`, calling the routine at `entry`.
pub fn build_accelerator_stub(
    module: &WasmModule,
    import_index: usize,
    entry: u64,
) -> Result<Code, Box<dyn Error>> {
    let (_, name, _) = &module.imports[import_index];
    let acc = accelerator(name).ok_or_else(|| format!("wasm: {name} is not an accelerator"))?;
    let declared = module.func_sig(import_index as u32)?;
    let params = vec![ValKind::I32; acc.args.len()];
    if declared.params != params || declared.results != [ValKind::I32] {
        return Err(format!(
            "wasm: import {IMPORT_MODULE}::{name} is declared as {:?} -> {:?}, expected \
             {params:?} -> [I32] (zkvm_accelerators.h compiled for wasm32)",
            declared.params, declared.results
        )
        .into());
    }

    let frame = callee_frame_size(module, import_index as u32)?;
    Ok(wrap_stub(|code, _fault, _done| {
        let arg_reg = |k: usize| 10 + k as u64;
        for (k, arg) in acc.args.iter().enumerate() {
            code.load_slot_to_reg(arg_reg(k), local_offset(k as u32));
            let mask = if matches!(arg, U8) { 0xFF } else { 0xFFFF_FFFF };
            code.alu_ri("and", arg_reg(k), arg_reg(k), mask);
        }

        let out_of_bounds = code.new_label();
        for (k, arg) in acc.args.iter().enumerate() {
            let Ptr(size) = *arg else { continue };
            let ptr = arg_reg(k);
            code.alu_ri("add", ptr, ptr, WASM_MEM_BASE as i64);
            let ok = code.new_label();
            match size {
                Bytes(n) => code.alu_ri("add", R_END, ptr, n as i64),
                LenArg(len) => {
                    // An empty range is never touched, so it may point anywhere (EIP-198 lets a
                    // zero-length modexp modulus come with any output pointer).
                    code.cmp_imm_branch("eq", arg_reg(len), 0, ok, true);
                    code.alu_rr("add", R_END, ptr, arg_reg(len));
                }
                Array { elem, count } => {
                    code.cmp_imm_branch("eq", arg_reg(count), 0, ok, true);
                    code.alu_ri("mul", R_LEN, arg_reg(count), elem as i64);
                    code.alu_rr("add", R_END, ptr, R_LEN);
                }
            }
            code.cmp_reg_branch("ltu", REG_MEM_END, R_END, out_of_bounds, true);
            code.bind(ok);
        }

        // The routines keep return addresses on a stack at `sp` (`push r1` / `pop r1`), which the
        // caller provides, and `sp` is `r2`, the wasm frame pointer: point it at the free stack
        // below this stub's frame for the call, as the next callee's frame would sit, and put it
        // back after. Each nested library call pushes 8 bytes.
        code.alu_ri("add", REG_FP, REG_FP, -frame);
        code.alu_ri("add", R_END, REG_FP, -LIBRARY_STACK_BYTES);
        let stack_ok = code.new_label();
        code.cmp_imm_branch("ltu", R_END, WASM_STACK_LIMIT as i64, stack_ok, false);
        code.alu_ri("add", REG_FP, REG_FP, frame);
        emit_console_str(code, &format!("wasm: {name}: call stack exhausted\n"));
        code.trap();
        code.bind(stack_ok);

        // Call the routine (pc <- entry, ra <- the next instruction), like a RISC-V `jal ra`.
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", entry, false);
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.store_pc("reg", REG_RA as i64, false);
        zib.j(0, 4);
        zib.set_meta_rd(REG_RA as u8);
        code.push_raw(zib, Fixup::None);
        // The routine restored `sp` to where it was set above (it is callee-saved).
        code.alu_ri("add", REG_FP, REG_FP, frame);
        // The status is an i32, which the wasm machine keeps sign-extended.
        code.signextend_w_reg(REG_RET);

        let done = code.new_label();
        code.jump(done);
        code.bind(out_of_bounds);
        emit_console_str(code, &format!("wasm: {name}: a pointer argument is out of bounds\n"));
        code.trap();
        code.bind(done);
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every accelerator but `zkvm_keccak_f1600` is a zkvmcall whose library routine is the one
    /// named here, so both transpilers run the same code for it.
    #[test]
    fn routines_match_the_zkvmcall_table() {
        for acc in ACCELERATORS {
            match zisk_definitions::ZKVMCALLS.iter().find(|c| c.name == acc.name) {
                Some(call) => assert_eq!(call.target, acc.routine, "{}", acc.name),
                None => assert_eq!(acc.name, "zkvm_keccak_f1600"),
            }
        }
    }

    /// A length or count refers to a `U32` argument, and every routine takes at most the eight
    /// argument registers `a0..a7`.
    #[test]
    fn signatures_are_consistent() {
        for acc in ACCELERATORS {
            assert!(acc.args.len() <= 8, "{}", acc.name);
            for arg in acc.args {
                if let Ptr(LenArg(i) | Array { count: i, .. }) = arg {
                    assert!(matches!(acc.args[*i], U32), "{}: argument {i}", acc.name);
                }
            }
        }
    }

    /// The library assembles and defines every routine.
    #[test]
    fn library_defines_every_routine() {
        let library = ziskasm::assemble_zisk_library().unwrap();
        for acc in ACCELERATORS {
            assert!(library.symbols.contains_key(acc.routine), "{}", acc.routine);
        }
    }
}
