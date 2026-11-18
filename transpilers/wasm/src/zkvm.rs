//! The ZisK guest C interface (`ziskasm/lang/c/include`) as wasm imports.
//!
//! A guest compiled to wasm against those headers (C with clang, TinyGo through cgo, Rust
//! through an `extern "C"` block) leaves each function undefined, and the linker turns it into
//! an import from module `env`, with the header's C signature lowered by the wasm32 C ABI:
//! pointers, `size_t`, `uint32_t`, `uint8_t` and the `zkvm_status` result are `i32`, and
//! `uint64_t` is `i64`.  For example `zkvm_keccak256(const uint8_t*, size_t, zkvm_keccak256_hash*)`
//! becomes `env.zkvm_keccak256: (i32, i32, i32) -> i32`.  The headers covered:
//!
//! * `zkvm_accelerators.h`, `zkvm_u256.h` and `zkvm_random.h` (the EF standard), and `write_log`
//!   of `zkvm_io.h`.  The guest reads its input and writes its public output through WASI
//!   (stdin and stdout), not `read_input` / `write_output`: `read_input` returns a pointer into
//!   the input region, which the guest cannot address;
//! * ZisK's own `zkvm_u256_le.h`, `zkvm_zisklib.h` and `zkvm_fcall.h`.
//!
//! Most imports become a stub with the frame convention of a WASI stub (see `wasi.rs`) that
//! calls the ZisK library routine a RISC-V guest reaches through the matching zkvmcall:
//!
//! 1. every argument is loaded into `a0..a7` (`r10..r17`), a 32-bit one zero-extended, and every
//!    pointer is rebased onto `WASM_MEM_BASE`;
//! 2. every range a pointer covers (a fixed size, or a length or a count taken from another
//!    argument) is checked against the current linear memory, and an out-of-range one traps, like
//!    an out-of-bounds load: the routines read and write absolute addresses, so an unchecked
//!    pointer could reach the wasm machine's own state.  A pointer to `uint64_t` limbs must also
//!    be 8-byte aligned, as the C type guarantees, since those routines hand it to precompiles;
//! 3. a call into the routine, which follows the RISC-V calling convention: it preserves `r2`
//!    and `r9` (`REG_MEM_END`), and returns its result in `r10` (`REG_RET`); the stub then
//!    recomputes the `REG_MEM_LIMITS` registers, which it may clobber.  `r2` is both the
//!    wasm frame pointer and the RISC-V `sp` the routines push return addresses on, so for the
//!    call it points below the stub's frame, at stack no frame is using.  A routine that RISC-V
//!    guests expand inline (most of the u256 functions) sets no status, so the stub returns
//!    `ZKVM_EOK` for it, as the RISC-V thunk does.
//!
//! `write_log` and the fcalls have no routine: their stubs issue the instructions themselves.
//! `wasm2rom` merges the library into the ROM (see `lib.rs`) only when a module calls a routine.

use std::error::Error;

use super::emit::{Code, Fixup, LabelId};
use super::layout::*;
use super::lowering::callee_frame_size;
use super::module::{ValKind, WasmModule};
use super::wasi::{emit_console_str, wrap_stub};
use zisk_core::{ZiskInstBuilder, FCALL_PARAMS_MAX_SIZE, INPUT_ADDR};

/// The import module the wasm32 C ABI puts undefined functions in.
pub const IMPORT_MODULE: &str = "env";

/// How many bytes a pointer argument covers.
#[derive(Clone, Copy)]
enum Size {
    /// A fixed-size type, e.g. `zkvm_bytes_32`.
    Bytes(u64),
    /// As many bytes as the length argument at this index.
    LenArg(usize),
    /// An array of `elem`-byte items, as many as the argument at `count`.
    Array { elem: u64, count: usize },
    /// The result of `zkvm_zisklib_modexp_u64_c`: `max(4, ceil(m / 4) * 4)` limbs, where `m` is
    /// the modulus length (in limbs) at this index.
    ModexpResult(usize),
}

/// One argument, as the header declares it.
#[derive(Clone, Copy)]
enum Arg {
    /// A byte pointer: any alignment.
    Ptr(Size),
    /// A pointer to `uint64_t` limbs (or a struct of them): 8-byte aligned.
    Ptr8(Size),
    /// `size_t` or `uint32_t`.
    U32,
    /// `uint8_t`: the wasm32 C ABI passes it in an `i32` whose upper bits are unspecified.
    U8,
    /// `uint64_t`: an `i64`.
    U64,
}

/// The C result type.
#[derive(Clone, Copy, PartialEq)]
enum Ret {
    /// `zkvm_status`, `size_t` or `uint32_t`.
    I32,
    /// `uint64_t`.
    I64,
    Void,
}

/// What the stub does.
#[derive(Clone, Copy)]
enum Kind {
    /// Calls this ZisK library routine.
    Routine(&'static str),
    /// `write_log`: one step that reads the text's pointer and length (see `opc_write_log`).
    WriteLog,
    /// `zkvm_fcall`: passes the parameters and issues the fcall.
    Fcall,
    /// `zkvm_fcall_get`: reads the next result word.
    FcallGet,
    /// `zkvm_fcall_read`: reads the next `n` result words into a buffer.
    FcallRead,
}

use Arg::{Ptr, Ptr8, U32, U64, U8};
use Size::{Array, Bytes, LenArg, ModexpResult};

/// A function of the C interface and how its stub implements it.
struct Function {
    name: &'static str,
    kind: Kind,
    args: &'static [Arg],
    ret: Ret,
}

/// A function the library routine `ziskasm_<name>` implements (the EF headers' convention).
macro_rules! ef {
    ($name:expr, $args:expr) => {
        ef!($name, $args, Ret::I32)
    };
    ($name:expr, $args:expr, $ret:expr) => {
        Function {
            name: $name,
            kind: Kind::Routine(concat!("ziskasm_", $name)),
            args: $args,
            ret: $ret,
        }
    };
}

/// A `zkvm_zisklib_<name>` function, implemented by the library routine `zisklib_<name>`.
macro_rules! zl {
    ($name:expr, $args:expr, $ret:expr) => {
        Function {
            name: concat!("zkvm_zisklib_", $name),
            kind: Kind::Routine(concat!("zisklib_", $name)),
            args: $args,
            ret: $ret,
        }
    };
}

/// The 27 functions of the U256 ABI, in header order, as `$p<name>` (`zkvm_u256_` or
/// `zkvm_u256_le_`), with the pointer argument `$ptr` (`Ptr` or `Ptr8`).
macro_rules! u256 {
    ($p:literal, $ptr:ident) => {
        [
            ef!(concat!($p, "div"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "mod"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(
                concat!($p, "divmod"),
                &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]
            ),
            ef!(concat!($p, "sdiv"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "smod"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(
                concat!($p, "sdivmod"),
                &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]
            ),
            ef!(concat!($p, "add"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "sub"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "mul"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(
                concat!($p, "addmod"),
                &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]
            ),
            ef!(
                concat!($p, "mulmod"),
                &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]
            ),
            ef!(concat!($p, "exp"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "lt"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "gt"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "slt"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "sgt"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "eq"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "iszero"), &[$ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "and"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "or"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "xor"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "not"), &[$ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "byte"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "shl"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "shr"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "sar"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
            ef!(concat!($p, "signextend"), &[$ptr(Bytes(32)), $ptr(Bytes(32)), $ptr(Bytes(32))]),
        ]
    };
}

/// `zkvm_accelerators.h`, in header order.  The routine is the target of the zkvmcall of the same
/// name (`zisk_definitions::ZKVMCALLS`), except for `zkvm_keccak_f1600`, which RISC-V guests
/// inline as the `keccak` precompile and so has no zkvmcall.
const ACCELERATORS: &[Function] = &[
    ef!("zkvm_keccak256", &[Ptr(LenArg(1)), U32, Ptr(Bytes(32))]),
    ef!("zkvm_keccak_f1600", &[Ptr(Bytes(200))]),
    ef!("zkvm_sha256", &[Ptr(LenArg(1)), U32, Ptr(Bytes(32))]),
    ef!("zkvm_ripemd160", &[Ptr(LenArg(1)), U32, Ptr(Bytes(32))]),
    ef!("zkvm_secp256k1_verify", &[Ptr(Bytes(32)), Ptr(Bytes(64)), Ptr(Bytes(64)), Ptr(Bytes(1))]),
    ef!("zkvm_secp256k1_ecrecover", &[Ptr(Bytes(32)), Ptr(Bytes(64)), U8, Ptr(Bytes(64))]),
    ef!("zkvm_secp256r1_verify", &[Ptr(Bytes(32)), Ptr(Bytes(64)), Ptr(Bytes(64)), Ptr(Bytes(1))]),
    ef!(
        "zkvm_modexp",
        &[Ptr(LenArg(1)), U32, Ptr(LenArg(3)), U32, Ptr(LenArg(5)), U32, Ptr(LenArg(5))]
    ),
    ef!("zkvm_bn254_g1_add", &[Ptr(Bytes(64)), Ptr(Bytes(64)), Ptr(Bytes(64))]),
    ef!("zkvm_bn254_g1_mul", &[Ptr(Bytes(64)), Ptr(Bytes(32)), Ptr(Bytes(64))]),
    ef!("zkvm_bn254_pairing", &[Ptr(Array { elem: 192, count: 1 }), U32, Ptr(Bytes(1))]),
    ef!("zkvm_blake2f", &[U32, Ptr(Bytes(64)), Ptr(Bytes(128)), Ptr(Bytes(16)), U8]),
    ef!(
        "zkvm_kzg_point_eval",
        &[Ptr(Bytes(48)), Ptr(Bytes(32)), Ptr(Bytes(32)), Ptr(Bytes(48)), Ptr(Bytes(1))]
    ),
    ef!("zkvm_bls12_g1_add", &[Ptr(Bytes(96)), Ptr(Bytes(96)), Ptr(Bytes(96))]),
    ef!("zkvm_bls12_g1_msm", &[Ptr(Array { elem: 128, count: 1 }), U32, Ptr(Bytes(96))]),
    ef!("zkvm_bls12_g2_add", &[Ptr(Bytes(192)), Ptr(Bytes(192)), Ptr(Bytes(192))]),
    ef!("zkvm_bls12_g2_msm", &[Ptr(Array { elem: 224, count: 1 }), U32, Ptr(Bytes(192))]),
    ef!("zkvm_bls12_pairing", &[Ptr(Array { elem: 288, count: 1 }), U32, Ptr(Bytes(1))]),
    ef!("zkvm_bls12_map_fp_to_g1", &[Ptr(Bytes(48)), Ptr(Bytes(96))]),
    ef!("zkvm_bls12_map_fp2_to_g2", &[Ptr(Bytes(96)), Ptr(Bytes(192))]),
];

/// `zkvm_u256.h`: 32-byte big-endian words.
const U256: [Function; 27] = u256!("zkvm_u256_", Ptr);
/// `zkvm_u256_le.h`: four little-endian `uint64_t` limbs.
const U256_LE: [Function; 27] = u256!("zkvm_u256_le_", Ptr8);

/// `zkvm_random.h` and `write_log` of `zkvm_io.h`.
const RANDOM_AND_LOG: &[Function] = &[
    ef!("zkvm_random_u64", &[], Ret::I64),
    Function {
        name: "write_log",
        kind: Kind::WriteLog,
        args: &[Ptr(LenArg(1)), U32],
        ret: Ret::Void,
    },
];

/// `zkvm_zisklib.h`, in header order.
const ZISKLIB: &[Function] = &[
    zl!("add", &[U64, U64], Ret::I64),
    zl!("inv256", &[Ptr8(Bytes(32)), Ptr8(Bytes(32))], Ret::I64),
    zl!("overflowing_add256", &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))], Ret::I64),
    zl!("overflowing_sub256", &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))], Ret::I64),
    zl!("overflowing_mul256", &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))], Ret::I64),
    zl!(
        "div_rem256",
        &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))],
        Ret::Void
    ),
    zl!("reduce_mod256", &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))], Ret::Void),
    zl!(
        "add_mod256",
        &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))],
        Ret::Void
    ),
    zl!(
        "mul_mod256",
        &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))],
        Ret::Void
    ),
    zl!("inv_mod256", &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))], Ret::I64),
    zl!(
        "pow_mod256",
        &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))],
        Ret::Void
    ),
    zl!("overflowing_pow256", &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))], Ret::I64),
    zl!(
        "ecdsa_verify_secp256k1",
        &[Ptr8(Bytes(64)), Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))],
        Ret::I64
    ),
    zl!(
        "ecdsa_recover_secp256k1",
        &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32)), U64, Ptr8(Bytes(64))],
        Ret::I64
    ),
    zl!(
        "schnorr_verify_secp256k1",
        &[Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr(LenArg(4)), U64],
        Ret::I64
    ),
    zl!(
        "ecdsa_verify_secp256r1",
        &[Ptr8(Bytes(64)), Ptr8(Bytes(32)), Ptr8(Bytes(32)), Ptr8(Bytes(32))],
        Ret::I64
    ),
    zl!(
        "pairing_check_bn254",
        &[Ptr8(Array { elem: 64, count: 2 }), Ptr8(Array { elem: 128, count: 2 }), U64],
        Ret::I64
    ),
    zl!(
        "pairing_check_bls12_381",
        &[Ptr8(Array { elem: 96, count: 2 }), Ptr8(Array { elem: 192, count: 2 }), U64],
        Ret::I64
    ),
    zl!("map_to_curve_g1_bls12_381", &[Ptr8(Bytes(48)), Ptr8(Bytes(96))], Ret::I64),
    zl!("map_to_curve_g2_bls12_381", &[Ptr8(Bytes(96)), Ptr8(Bytes(192))], Ret::I64),
    zl!(
        "hash_to_curve_g2_bls12_381",
        &[Ptr(LenArg(1)), U64, Ptr(LenArg(3)), U64, Ptr8(Bytes(192))],
        Ret::Void
    ),
    zl!("bls_verify_bls12_381", &[Ptr(Bytes(48)), Ptr(LenArg(2)), U64, Ptr(Bytes(96))], Ret::I64),
    zl!(
        "verify_kzg_proof_bls12_381",
        &[Ptr(Bytes(32)), Ptr(Bytes(32)), Ptr(Bytes(48)), Ptr(Bytes(48))],
        Ret::I64
    ),
    zl!(
        "modexp_u64_c",
        &[
            Ptr8(Array { elem: 8, count: 1 }),
            U32,
            Ptr8(Array { elem: 8, count: 3 }),
            U32,
            Ptr8(Array { elem: 8, count: 5 }),
            U32,
            Ptr8(ModexpResult(5)),
        ],
        Ret::I32
    ),
];

/// `zkvm_fcall.h`.
const FCALLS: &[Function] = &[
    Function {
        name: "zkvm_fcall",
        kind: Kind::Fcall,
        args: &[U32, Ptr(Array { elem: 8, count: 2 }), U32],
        ret: Ret::I32,
    },
    Function { name: "zkvm_fcall_get", kind: Kind::FcallGet, args: &[], ret: Ret::I64 },
    Function {
        name: "zkvm_fcall_read",
        kind: Kind::FcallRead,
        args: &[Ptr(Array { elem: 8, count: 1 }), U32],
        ret: Ret::Void,
    },
];

/// The fcalls `zkvm_fcall` issues: the hints (`ZKVM_FCALL_*` in `zkvm_fcall.h`).  The others
/// change the executor's state (input readiness, the keccak-f cache) or hand out the random seed,
/// which only the library's `zkvm_random_u64` may read.
pub const FCALL_HINT_IDS: &[u64] =
    &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 26];

/// `ZKVM_EFAIL`.
const STATUS_FAIL: u64 = u64::MAX;

/// Stack the library routines may use below the stub's frame: 512 nested calls, far more than
/// any routine makes. A call that would take it below `WASM_STACK_LIMIT` traps instead.
const LIBRARY_STACK_BYTES: i64 = 4096;

/// Holds the byte length of the range being checked.
const R_LEN: u64 = 24;
/// Holds the end of the range being checked.
const R_END: u64 = 25;

fn function(name: &str) -> Option<&'static Function> {
    [ACCELERATORS, &U256, &U256_LE, RANDOM_AND_LOG, ZISKLIB, FCALLS]
        .into_iter()
        .flatten()
        .find(|f| f.name == name)
}

/// Whether import `(module, name)` is a function this module implements.
pub fn is_zkvm_import(module: &str, name: &str) -> bool {
    module == IMPORT_MODULE && function(name).is_some()
}

/// The ZisK library routine behind function `name`, if it calls one.
pub fn routine(name: &str) -> Option<&'static str> {
    match function(name)?.kind {
        Kind::Routine(routine) => Some(routine),
        _ => None,
    }
}

/// The zkvmcall ID of function `name`, if it has one.
pub fn zkvmcall_id(name: &str) -> Option<u16> {
    zisk_definitions::ZKVMCALLS.iter().find(|c| c.name == name).map(|c| c.id)
}

/// Whether RISC-V guests expand routine `name` inline: then it sets no status.
fn is_inline(name: &str) -> bool {
    zisk_definitions::ZKVMCALLS.iter().any(|c| c.name == name && c.inline_args > 0)
}

fn val_kind(arg: &Arg) -> ValKind {
    if matches!(arg, U64) {
        ValKind::I64
    } else {
        ValKind::I32
    }
}

/// Builds the stub for import `import_index`; `entry` is the address of its routine, if any.
pub fn build_zkvm_stub(
    module: &WasmModule,
    import_index: usize,
    entry: Option<u64>,
) -> Result<Code, Box<dyn Error>> {
    let (_, name, _) = &module.imports[import_index];
    let f = function(name).ok_or_else(|| format!("wasm: {name} is not a ZisK function"))?;
    let declared = module.func_sig(import_index as u32)?;
    let params: Vec<ValKind> = f.args.iter().map(val_kind).collect();
    let results = match f.ret {
        Ret::I32 => vec![ValKind::I32],
        Ret::I64 => vec![ValKind::I64],
        Ret::Void => vec![],
    };
    if declared.params != params || declared.results != results {
        return Err(format!(
            "wasm: import {IMPORT_MODULE}::{name} is declared as {:?} -> {:?}, expected \
             {params:?} -> {results:?} (its C header compiled for wasm32)",
            declared.params, declared.results
        )
        .into());
    }

    let frame = callee_frame_size(module, import_index as u32)?;
    Ok(wrap_stub(|code, _fault, _done| {
        let out_of_bounds = code.new_label();
        let unaligned = code.new_label();
        load_and_check_args(code, f, out_of_bounds, unaligned);

        match f.kind {
            Kind::Routine(_) => {
                call_routine(code, name, frame, entry.unwrap());
                if f.ret == Ret::I32 && is_inline(name) {
                    code.load_imm_to_reg(REG_RET, 0);
                } else if f.ret == Ret::I32 {
                    // An i32, which the wasm machine keeps sign-extended.
                    code.signextend_w_reg(REG_RET);
                }
            }
            Kind::WriteLog => {
                // The prover sees a no-op; ziskemu prints the text when asked to.
                let mut zib = ZiskInstBuilder::new(0);
                zib.src_a("reg", arg_reg(0), false);
                zib.src_b("reg", arg_reg(1), false);
                zib.op("flag").unwrap();
                zib.i.write_log = true;
                zib.i.func = zisk_core::opc_write_log;
                zib.j(4, 4);
                code.push_raw(zib, Fixup::None);
            }
            Kind::Fcall => emit_fcall(code),
            Kind::FcallGet => fcall_get(code, REG_RET),
            Kind::FcallRead => {
                // r10 = the next result's address, r11 = its end.
                let (ptr, end) = (arg_reg(0), arg_reg(1));
                code.alu_ri("sll", end, end, 3);
                code.alu_rr("add", end, ptr, end);
                let (next, finished) = (code.new_label(), code.new_label());
                code.bind(next);
                code.cmp_reg_branch("eq", ptr, end, finished, true);
                fcall_get(code, R_LEN);
                code.store_reg_to_mem(ptr, 0, R_LEN, 8);
                code.alu_ri("add", ptr, ptr, 8);
                code.jump(next);
                code.bind(finished);
            }
        }

        let done = code.new_label();
        code.jump(done);
        code.bind(out_of_bounds);
        emit_console_str(code, &format!("wasm: {name}: a pointer argument is out of bounds\n"));
        code.trap();
        code.bind(unaligned);
        emit_console_str(
            code,
            &format!("wasm: {name}: a uint64_t pointer argument is not 8-byte aligned\n"),
        );
        code.trap();
        code.bind(done);
    }))
}

fn arg_reg(k: usize) -> u64 {
    10 + k as u64
}

/// Loads every argument into `a0..`, rebases every pointer and checks the range it covers.
fn load_and_check_args(code: &mut Code, f: &Function, out_of_bounds: LabelId, unaligned: LabelId) {
    for (k, arg) in f.args.iter().enumerate() {
        code.load_slot_to_reg(arg_reg(k), local_offset(k as u32));
        match arg {
            U8 => code.alu_ri("and", arg_reg(k), arg_reg(k), 0xFF),
            U64 => {}
            _ => code.alu_ri("and", arg_reg(k), arg_reg(k), 0xFFFF_FFFF),
        }
    }

    for (k, arg) in f.args.iter().enumerate() {
        let (size, aligned) = match *arg {
            Ptr(size) => (size, false),
            Ptr8(size) => (size, true),
            _ => continue,
        };
        let ptr = arg_reg(k);
        if aligned {
            let ok = code.new_label();
            code.alu_ri("and", R_LEN, ptr, 7);
            code.cmp_imm_branch("eq", R_LEN, 0, ok, true);
            code.jump(unaligned);
            code.bind(ok);
        }
        code.alu_ri("add", ptr, ptr, WASM_MEM_BASE as i64);
        let ok = code.new_label();
        // A 64-bit length or count above 2^32 cannot fit in the memory, and would overflow the
        // end computed below.
        let check_u64 = |code: &mut Code, i: usize| {
            if matches!(f.args[i], U64) {
                code.load_imm_to_reg(R_LEN, u32::MAX as u64);
                code.cmp_reg_branch("ltu", R_LEN, arg_reg(i), out_of_bounds, true);
            }
        };
        match size {
            Bytes(n) => code.alu_ri("add", R_END, ptr, n as i64),
            LenArg(len) => {
                // An empty range is never touched, so it may point anywhere (EIP-198 lets a
                // zero-length modexp modulus come with any output pointer).
                code.cmp_imm_branch("eq", arg_reg(len), 0, ok, true);
                check_u64(code, len);
                code.alu_rr("add", R_END, ptr, arg_reg(len));
            }
            Array { elem, count } => {
                code.cmp_imm_branch("eq", arg_reg(count), 0, ok, true);
                check_u64(code, count);
                code.alu_ri("mul", R_LEN, arg_reg(count), elem as i64);
                code.alu_rr("add", R_END, ptr, R_LEN);
            }
            ModexpResult(m) => {
                // 32 * ceil(m / 4) bytes, and at least 32.
                code.alu_ri("add", R_LEN, arg_reg(m), 3);
                code.alu_ri("srl", R_LEN, R_LEN, 2);
                code.alu_ri("sll", R_LEN, R_LEN, 5);
                let big_enough = code.new_label();
                code.cmp_imm_branch("eq", R_LEN, 0, big_enough, false);
                code.load_imm_to_reg(R_LEN, 32);
                code.bind(big_enough);
                code.alu_rr("add", R_END, ptr, R_LEN);
            }
        }
        code.cmp_reg_branch("ltu", REG_MEM_END, R_END, out_of_bounds, true);
        code.bind(ok);
    }
}

/// Calls the routine at `entry` with the arguments in place.
fn call_routine(code: &mut Code, name: &str, frame: i64, entry: u64) {
    // The routines keep return addresses on a stack at `sp` (`push r1` / `pop r1`), which the
    // caller provides, and `sp` is `r2`, the wasm frame pointer: point it at the free stack below
    // this stub's frame for the call, as the next callee's frame would sit, and put it back
    // after. Each nested library call pushes 8 bytes.
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
    // It preserved `REG_MEM_END` too, but not the limit registers, which are RISC-V temporaries.
    code.set_mem_limits();
}

/// `zkvm_fcall(id, params, n_params)`: `ZKVM_EFAIL` for an ID that is not a hint or more
/// parameters than an fcall takes, before anything is passed; otherwise passes the parameters, one
/// word each, issues the fcall and returns `ZKVM_EOK`.  The executor needs the ID in the
/// instruction, so there is one `fcall` per hint, and the ID picks it.
fn emit_fcall(code: &mut Code) {
    let (id, ptr, end) = (arg_reg(0), arg_reg(1), arg_reg(2));
    let (valid, fail, done) = (code.new_label(), code.new_label(), code.new_label());
    code.load_imm_to_reg(R_LEN, FCALL_PARAMS_MAX_SIZE as u64);
    code.cmp_reg_branch("ltu", R_LEN, end, fail, true);
    for &hint in FCALL_HINT_IDS {
        code.cmp_imm_branch("eq", id, hint as i64, valid, true);
    }
    code.jump(fail);

    code.bind(valid);
    code.alu_ri("sll", end, end, 3);
    code.alu_rr("add", end, ptr, end);
    let (next, passed) = (code.new_label(), code.new_label());
    code.bind(next);
    code.cmp_reg_branch("eq", ptr, end, passed, true);
    code.load_mem_to_reg("copyb", R_LEN, ptr, 0, 8);
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("imm", 1, false);
    zib.src_b("reg", R_LEN, false);
    zib.op("fcall_param").unwrap();
    zib.j(4, 4);
    code.push_raw(zib, Fixup::None);
    code.alu_ri("add", ptr, ptr, 8);
    code.jump(next);

    code.bind(passed);
    let issue: Vec<_> = FCALL_HINT_IDS.iter().map(|_| code.new_label()).collect();
    for (&hint, &label) in FCALL_HINT_IDS.iter().zip(&issue) {
        code.cmp_imm_branch("eq", id, hint as i64, label, true);
    }
    for (&hint, &label) in FCALL_HINT_IDS.iter().zip(&issue) {
        code.bind(label);
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", hint, false);
        zib.src_b("imm", 0, false);
        zib.op("fcall").unwrap();
        zib.j(4, 4);
        code.push_raw(zib, Fixup::None);
        code.load_imm_to_reg(REG_RET, 0);
        code.jump(done);
    }

    code.bind(fail);
    code.load_imm_to_reg(REG_RET, STATUS_FAIL);
    code.bind(done);
}

/// `reg` = the next fcall result word.
fn fcall_get(code: &mut Code, reg: u64) {
    let mut zib = ZiskInstBuilder::new(0);
    zib.src_a("imm", 0, false);
    zib.src_b("mem", INPUT_ADDR, false);
    zib.op("fcall_get").unwrap();
    zib.store("reg", reg as i64, false, false);
    zib.j(4, 4);
    code.push_raw(zib, Fixup::None);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> impl Iterator<Item = &'static Function> {
        [ACCELERATORS, &U256, &U256_LE, RANDOM_AND_LOG, ZISKLIB, FCALLS].into_iter().flatten()
    }

    /// Every routine is the target of the zkvmcall of the same name, so both transpilers run the
    /// same code, except for `zkvm_keccak_f1600`, which has no zkvmcall.
    #[test]
    fn routines_match_the_zkvmcall_table() {
        for f in all() {
            let Kind::Routine(routine) = f.kind else { continue };
            match zisk_definitions::ZKVMCALLS.iter().find(|c| c.name == f.name) {
                Some(call) => assert_eq!(call.target, routine, "{}", f.name),
                None => assert_eq!(f.name, "zkvm_keccak_f1600"),
            }
        }
    }

    /// Every zkvmcall with a library routine has a wasm import, except `read_input` and
    /// `write_output`, which WASI's stdin and stdout replace, and the fork-join driver
    /// `zkvm_parallel` with its return thunk `zkvm_parallel_ret`: a task is a code address in the
    /// ROM, and a wasm guest's function reference is a table index.
    #[test]
    fn every_zkvmcall_is_covered() {
        const NOT_IMPORTED: [&str; 4] =
            ["read_input", "write_output", "zkvm_parallel", "zkvm_parallel_ret"];
        for call in zisk_definitions::ZKVMCALLS {
            if !NOT_IMPORTED.contains(&call.name) {
                assert!(function(call.name).is_some(), "{}", call.name);
            }
        }
    }

    /// A length or count refers to an integer argument, and every function takes at most the
    /// eight argument registers `a0..a7`.
    #[test]
    fn signatures_are_consistent() {
        for f in all() {
            assert!(f.args.len() <= 8, "{}", f.name);
            for arg in f.args {
                if let Ptr(size) | Ptr8(size) = arg {
                    if let LenArg(i) | Array { count: i, .. } | ModexpResult(i) = size {
                        assert!(matches!(f.args[*i], U32 | U64), "{}: argument {i}", f.name);
                    }
                }
            }
        }
    }

    /// The library assembles and defines every routine.
    #[test]
    fn library_defines_every_routine() {
        let library = ziskasm::assemble_zisk_library().unwrap();
        for f in all() {
            if let Kind::Routine(routine) = f.kind {
                assert!(library.symbols.contains_key(routine), "{routine}");
            }
        }
    }

    /// None of the executor's own fcalls (ziskos's `FCALL_INPUT_READY_ID`, the two keccak-f cache
    /// IDs and `FCALL_RANDOM_SEED_ID`) is a hint.
    #[test]
    fn hint_ids_exclude_the_executor_fcalls() {
        for id in [23, 24, 25, 27] {
            assert!(!FCALL_HINT_IDS.contains(&id), "{id}");
        }
    }
}
