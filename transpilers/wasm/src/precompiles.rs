//! The ZisK precompiles as host imports of the `env` module.
//!
//! A RISC-V guest reaches a precompile through a `csrs` on its syscall port, which `riscv2zisk`
//! folds into a single Zisk instruction whose op *is* the precompile (`keccak`, `sha256`,
//! `secp256k1_add`, ...).  A wasm guest imports the same operation from `env` — e.g.
//! `(import "env" "keccak_f" (func (param i32)))` — and this module lowers each import into a
//! routine that emits that very instruction, so the emulator, the state machines and the prover
//! see exactly what they see for a RISC-V guest.  The `env` module also carries the ZisK C
//! interface (`zkvm_*`, see [`super::zkvm`]), which builds on the library routines; the two name
//! sets are disjoint and [`is_precompile_import`] tells this one apart.
//!
//! The import names and operand layouts mirror the `ziskos` `syscall_*` functions (the canonical
//! contract, see `ziskos/entrypoint/src/syscalls`), with pointers as `i32` wasm addresses.  Three
//! operand shapes exist:
//!
//! * **in place** — one pointer to `words` u64 that are read and overwritten (`keccak_f`,
//!   `poseidon*`, the curve doublings);
//! * **two pointers** — `p1` (read, then overwritten with the result) and `p2` (read): the
//!   hash compressions and the curve/complex additions;
//! * **parameter block** — one pointer to a block of 8-byte slots, each holding either the wasm
//!   address of an operand or a direct value (`arith256*`, `arith384_mod`, `add256`).  The
//!   precompile dereferences the block's addresses itself, so the routine first translates the
//!   guest's block into a scratch block of absolute addresses (`WASM_PRECOMPILE_BLOCK_ADDR`).
//!
//! Every address a precompile would touch is range-checked against the current linear memory and
//! checked for 8-byte alignment *before* the instruction is emitted; a bad one is a trap.  The ops
//! themselves `panic!` the emulator on a misaligned address, so this is what keeps a guest from
//! crashing the host.
//!
//! Not exposed: `blake2b_round` (its chaining protocol encodes a round index and a step gap in the
//! instruction's static argument, which only the `ziskos` driver produces), and the DMA/profiling
//! ports, which are internal to the RISC-V `ziskos` runtime.

use std::error::Error;

use super::emit::{Code, Fixup, LabelId};
use super::layout::*;
use super::module::{FuncSig, ValKind, WasmModule};
use super::wasi::{checked_linear, Len, R_A, R_B, R_C, R_E};
use zisk_core::ZiskInstBuilder;

/// The import module the precompiles live in.
pub const MODULE: &str = "env";

/// Whether import `name` of module `module` is a ZisK precompile.
pub fn is_precompile_import(module: &str, name: &str) -> bool {
    module == MODULE && lookup(name).is_some()
}

/// One slot of a parameter block.
#[derive(Clone, Copy)]
enum Param {
    /// The wasm address of `words` u64 the precompile reads and/or writes.
    Ptr { words: u64 },
    /// A value the precompile consumes as is.
    Direct,
}

/// How a precompile takes its operands.
#[derive(Clone, Copy)]
enum Shape {
    /// `a = 0`, `b = ptr`: `words` u64 at `ptr`, read and overwritten.
    InPlace { words: u64 },
    /// `a = p1`, `b = p2`: `a_words` u64 at `p1` (read, overwritten with the result) and
    /// `b_words` u64 at `p2` (read).
    TwoPtrs { a_words: u64, b_words: u64 },
    /// `a = 0`, `b = block`: a block of 8-byte slots holding the operands' wasm addresses (or
    /// direct values); the precompile follows the addresses.  `returns_c` reports the
    /// instruction's `c` result to the guest (`add256`'s carry-out).
    Block { params: &'static [Param], returns_c: bool },
}

/// A precompile import: its Zisk op and operand shape.
struct Precompile {
    op: &'static str,
    shape: Shape,
}

const P4: Param = Param::Ptr { words: 4 };
const P6: Param = Param::Ptr { words: 6 };

/// The import table.  Names follow `ziskos` (`syscall_<name>`), ops follow `CSR_PRECOMPILED`.
fn lookup(name: &str) -> Option<Precompile> {
    use Shape::*;
    let (op, shape) = match name {
        // -- hashes ---------------------------------------------------------
        "keccak_f" => ("keccak", InPlace { words: 25 }),
        "sha256_f" => ("sha256", TwoPtrs { a_words: 4, b_words: 8 }),
        "blake3f" => ("blake3", TwoPtrs { a_words: 8, b_words: 8 }),
        "blake2sf" => ("blake2s", TwoPtrs { a_words: 8, b_words: 8 }),
        "poseidon1" => ("poseidon1", InPlace { words: 16 }),
        "poseidon2" => ("poseidon2", InPlace { words: 16 }),
        "poseidon2_koalabear" => ("poseidon2_koalabear", InPlace { words: 8 }),
        // -- big-int arithmetic ---------------------------------------------
        // (a, b, c, dl, dh): dl:dh = a * b + c
        "arith256" => ("arith256", Block { params: &[P4, P4, P4, P4, P4], returns_c: false }),
        // (a, b, c, module, d): d = (a * b + c) mod module
        "arith256_mod" => {
            ("arith256_mod", Block { params: &[P4, P4, P4, P4, P4], returns_c: false })
        }
        "arith384_mod" => {
            ("arith384_mod", Block { params: &[P6, P6, P6, P6, P6], returns_c: false })
        }
        // (a, b, cin, c): c = a + b + cin, the carry-out is the result
        "add256" => ("add256", Block { params: &[P4, P4, Param::Direct, P4], returns_c: true }),
        // -- curves (affine points as two coordinates, 4 or 6 words each) --------
        "secp256k1_add" => ("secp256k1_add", TwoPtrs { a_words: 8, b_words: 8 }),
        "secp256k1_dbl" => ("secp256k1_dbl", InPlace { words: 8 }),
        "secp256r1_add" => ("secp256r1_add", TwoPtrs { a_words: 8, b_words: 8 }),
        "secp256r1_dbl" => ("secp256r1_dbl", InPlace { words: 8 }),
        "bn254_curve_add" => ("bn254_curve_add", TwoPtrs { a_words: 8, b_words: 8 }),
        "bn254_curve_dbl" => ("bn254_curve_dbl", InPlace { words: 8 }),
        "bn254_complex_add" => ("bn254_complex_add", TwoPtrs { a_words: 8, b_words: 8 }),
        "bn254_complex_sub" => ("bn254_complex_sub", TwoPtrs { a_words: 8, b_words: 8 }),
        "bn254_complex_mul" => ("bn254_complex_mul", TwoPtrs { a_words: 8, b_words: 8 }),
        "bls12_381_curve_add" => ("bls12_381_curve_add", TwoPtrs { a_words: 12, b_words: 12 }),
        "bls12_381_curve_dbl" => ("bls12_381_curve_dbl", InPlace { words: 12 }),
        "bls12_381_complex_add" => ("bls12_381_complex_add", TwoPtrs { a_words: 12, b_words: 12 }),
        "bls12_381_complex_sub" => ("bls12_381_complex_sub", TwoPtrs { a_words: 12, b_words: 12 }),
        "bls12_381_complex_mul" => ("bls12_381_complex_mul", TwoPtrs { a_words: 12, b_words: 12 }),
        "babyjubjub_add" => ("babyjubjub_add", TwoPtrs { a_words: 8, b_words: 8 }),
        "ed25519_add" => ("ed25519_add", TwoPtrs { a_words: 8, b_words: 8 }),
        _ => return None,
    };
    Some(Precompile { op, shape })
}

/// The wasm signature an import must declare: every pointer is an `i32`, `add256` returns its
/// carry-out as an `i64`.
fn expected_signature(shape: Shape) -> (Vec<ValKind>, Vec<ValKind>) {
    use ValKind::{I32, I64};
    match shape {
        Shape::InPlace { .. } => (vec![I32], vec![]),
        Shape::TwoPtrs { .. } => (vec![I32, I32], vec![]),
        Shape::Block { returns_c, .. } => (vec![I32], if returns_c { vec![I64] } else { vec![] }),
    }
}

/// Builds the routine implementing the `env` import `import_index`, which
/// [`is_precompile_import`] accepted.
pub fn build_stub(module: &WasmModule, import_index: usize) -> Result<Code, Box<dyn Error>> {
    let (_, name, _) = &module.imports[import_index];
    let precompile = lookup(name).expect("build_stub: the import is a precompile");
    let (params, results) = expected_signature(precompile.shape);
    let declared: &FuncSig = module.func_sig(import_index as u32)?;
    if declared.params != params || declared.results != results {
        return Err(format!(
            "wasm: import {MODULE}::{name} is declared as {:?} -> {:?}, expected {params:?} -> \
             {results:?}",
            declared.params, declared.results
        )
        .into());
    }

    let mut code = Code::new();
    code.store_reg_to_slot(FRAME_RET_PC_OFF, REG_RA);
    let fault = code.new_label();
    match precompile.shape {
        Shape::InPlace { words } => {
            // b = absolute address of the state.
            code.load_slot_to_reg(R_A, local_offset(0));
            checked_aligned(&mut code, R_A, words, fault);
            emit_precompile(&mut code, precompile.op, None, R_A, None);
        }
        Shape::TwoPtrs { a_words, b_words } => {
            code.load_slot_to_reg(R_A, local_offset(0));
            checked_aligned(&mut code, R_A, a_words, fault);
            code.load_slot_to_reg(R_B, local_offset(1));
            checked_aligned(&mut code, R_B, b_words, fault);
            emit_precompile(&mut code, precompile.op, Some(R_A), R_B, None);
        }
        Shape::Block { params, returns_c } => {
            assert!(params.len() as u64 <= WASM_PRECOMPILE_BLOCK_SLOTS);
            // The guest's block: `params.len()` 8-byte slots.
            code.load_slot_to_reg(R_A, local_offset(0));
            checked_aligned(&mut code, R_A, params.len() as u64, fault);
            // Translate it slot by slot into the scratch block.
            for (i, param) in params.iter().enumerate() {
                let off = 8 * i as i64;
                code.load_mem_to_reg("copyb", R_B, R_A, off, 8);
                if let Param::Ptr { words } = *param {
                    checked_aligned(&mut code, R_B, words, fault);
                }
                code.store_reg_to_abs(WASM_PRECOMPILE_BLOCK_ADDR + off as u64, R_B, 8);
            }
            code.load_imm_to_reg(R_C, WASM_PRECOMPILE_BLOCK_ADDR);
            emit_precompile(&mut code, precompile.op, None, R_C, returns_c.then_some(REG_RET));
        }
    }
    // Epilogue (shared with the WASI stubs' layout): return; the caller restores FP.
    let done = code.new_label();
    code.jump(done);
    code.bind(fault);
    code.trap();
    code.bind(done);
    code.ret_via_slot(FRAME_RET_PC_OFF);
    Ok(code)
}

/// Turns the wasm address in `reg` into an absolute one and jumps to `fault` unless the `words`
/// u64 from it lie inside linear memory and the address is 8-byte aligned (`WASM_MEM_BASE` is,
/// so the wasm address's alignment is the absolute one's).
fn checked_aligned(code: &mut Code, reg: u64, words: u64, fault: LabelId) {
    checked_linear(code, reg, Len::Imm(8 * words), fault);
    code.alu_ri("and", R_E, reg, 7);
    code.cmp_imm_branch("eq", R_E, 0, fault, false);
}

/// Emits the precompile instruction: `a` is the register holding the first operand's address
/// (or the immediate 0), `b` the register holding the second (or only) one; `c` is stored to
/// `result` when the precompile produces one.  `jmp_offset1` carries the static argument of the
/// precompiles that have one, so it must be 0 for the ones exposed here.
fn emit_precompile(code: &mut Code, op: &str, a: Option<u64>, b: u64, result: Option<u64>) {
    let mut zib = ZiskInstBuilder::new(0);
    match a {
        Some(reg) => zib.src_a("reg", reg, false),
        None => zib.src_a("imm", 0, false),
    }
    zib.src_b("reg", b, false);
    zib.op(op).unwrap();
    match result {
        Some(reg) => zib.store("reg", reg as i64, false, false),
        None => zib.store("none", 0, false, false),
    }
    zib.j(0, 4);
    code.push_raw(zib, Fixup::None);
}
