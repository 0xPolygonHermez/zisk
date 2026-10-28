#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/kzg.zisk : verify_kzg_proof (EIP-4844 point
# evaluation). Mirrors kzg.rs. Reuses decompress_g1/pairing/subgroup. Prefix lk_.
out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)
S="_bls12_381"

w("; ============================================================================")
w("; bls12_381/kzg.zisk - verify_kzg_proof (EIP-4844 point evaluation). GENERATED")
w("; (scripts/legacy/gen_bls_kzg.py). e(C-[y]G1,-G2)·e(proof,[tau-z]G2)==1. lk_.")
w("; ============================================================================")
w("")
w("u64 BLS_K_Z[4] = 0, 0, 0, 0")
w("u64 BLS_K_Y[4] = 0, 0, 0, 0")
w("u64 BLS_K_COMM[12] = "+", ".join(["0"]*12))
w("u64 BLS_K_PROOF[12] = "+", ".join(["0"]*12))
w("u64 BLS_K_YG1[12] = "+", ".join(["0"]*12))
w("u64 BLS_K_CMY[12] = "+", ".join(["0"]*12))
w("u64 BLS_K_ZG2[24] = "+", ".join(["0"]*24))
w("u64 BLS_K_TMZ[24] = "+", ".join(["0"]*24))
w("u64 BLS_K_NEGG2[24] = "+", ".join(["0"]*24))
w("u64 BLS_K_M1[72] = "+", ".join(["0"]*72))
w("u64 BLS_K_M2[72] = "+", ".join(["0"]*72))
w("u64 BLS_K_F[72] = "+", ".join(["0"]*72))
w("u64 BLS_K_RES[72] = "+", ".join(["0"]*72))
w("const u64 BLS_ONE72K[72] = 1"+", 0"*71)
for nm in ["K_IN","K_DEST","K_I","K_CMYINF","K_PFINF","K_TMZINF"]:
    w(f"u64 BLS_{nm}[1] = 0")
w("")

# ---------- scalar_be32_canonical(bytes r10, dest4 r11) -> r10 status(0 ok,1 err: >=R) ----------
w("; u64 zisklib_scalar_be32_canonical(const u8* bytes32, u64* dest4) -> r10 (0 ok, 1 >=R)")
w("zisklib_scalar_be32_canonical_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_K_IN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_K_DEST) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
raw("copyb(0, BLS_K_I) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("lk_sc_loop:")
raw("copyb(0, BLS_K_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("eq(r13, 4), j(lk_sc_done)")
raw("copyb(0, BLS_K_IN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("sll(r13, 3) -> r6"); raw("add(r10, r6) -> r10")
raw("call zisklib_be64")
raw("copyb(0, BLS_K_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("copyb(0, 3) -> r6"); raw("sub(r6, r13) -> r6"); raw("sll(r6, 3) -> r6")
raw("copyb(0, BLS_K_DEST) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("add(r7, r6) -> r7"); raw("copyb(r7, r10) -> 8[a + 0]")
raw("add(r13, 1) -> r13"); raw("copyb(0, BLS_K_I) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]")
raw("jump(lk_sc_loop)")
w("lk_sc_done:")
# dest < R ?
raw("copyb(0, BLS_K_DEST) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_R) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_ltn")
raw("eq(r10, 0), j(lk_sc_err)")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret")
w("lk_sc_err:")
raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret"); w("")

# ---------- verify_kzg_proof(z r10, y r11, commitment r12, proof r13) -> r10 (1/0) ----------
w("; u64 zisklib_verify_kzg_proof(z32,y32,commitment48,proof48) -> r10 (1 valid/0 invalid)")
w("zisklib_verify_kzg_proof_bls12_381:")
raw("push r1")
# stash the 4 input ptrs (use K buffers' first words via dedicated slots)
raw("copyb(0, BLS_K_Z) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")     # temporarily store z ptr in K_Z[0]
raw("copyb(0, BLS_K_Y) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")     # y ptr in K_Y[0]
raw("copyb(0, BLS_K_M1) -> r5"); raw("copyb(r5, r12) -> 8[a + 0]")    # commitment ptr in K_M1[0]
raw("copyb(0, BLS_K_M2) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]")    # proof ptr in K_M2[0]
# decompress commitment -> BLS_V_PK ; subgroup if not inf
raw("copyb(0, BLS_K_M1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_decompress_g1_bls12_381"); raw("ltu(0, r10), j(lk_bad)")
raw("copyb(0, BLS_V_PK) -> r10"); raw("copyb(0, BLS_K_COMM) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_V_G1INF) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("ltu(0, r6), j(lk_comm_ok)")
raw("copyb(0, BLS_K_COMM) -> r10"); raw("call zisklib_is_on_subgroup_bls12_381"); raw("eq(r10, 0), j(lk_bad)")
w("lk_comm_ok:")
# decompress proof -> BLS_V_PK ; subgroup if not inf
raw("copyb(0, BLS_K_M2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_decompress_g1_bls12_381"); raw("ltu(0, r10), j(lk_bad)")
raw("copyb(0, BLS_V_PK) -> r10"); raw("copyb(0, BLS_K_PROOF) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_V_G1INF) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("ltu(0, r6), j(lk_proof_ok)")
raw("copyb(0, BLS_K_PROOF) -> r10"); raw("call zisklib_is_on_subgroup_bls12_381"); raw("eq(r10, 0), j(lk_bad)")
w("lk_proof_ok:")
# z, y scalars (canonical). Reload ptrs BEFORE overwriting K_Z/K_Y with the parsed value.
raw("copyb(0, BLS_K_Z) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_K_Z) -> r11"); raw("call zisklib_scalar_be32_canonical_bls12_381"); raw("ltu(0, r10), j(lk_bad)")
raw("copyb(0, BLS_K_Y) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_K_Y) -> r11"); raw("call zisklib_scalar_be32_canonical_bls12_381"); raw("ltu(0, r10), j(lk_bad)")
# y_g1 = scalar_mul(G1_GEN, y) ; c_minus_y = sub_complete(commitment, y_g1)
raw("copyb(0, BLS_G1_GENERATOR) -> r10"); raw("copyb(0, BLS_K_Y) -> r11"); raw("copyb(0, BLS_K_YG1) -> r12"); raw(f"call zisklib_scalar_mul{S}")
raw("copyb(0, BLS_K_COMM) -> r10"); raw("copyb(0, BLS_K_YG1) -> r11"); raw("copyb(0, BLS_K_CMY) -> r12"); raw(f"call zisklib_sub_complete{S}")
# z_g2 = scalar_mul_twist(G2_GEN, z) ; t_minus_z = sub_complete_twist(TAU_G2, z_g2)
raw("copyb(0, BLS_G2_GENERATOR) -> r10"); raw("copyb(0, BLS_K_Z) -> r11"); raw("copyb(0, BLS_K_ZG2) -> r12"); raw(f"call zisklib_scalar_mul_twist{S}")
raw("copyb(0, BLS_TRUSTED_SETUP_TAU_G2) -> r10"); raw("copyb(0, BLS_K_ZG2) -> r11"); raw("copyb(0, BLS_K_TMZ) -> r12"); raw(f"call zisklib_sub_complete_twist{S}")
# inf flags
raw("copyb(0, BLS_K_CMY) -> r10"); raw("copyb(0, BLS_G1_IDENTITY) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn"); raw("copyb(0, BLS_K_CMYINF) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_K_PROOF) -> r10"); raw("copyb(0, BLS_G1_IDENTITY) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn"); raw("copyb(0, BLS_K_PFINF) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_K_TMZ) -> r10"); raw("copyb(0, BLS_G2_IDENTITY) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_eqn"); raw("copyb(0, BLS_K_TMZINF) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
# if c_minus_y_is_inf: return proof_inf || tmz_inf
raw("copyb(0, BLS_K_CMYINF) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("eq(r6, 0), j(lk_cmy_nz)")
raw("copyb(0, BLS_K_PFINF) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("ltu(0, r6), j(lk_accept)")
raw("copyb(0, BLS_K_TMZINF) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("ltu(0, r6), j(lk_accept)")
raw("jump(lk_bad)")
w("lk_cmy_nz:")
# if proof_inf || tmz_inf: return false
raw("copyb(0, BLS_K_PFINF) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("ltu(0, r6), j(lk_bad)")
raw("copyb(0, BLS_K_TMZINF) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("ltu(0, r6), j(lk_bad)")
# neg_g2 = neg_twist(G2_GEN)
raw("copyb(0, BLS_G2_GENERATOR) -> r10"); raw("copyb(0, BLS_K_NEGG2) -> r11"); raw(f"call zisklib_neg_twist{S}")
# f = miller(c_minus_y, neg_g2) * miller(proof, t_minus_z)
raw("copyb(0, BLS_K_CMY) -> r10"); raw("copyb(0, BLS_K_NEGG2) -> r11"); raw("copyb(0, BLS_K_M1) -> r12"); raw(f"call zisklib_miller_loop{S}")
raw("copyb(0, BLS_K_PROOF) -> r10"); raw("copyb(0, BLS_K_TMZ) -> r11"); raw("copyb(0, BLS_K_M2) -> r12"); raw(f"call zisklib_miller_loop{S}")
raw("copyb(0, BLS_K_M1) -> r10"); raw("copyb(0, BLS_K_M2) -> r11"); raw("copyb(0, BLS_K_F) -> r12"); raw(f"call zisklib_mul_fp12{S}")
raw("copyb(0, BLS_K_F) -> r10"); raw("copyb(0, BLS_K_RES) -> r11"); raw(f"call zisklib_final_exp{S}")
raw("copyb(0, BLS_K_RES) -> r10"); raw("copyb(0, BLS_ONE72K) -> r11"); raw("copyb(0, 72) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret")
w("lk_accept:")
raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret")
w("lk_bad:")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret"); w("")

open(_OUT+"/bls12_381/kzg.zisk","w").write("\n".join(out)+"\n")
print("wrote kzg.zisk", len(out), "lines")
