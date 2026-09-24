/* zkvm_calls.s -- the zkvmcall thunks: every EF standard function implemented by
 * a hand-written .zisk routine (zkvm_accelerators.h, zkvm_io.h, zkvm_u256.h).
 *
 * Each thunk is a `csrs <id>, x0` followed by `ret`. The caller has already put
 * the arguments in a0..a7 and its return address in ra, following the RISC-V
 * calling convention. At transpile time the `csrs` becomes a jump (no link) to
 * the .zisk routine, which returns its result in a0 and `ret`s straight to the
 * caller. The thunk's own `ret` is never reached; it is there so the function
 * looks like any other to disassemblers and debuggers.
 *
 * The IDs are the ones in definitions/src/zkvmcall.rs, and a test in
 * transpilers/common checks that the two lists match. Since the transpiler finds
 * the calls by instruction, not by symbol name, the guest ELF can be stripped. A
 * guest that uses one needs ziskemu/cargo-zisk built with --features ziskasm;
 * without it, the transpiler rejects the ELF.
 *
 * zkvm_keccak_f1600 is not here: it is a single precompile, so zkvm_accelerators.h
 * defines it inline.
 *
 * Each thunk has its own section so --gc-sections drops the unused ones.
 */

.macro ZKVMCALL name, id
    .section .text.\name,"ax",@progbits
    .globl  \name
    .type   \name, @function
    .p2align 2
\name:
    csrs    \id, x0
    ret
    .size   \name, . - \name
.endm

/* ---- accelerators (zkvm_accelerators.h) ---- */
ZKVMCALL zkvm_keccak256,           0x850
ZKVMCALL zkvm_sha256,              0x851
ZKVMCALL zkvm_ripemd160,           0x852
ZKVMCALL zkvm_secp256k1_verify,    0x853
ZKVMCALL zkvm_secp256k1_ecrecover, 0x854
ZKVMCALL zkvm_secp256r1_verify,    0x855
ZKVMCALL zkvm_modexp,              0x856
ZKVMCALL zkvm_bn254_g1_add,        0x857
ZKVMCALL zkvm_bn254_g1_mul,        0x858
ZKVMCALL zkvm_bn254_pairing,       0x859
ZKVMCALL zkvm_blake2f,             0x85A
ZKVMCALL zkvm_kzg_point_eval,      0x85B
ZKVMCALL zkvm_bls12_g1_add,        0x85C
ZKVMCALL zkvm_bls12_g1_msm,        0x85D
ZKVMCALL zkvm_bls12_g2_add,        0x85E
ZKVMCALL zkvm_bls12_g2_msm,        0x85F
ZKVMCALL zkvm_bls12_pairing,       0x860
ZKVMCALL zkvm_bls12_map_fp_to_g1,  0x861
ZKVMCALL zkvm_bls12_map_fp2_to_g2, 0x862

/* ---- I/O (zkvm_io.h) ---- */
ZKVMCALL read_input,               0x863
ZKVMCALL write_output,             0x864

/* ---- U256 arithmetic (zkvm_u256.h) ---- */
ZKVMCALL zkvm_u256_add,            0x865
ZKVMCALL zkvm_u256_sub,            0x866
ZKVMCALL zkvm_u256_mul,            0x867
ZKVMCALL zkvm_u256_div,            0x868
ZKVMCALL zkvm_u256_mod,            0x869
ZKVMCALL zkvm_u256_divmod,         0x86A
ZKVMCALL zkvm_u256_addmod,         0x86B
ZKVMCALL zkvm_u256_mulmod,         0x86C
ZKVMCALL zkvm_u256_exp,            0x86D
ZKVMCALL zkvm_u256_sdiv,           0x86E
ZKVMCALL zkvm_u256_smod,           0x86F
ZKVMCALL zkvm_u256_sdivmod,        0x870
ZKVMCALL zkvm_u256_lt,             0x871
ZKVMCALL zkvm_u256_gt,             0x872
ZKVMCALL zkvm_u256_slt,            0x873
ZKVMCALL zkvm_u256_sgt,            0x874
ZKVMCALL zkvm_u256_eq,             0x875
ZKVMCALL zkvm_u256_iszero,         0x876
ZKVMCALL zkvm_u256_and,            0x877
ZKVMCALL zkvm_u256_or,             0x878
ZKVMCALL zkvm_u256_xor,            0x879
ZKVMCALL zkvm_u256_not,            0x87A
ZKVMCALL zkvm_u256_byte,           0x87B
ZKVMCALL zkvm_u256_shl,            0x87C
ZKVMCALL zkvm_u256_shr,            0x87D
ZKVMCALL zkvm_u256_sar,            0x87E
ZKVMCALL zkvm_u256_signextend,     0x87F
