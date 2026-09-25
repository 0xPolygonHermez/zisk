// zkvmcalls 0x850 - 0x8BF (112 calls), between the syscalls (0x800 - 0x84F) and the
// fcalls (0x8C0 - 0x8DF)
//
// A zkvmcall is a call into a hand-written `.zisk` routine of the ZisK library
// (ziskasm/zisklib/). The guest side is a two-instruction thunk named after the
// standard function:
//
//     zkvm_modexp:
//         csrs 0x856, x0
//         ret
//
// The caller has already loaded the arguments into a0..a7 and its return address
// into ra, following the RISC-V calling convention. The transpiler replaces the
// `csrs` with a jump (no link) to the routine, which reads its arguments from
// a0..a7, returns its result in a0 and `ret`s straight to the caller; the thunk's
// own `ret` is never reached.
//
// Important: IDs must match ziskasm/lang/c/src/zkvm_calls.s (checked by a test in
// transpilers/common), and are never reused or renumbered once assigned.

pub const ZKVMCALL_ADDR_START: u16 = 0x850;
pub const ZKVMCALL_ADDR_END: u16 = 0x8BF;

/// One zkvmcall: its CSR number, the guest function it implements and the ZisK
/// library routine the transpiler jumps to.
pub struct ZkvmCall {
    pub id: u16,
    pub name: &'static str,
    pub target: &'static str,
}

const fn zc(id: u16, name: &'static str, target: &'static str) -> ZkvmCall {
    ZkvmCall { id, name, target }
}

/// Every zkvmcall, in ID order. `zkvm_keccak_f1600` is not here: it is a single
/// precompile, so the guest header inlines it as `csrs 0x800, state` (the keccakf
/// syscall) instead of calling anything.
pub const ZKVMCALLS: &[ZkvmCall] = &[
    // EF accelerators (zkvm_accelerators.h)
    zc(0x850, "zkvm_keccak256", "ziskasm_zkvm_keccak256"),
    zc(0x851, "zkvm_sha256", "ziskasm_zkvm_sha256"),
    zc(0x852, "zkvm_ripemd160", "ziskasm_zkvm_ripemd160"),
    zc(0x853, "zkvm_secp256k1_verify", "ziskasm_zkvm_secp256k1_verify"),
    zc(0x854, "zkvm_secp256k1_ecrecover", "ziskasm_zkvm_secp256k1_ecrecover"),
    zc(0x855, "zkvm_secp256r1_verify", "ziskasm_zkvm_secp256r1_verify"),
    zc(0x856, "zkvm_modexp", "ziskasm_zkvm_modexp"),
    zc(0x857, "zkvm_bn254_g1_add", "ziskasm_zkvm_bn254_g1_add"),
    zc(0x858, "zkvm_bn254_g1_mul", "ziskasm_zkvm_bn254_g1_mul"),
    zc(0x859, "zkvm_bn254_pairing", "ziskasm_zkvm_bn254_pairing"),
    zc(0x85A, "zkvm_blake2f", "ziskasm_zkvm_blake2f"),
    zc(0x85B, "zkvm_kzg_point_eval", "ziskasm_zkvm_kzg_point_eval"),
    zc(0x85C, "zkvm_bls12_g1_add", "ziskasm_zkvm_bls12_g1_add"),
    zc(0x85D, "zkvm_bls12_g1_msm", "ziskasm_zkvm_bls12_g1_msm"),
    zc(0x85E, "zkvm_bls12_g2_add", "ziskasm_zkvm_bls12_g2_add"),
    zc(0x85F, "zkvm_bls12_g2_msm", "ziskasm_zkvm_bls12_g2_msm"),
    zc(0x860, "zkvm_bls12_pairing", "ziskasm_zkvm_bls12_pairing"),
    zc(0x861, "zkvm_bls12_map_fp_to_g1", "ziskasm_zkvm_bls12_map_fp_to_g1"),
    zc(0x862, "zkvm_bls12_map_fp2_to_g2", "ziskasm_zkvm_bls12_map_fp2_to_g2"),
    // EF I/O (zkvm_io.h)
    zc(0x863, "read_input", "zisklib_read_input"),
    zc(0x864, "write_output", "zisklib_write_output"),
    // EF U256 arithmetic (zkvm_u256.h)
    zc(0x865, "zkvm_u256_add", "ziskasm_zkvm_u256_add"),
    zc(0x866, "zkvm_u256_sub", "ziskasm_zkvm_u256_sub"),
    zc(0x867, "zkvm_u256_mul", "ziskasm_zkvm_u256_mul"),
    zc(0x868, "zkvm_u256_div", "ziskasm_zkvm_u256_div"),
    zc(0x869, "zkvm_u256_mod", "ziskasm_zkvm_u256_mod"),
    zc(0x86A, "zkvm_u256_divmod", "ziskasm_zkvm_u256_divmod"),
    zc(0x86B, "zkvm_u256_addmod", "ziskasm_zkvm_u256_addmod"),
    zc(0x86C, "zkvm_u256_mulmod", "ziskasm_zkvm_u256_mulmod"),
    zc(0x86D, "zkvm_u256_exp", "ziskasm_zkvm_u256_exp"),
    zc(0x86E, "zkvm_u256_sdiv", "ziskasm_zkvm_u256_sdiv"),
    zc(0x86F, "zkvm_u256_smod", "ziskasm_zkvm_u256_smod"),
    zc(0x870, "zkvm_u256_sdivmod", "ziskasm_zkvm_u256_sdivmod"),
    zc(0x871, "zkvm_u256_lt", "ziskasm_zkvm_u256_lt"),
    zc(0x872, "zkvm_u256_gt", "ziskasm_zkvm_u256_gt"),
    zc(0x873, "zkvm_u256_slt", "ziskasm_zkvm_u256_slt"),
    zc(0x874, "zkvm_u256_sgt", "ziskasm_zkvm_u256_sgt"),
    zc(0x875, "zkvm_u256_eq", "ziskasm_zkvm_u256_eq"),
    zc(0x876, "zkvm_u256_iszero", "ziskasm_zkvm_u256_iszero"),
    zc(0x877, "zkvm_u256_and", "ziskasm_zkvm_u256_and"),
    zc(0x878, "zkvm_u256_or", "ziskasm_zkvm_u256_or"),
    zc(0x879, "zkvm_u256_xor", "ziskasm_zkvm_u256_xor"),
    zc(0x87A, "zkvm_u256_not", "ziskasm_zkvm_u256_not"),
    zc(0x87B, "zkvm_u256_byte", "ziskasm_zkvm_u256_byte"),
    zc(0x87C, "zkvm_u256_shl", "ziskasm_zkvm_u256_shl"),
    zc(0x87D, "zkvm_u256_shr", "ziskasm_zkvm_u256_shr"),
    zc(0x87E, "zkvm_u256_sar", "ziskasm_zkvm_u256_sar"),
    zc(0x87F, "zkvm_u256_signextend", "ziskasm_zkvm_u256_signextend"),
    // Little-endian U256 (ziskasm/lang/c/include/zkvm_u256_le.h, not EF)
    zc(0x880, "zkvm_u256_le_div", "ziskasm_zkvm_u256_le_div"),
    zc(0x881, "zkvm_u256_le_mod", "ziskasm_zkvm_u256_le_mod"),
    zc(0x882, "zkvm_u256_le_divmod", "ziskasm_zkvm_u256_le_divmod"),
    zc(0x883, "zkvm_u256_le_sdiv", "ziskasm_zkvm_u256_le_sdiv"),
    zc(0x884, "zkvm_u256_le_smod", "ziskasm_zkvm_u256_le_smod"),
    zc(0x885, "zkvm_u256_le_sdivmod", "ziskasm_zkvm_u256_le_sdivmod"),
    zc(0x886, "zkvm_u256_le_add", "ziskasm_zkvm_u256_le_add"),
    zc(0x887, "zkvm_u256_le_sub", "ziskasm_zkvm_u256_le_sub"),
    zc(0x888, "zkvm_u256_le_mul", "ziskasm_zkvm_u256_le_mul"),
    zc(0x889, "zkvm_u256_le_addmod", "ziskasm_zkvm_u256_le_addmod"),
    zc(0x88A, "zkvm_u256_le_mulmod", "ziskasm_zkvm_u256_le_mulmod"),
    zc(0x88B, "zkvm_u256_le_exp", "ziskasm_zkvm_u256_le_exp"),
    zc(0x88C, "zkvm_u256_le_lt", "ziskasm_zkvm_u256_le_lt"),
    zc(0x88D, "zkvm_u256_le_gt", "ziskasm_zkvm_u256_le_gt"),
    zc(0x88E, "zkvm_u256_le_slt", "ziskasm_zkvm_u256_le_slt"),
    zc(0x88F, "zkvm_u256_le_sgt", "ziskasm_zkvm_u256_le_sgt"),
    zc(0x890, "zkvm_u256_le_eq", "ziskasm_zkvm_u256_le_eq"),
    zc(0x891, "zkvm_u256_le_iszero", "ziskasm_zkvm_u256_le_iszero"),
    zc(0x892, "zkvm_u256_le_and", "ziskasm_zkvm_u256_le_and"),
    zc(0x893, "zkvm_u256_le_or", "ziskasm_zkvm_u256_le_or"),
    zc(0x894, "zkvm_u256_le_xor", "ziskasm_zkvm_u256_le_xor"),
    zc(0x895, "zkvm_u256_le_not", "ziskasm_zkvm_u256_le_not"),
    zc(0x896, "zkvm_u256_le_byte", "ziskasm_zkvm_u256_le_byte"),
    zc(0x897, "zkvm_u256_le_shl", "ziskasm_zkvm_u256_le_shl"),
    zc(0x898, "zkvm_u256_le_shr", "ziskasm_zkvm_u256_le_shr"),
    zc(0x899, "zkvm_u256_le_sar", "ziskasm_zkvm_u256_le_sar"),
    zc(0x89A, "zkvm_u256_le_signextend", "ziskasm_zkvm_u256_le_signextend"),
];

/// Returns the zkvmcall with CSR number `id`, if any.
pub fn zkvmcall_by_id(id: u16) -> Option<&'static ZkvmCall> {
    ZKVMCALLS.iter().find(|c| c.id == id)
}

/// Returns the CSR number of the zkvmcall that implements `name`, at compile time:
/// `zkvmcall_id("zkvm_modexp")` is 0x856, and an unknown name fails the build.
pub const fn zkvmcall_id(name: &str) -> u16 {
    let mut i = 0;
    while i < ZKVMCALLS.len() {
        if str_eq(ZKVMCALLS[i].name, name) {
            return ZKVMCALLS[i].id;
        }
        i += 1;
    }
    panic!("unknown zkvmcall name");
}

const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
