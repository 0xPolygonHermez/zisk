#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/subgroup.zisk : G1 sub_complete,
# scalar_mul_by_x2div3, is_on_subgroup ; G2 utf(psi), scalar_mul_by_abs_x,
# is_on_subgroup_twist.  Mirrors curve.rs/twist.rs.  Prefix lsg_.
import re
# --- bit arrays from constants.rs ---
rs=open(_REPO+"/ziskos/entrypoint/src/zisklib/lib/bls12_381/constants.rs").read()
def arr(name):
    m=re.search(name+r"\s*:\s*\[u8;\s*\d+\]\s*=\s*\[([0-9,\s]+)\];", rs)
    return [int(x) for x in m.group(1).replace("\n"," ").split(",") if x.strip()!=""]
X2DIV3=arr("X2DIV3_BIN_BE"); XABS=arr("X_ABS_BIN_BE")
assert len(X2DIV3)==126 and len(XABS)==64, (len(X2DIV3),len(XABS))

out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)
S="_bls12_381"

w("; ============================================================================")
w("; bls12_381/subgroup.zisk - G1/G2 subgroup membership (EIP-2537). GENERATED")
w("; (scripts/legacy/gen_bls_subgroup.py). G1: ((x^2-1)/3)(2σP-P-σ^2P)==σ^2P.")
w("; G2: x·ψ^3(P)+P==ψ^2(P).  Prefix lsg_.")
w("; ============================================================================")
w("")
w("const u64 BLS_X2DIV3_BE[126] = "+", ".join(str(b) for b in X2DIV3))
w("const u64 BLS_XABS_BE[64] = "+", ".join(str(b) for b in XABS))
w("")
# scratch
for nm,sz in [("SGP",1),("SGR",1),("SGP2",1)]:
    w(f"u64 BLS_{nm}[{sz}] = 0")
for nm in ["SG1_NEG","SG1_R","SG1_R2","SG1_S1","SG1_S2","SG1_LHS","SG1_T"]:
    w(f"u64 BLS_{nm}[12] = "+", ".join(["0"]*12))
for nm in ["SG2_XA","SG2_XB","SG2_YA","SG2_YB"]:
    w(f"u64 BLS_{nm}[12] = "+", ".join(["0"]*12))
for nm in ["SG2_R","SG2_R2","SG2_U1","SG2_U2","SG2_U3","SG2_XU","SG2_LHS"]:
    w(f"u64 BLS_{nm}[24] = "+", ".join(["0"]*24))
w("")

def save(slot,reg): raw(f"copyb(0, {slot}) -> r5"); raw(f"copyb(r5, {reg}) -> 8[a + 0]")
def load(reg,slot): raw(f"copyb(0, {slot}) -> {reg}"); raw(f"copyb({reg}, 8[a + 0]) -> {reg}")
def cpyn(src,dst,n): raw(f"copyb(0, {src}) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"copyb(0, {n}) -> r12"); raw("call bn254_memcpy")
def cpyn_from_ptr(ptrslot,dst,n):  # dst = [ptrslot]
    raw(f"copyb(0, {ptrslot}) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
    raw(f"copyb(0, {dst}) -> r11"); raw(f"copyb(0, {n}) -> r12"); raw("call bn254_memcpy")
def cpyn_to_ptr(src,ptrslot,n):    # [ptrslot] = src
    raw(f"copyb(0, {src}) -> r10")
    raw(f"copyb(0, {ptrslot}) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw(f"copyb(0, {n}) -> r12"); raw("call bn254_memcpy")

# ============ G1 sub_complete(p1, p2, result) ============
w("; void zisklib_sub_complete_bls12_381(const u64* p1, const u64* p2, u64* result)  [12,12,12]")
w("zisklib_sub_complete_bls12_381:")
raw("push r1")
save("BLS_SGP","r10"); save("BLS_SGR","r12")   # p1, result
raw("copyb(0, r11) -> r10"); raw("copyb(0, BLS_SG1_NEG) -> r11"); raw(f"call zisklib_neg{S}")
load("r10","BLS_SGP"); raw("copyb(0, BLS_SG1_NEG) -> r11")
raw("copyb(0, BLS_SGR) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw(f"call zisklib_add_complete{S}")
raw("pop r1"); raw("ret"); w("")

# ============ G1 scalar_mul_by_x2div3(p, result) ============
w("; void zisklib_scalar_mul_by_x2div3_complete_bls12_381(const u64* p, u64* result)  [12,12]")
w("zisklib_scalar_mul_by_x2div3_complete_bls12_381:")
raw("push r1")
save("BLS_SGP","r10"); save("BLS_SGR","r11")
# identity? -> result = p
raw("copyb(0, BLS_SGP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_G1_IDENTITY) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(lsg_x2_go)")
cpyn_from_ptr("BLS_SGP","BLS_SG1_T",12); cpyn_to_ptr("BLS_SG1_T","BLS_SGR",12)
raw("pop r1"); raw("ret")
w("lsg_x2_go:")
cpyn_from_ptr("BLS_SGP","BLS_SG1_R",12)   # r = p
raw("copyb(0, 1) -> r13")
w("lsg_x2_loop:")
raw("eq(r13, 126), j(lsg_x2_done)")
raw("copyb(0, BLS_SG1_R) -> r10"); raw("copyb(0, BLS_SG1_R2) -> r11"); raw(f"call zisklib_dbl_complete{S}")
cpyn("BLS_SG1_R2","BLS_SG1_R",12)
raw("copyb(0, BLS_X2DIV3_BE) -> r5"); raw("sll(r13, 3) -> r6"); raw("add(r5, r6) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("eq(r6, 0), j(lsg_x2_next)")
raw("copyb(0, BLS_SG1_R) -> r10"); raw("copyb(0, BLS_SGP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_SG1_R2) -> r12"); raw(f"call zisklib_add_complete{S}")
cpyn("BLS_SG1_R2","BLS_SG1_R",12)
w("lsg_x2_next:")
raw("add(r13, 1) -> r13"); raw("jump(lsg_x2_loop)")
w("lsg_x2_done:")
cpyn_to_ptr("BLS_SG1_R","BLS_SGR",12)
raw("pop r1"); raw("ret"); w("")

# ============ G1 is_on_subgroup(p) -> r10 ============
w("; u64 zisklib_is_on_subgroup_bls12_381(const u64* p)  [r10=p] -> r10 (1/0)")
w("zisklib_is_on_subgroup_bls12_381:")
raw("push r1")
save("BLS_SGP","r10")
# S1 = sigma(p) ; S2 = sigma(S1)
raw("copyb(0, BLS_SGP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_SG1_S1) -> r11"); raw(f"call zisklib_sigma_endomorphism{S}")
raw("copyb(0, BLS_SG1_S1) -> r10"); raw("copyb(0, BLS_SG1_S2) -> r11"); raw(f"call zisklib_sigma_endomorphism{S}")
# LHS = dbl_complete(S1)
raw("copyb(0, BLS_SG1_S1) -> r10"); raw("copyb(0, BLS_SG1_LHS) -> r11"); raw(f"call zisklib_dbl_complete{S}")
# LHS = sub_complete(LHS, p)
raw("copyb(0, BLS_SG1_LHS) -> r10"); raw("copyb(0, BLS_SGP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_SG1_T) -> r12"); raw(f"call zisklib_sub_complete{S}")
cpyn("BLS_SG1_T","BLS_SG1_LHS",12)
# LHS = sub_complete(LHS, S2)
raw("copyb(0, BLS_SG1_LHS) -> r10"); raw("copyb(0, BLS_SG1_S2) -> r11"); raw("copyb(0, BLS_SG1_T) -> r12"); raw(f"call zisklib_sub_complete{S}")
cpyn("BLS_SG1_T","BLS_SG1_LHS",12)
# LHS = scalar_mul_by_x2div3(LHS)
raw("copyb(0, BLS_SG1_LHS) -> r10"); raw("copyb(0, BLS_SG1_T) -> r11"); raw(f"call zisklib_scalar_mul_by_x2div3_complete{S}")
# return eqn(T, S2, 12)
raw("copyb(0, BLS_SG1_T) -> r10"); raw("copyb(0, BLS_SG1_S2) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret"); w("")

# ============ G2 utf (psi) endomorphism(p, result) ============
w("; void zisklib_utf_endomorphism_twist_bls12_381(const u64* p, u64* result)  [24,24]")
w("zisklib_utf_endomorphism_twist_bls12_381:")
raw("push r1")
save("BLS_SGR","r11")   # result
# XA = p[0..12] ; YA = p[12..24]
raw("copyb(0, r10) -> r5")   # p ptr in r5 (transient, will save)
save("BLS_SGP","r10")
cpyn_from_ptr("BLS_SGP","BLS_SG2_XA",12)
raw("copyb(0, BLS_SGP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 96) -> r10"); raw("copyb(0, BLS_SG2_YA) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
# x = mul_fp2(x, EXT_U_INV) ; y = mul_fp2(y, EXT_U_INV)   (result != b(const) ok, dst==a ok)
raw("copyb(0, BLS_SG2_XA) -> r10"); raw("copyb(0, BLS_EXT_U_INV) -> r11"); raw("copyb(0, BLS_SG2_XB) -> r12"); raw(f"call zisklib_mul_fp2{S}")
raw("copyb(0, BLS_SG2_YA) -> r10"); raw("copyb(0, BLS_EXT_U_INV) -> r11"); raw("copyb(0, BLS_SG2_YB) -> r12"); raw(f"call zisklib_mul_fp2{S}")
# x = conj(x) ; x = scalar_mul_fp2(x, GAMMA14)
raw("copyb(0, BLS_SG2_XB) -> r10"); raw("copyb(0, BLS_SG2_XA) -> r11"); raw(f"call zisklib_conjugate_fp2{S}")
raw("copyb(0, BLS_SG2_XA) -> r10"); raw("copyb(0, BLS_FROBENIUS_GAMMA14) -> r11"); raw("copyb(0, BLS_SG2_XB) -> r12"); raw(f"call zisklib_scalar_mul_fp2{S}")
# y = conj(y) ; y = mul_fp2(y, GAMMA13)
raw("copyb(0, BLS_SG2_YB) -> r10"); raw("copyb(0, BLS_SG2_YA) -> r11"); raw(f"call zisklib_conjugate_fp2{S}")
raw("copyb(0, BLS_SG2_YA) -> r10"); raw("copyb(0, BLS_FROBENIUS_GAMMA13) -> r11"); raw("copyb(0, BLS_SG2_YB) -> r12"); raw(f"call zisklib_mul_fp2{S}")
# x = mul_fp2(x, XI) ; y = mul_fp2(y, XI)   (EXT_U = 1+u = BLS_XI)
raw("copyb(0, BLS_SG2_XB) -> r10"); raw("copyb(0, BLS_XI) -> r11"); raw("copyb(0, BLS_SG2_XA) -> r12"); raw(f"call zisklib_mul_fp2{S}")
raw("copyb(0, BLS_SG2_YB) -> r10"); raw("copyb(0, BLS_XI) -> r11"); raw("copyb(0, BLS_SG2_YA) -> r12"); raw(f"call zisklib_mul_fp2{S}")
# result[0..12] = XA ; result[12..24] = YA
raw("copyb(0, BLS_SG2_XA) -> r10"); raw("copyb(0, BLS_SGR) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_SG2_YA) -> r10"); raw("copyb(0, BLS_SGR) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("add(r11, 96) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ============ G2 scalar_mul_by_abs_x(p, result) ============
w("; void zisklib_scalar_mul_by_abs_x_complete_twist_bls12_381(const u64* p, u64* result)  [24,24]")
w("zisklib_scalar_mul_by_abs_x_complete_twist_bls12_381:")
raw("push r1")
save("BLS_SGP","r10"); save("BLS_SGR","r11")
raw("copyb(0, BLS_SGP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_G2_IDENTITY) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(lsg_ax_go)")
cpyn_from_ptr("BLS_SGP","BLS_SG2_R",24); cpyn_to_ptr("BLS_SG2_R","BLS_SGR",24)
raw("pop r1"); raw("ret")
w("lsg_ax_go:")
cpyn_from_ptr("BLS_SGP","BLS_SG2_R",24)
raw("copyb(0, 1) -> r13")
w("lsg_ax_loop:")
raw("eq(r13, 64), j(lsg_ax_done)")
raw("copyb(0, BLS_SG2_R) -> r10"); raw("copyb(0, BLS_SG2_R2) -> r11"); raw(f"call zisklib_dbl_complete_twist{S}")
cpyn("BLS_SG2_R2","BLS_SG2_R",24)
raw("copyb(0, BLS_XABS_BE) -> r5"); raw("sll(r13, 3) -> r6"); raw("add(r5, r6) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("eq(r6, 0), j(lsg_ax_next)")
raw("copyb(0, BLS_SG2_R) -> r10"); raw("copyb(0, BLS_SGP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_SG2_R2) -> r12"); raw(f"call zisklib_add_complete_twist{S}")
cpyn("BLS_SG2_R2","BLS_SG2_R",24)
w("lsg_ax_next:")
raw("add(r13, 1) -> r13"); raw("jump(lsg_ax_loop)")
w("lsg_ax_done:")
cpyn_to_ptr("BLS_SG2_R","BLS_SGR",24)
raw("pop r1"); raw("ret"); w("")

# ============ G2 is_on_subgroup_twist(p) -> r10 ============
w("; u64 zisklib_is_on_subgroup_twist_bls12_381(const u64* p)  [r10=p] -> r10 (1/0)")
w("zisklib_is_on_subgroup_twist_bls12_381:")
raw("push r1")
save("BLS_SGP2","r10")
# U1 = psi(p) ; U2 = psi(U1) ; U3 = psi(U2)
raw("copyb(0, BLS_SGP2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_SG2_U1) -> r11"); raw(f"call zisklib_utf_endomorphism_twist{S}")
raw("copyb(0, BLS_SG2_U1) -> r10"); raw("copyb(0, BLS_SG2_U2) -> r11"); raw(f"call zisklib_utf_endomorphism_twist{S}")
raw("copyb(0, BLS_SG2_U2) -> r10"); raw("copyb(0, BLS_SG2_U3) -> r11"); raw(f"call zisklib_utf_endomorphism_twist{S}")
# XU = scalar_mul_by_abs_x(U3) ; LHS = neg_twist(XU) ; LHS = add_complete_twist(LHS, p)
raw("copyb(0, BLS_SG2_U3) -> r10"); raw("copyb(0, BLS_SG2_XU) -> r11"); raw(f"call zisklib_scalar_mul_by_abs_x_complete_twist{S}")
raw("copyb(0, BLS_SG2_XU) -> r10"); raw("copyb(0, BLS_SG2_LHS) -> r11"); raw(f"call zisklib_neg_twist{S}")
raw("copyb(0, BLS_SG2_LHS) -> r10"); raw("copyb(0, BLS_SGP2) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_SG2_R) -> r12"); raw(f"call zisklib_add_complete_twist{S}")
# return eqn(R, U2, 24)
raw("copyb(0, BLS_SG2_R) -> r10"); raw("copyb(0, BLS_SG2_U2) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret"); w("")

open(_OUT+"/bls12_381/subgroup.zisk","w").write("\n".join(out)+"\n")
print("wrote subgroup.zisk", len(out), "lines; X2DIV3", len(X2DIV3), "XABS", len(XABS))
