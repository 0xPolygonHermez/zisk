#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/fp12.zisk from bls12_381/fp12.rs.
# Fp12 = Fp6[w]/(w²-v), element = 72 u64 = a1‖a2 (each Fp6 = 36 u64).
# mul/square/inv/conjugate MATCH bn254; sparse_mul_fp12 and frobenius are
# BLS-specific (only frob1+frob2, frob1 a13-component uses scalar_mul).
out=[]
def w(s=""): out.append(s)
S="_bls12_381"
def f6bin(op,dst,x,y):
    w(f"\tcopyb(0, {x}) -> r10"); w(f"\tcopyb(0, {y}) -> r11"); w(f"\tcopyb(0, {dst}) -> r12"); w(f"\tcall zisklib_{op}_fp6{S}")
def f6un(op,dst,x):
    w(f"\tcopyb(0, {x}) -> r10"); w(f"\tcopyb(0, {dst}) -> r11"); w(f"\tcall zisklib_{op}_fp6{S}")
def mulv6(dst,x):
    w(f"\tcopyb(0, {x}) -> r10"); w("\tcopyb(0, BLS_F2_ONE) -> r11"); w(f"\tcopyb(0, {dst}) -> r12"); w(f"\tcall zisklib_sparse_mula_fp6{S}")
def add6(d,x,y): f6bin("add",d,x,y)
def sub6(d,x,y): f6bin("sub",d,x,y)
def mul6(d,x,y): f6bin("mul",d,x,y)
def sq6(d,x): f6un("square",d,x)
def dbl6(d,x): f6un("dbl",d,x)
def neg6(d,x): f6un("neg",d,x)
def inv6(d,x): f6un("inv",d,x)
def cpin(slot, ptrslot, off_words, n):
    w(f"\tcopyb(0, {ptrslot}) -> r10"); w(f"\tcopyb(r10, 8[a + 0]) -> r10")
    if off_words: w(f"\tadd(r10, {off_words*8}) -> r10")
    w(f"\tcopyb(0, {slot}) -> r11"); w(f"\tcopyb(0, {n}) -> r12"); w("\tcall bn254_memcpy")
def cpout(slot, off_words, n):
    w(f"\tcopyb(0, {slot}) -> r10"); w("\tcopyb(0, BLS_F12_PR) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11")
    if off_words: w(f"\tadd(r11, {off_words*8}) -> r11")
    w(f"\tcopyb(0, {n}) -> r12"); w("\tcall bn254_memcpy")
def save_ptr(slot,reg): w(f"\tcopyb(0, {slot}) -> r5"); w(f"\tcopyb(r5, {reg}) -> 8[a + 0]")

A1,A2,B1,B2="BLS_F12_A1","BLS_F12_A2","BLS_F12_B1","BLS_F12_B2"
C1,C2="BLS_F12_C1","BLS_F12_C2"
T=[None]+[f"BLS_F12_T{i}" for i in range(1,7)]

w("; ============================================================================")
w("; bls12_381/fp12.zisk - degree-12 extension Fp12 = Fp6[w]/(w²-v). GENERATED")
w("; from bls12_381/fp12.rs (scripts/legacy/gen_bls_fp12.py). Fp12 = 72 u64 = a1‖a2")
w("; (each Fp6 = 36 u64). Only frobenius1 + frobenius2 (no frob3). Prefix BLS_F12_.")
w("; ============================================================================")
w("")
w("u64 BLS_F12_PA[1] = 0"); w("u64 BLS_F12_PB[1] = 0"); w("u64 BLS_F12_PR[1] = 0")
for s in ["A1","A2","B1","B2","C1","C2"] + [f"T{i}" for i in range(1,7)]:
    w(f"u64 BLS_F12_{s}[36] = "+", ".join(["0"]*36))
w("u64 BLS_F12_IN[72] = "+", ".join(["0"]*72))
w("u64 BLS_F12_OUT[72] = "+", ".join(["0"]*72))
w("u64 BLS_F12_TMP2[12] = "+", ".join(["0"]*12))
w("u64 BLS_F12_SC1[24] = "+", ".join(["0"]*24))
w("u64 BLS_F12_SC2[24] = "+", ".join(["0"]*24))
w("u64 BLS_F12_ERES[72] = "+", ".join(["0"]*72))
w("u64 BLS_F12_E[1] = 0"); w("u64 BLS_F12_EA[1] = 0"); w("u64 BLS_F12_ER[1] = 0")
w("")

# ---- mul_fp12 ----
w("; void zisklib_mul_fp12_bls12_381(const u64* a, const u64* b, u64* result)  [72,72,72]")
w("zisklib_mul_fp12_bls12_381:")
w("\tpush r1")
save_ptr("BLS_F12_PA","r10"); save_ptr("BLS_F12_PB","r11"); save_ptr("BLS_F12_PR","r12")
cpin(A1,"BLS_F12_PA",0,36); cpin(A2,"BLS_F12_PA",36,36); cpin(B1,"BLS_F12_PB",0,36); cpin(B2,"BLS_F12_PB",36,36)
mul6(T[1],A1,B1); mul6(T[2],A2,B2); mulv6(T[3],T[2]); add6(C1,T[1],T[3])
add6(T[4],A1,A2); add6(T[5],B1,B2); mul6(C2,T[4],T[5]); sub6(C2,C2,T[1]); sub6(C2,C2,T[2])
cpout(C1,0,36); cpout(C2,36,36)
w("\tpop r1"); w("\tret"); w("")

# ---- sparse_mul_fp12 (BLS-specific) ----
w("; void zisklib_sparse_mul_fp12_bls12_381(const u64* a, const u64* b, u64* result)  [72,24,72]")
w("; b = b22‖b23 (two Fp2). c1 = sparse_mulc(a2,[b23·ξ‖b22]) + a1 ; c2 = sparse_mulb(a1,[b22‖b23]) + a2")
w("zisklib_sparse_mul_fp12_bls12_381:")
w("\tpush r1")
save_ptr("BLS_F12_PA","r10"); save_ptr("BLS_F12_PB","r11"); save_ptr("BLS_F12_PR","r12")
cpin(A1,"BLS_F12_PA",0,36); cpin(A2,"BLS_F12_PA",36,36)
# b23u = mul_fp2(ξ, b23) -> SC1[0..12]
w("\tcopyb(0, BLS_XI) -> r10"); w("\tcopyb(0, BLS_F12_PB) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11"); w("\tadd(r11, 96) -> r11")
w("\tcopyb(0, BLS_F12_SC1) -> r12"); w(f"\tcall zisklib_mul_fp2{S}")
# SC1[12..24] = b22 (PB+0)
w("\tcopyb(0, BLS_F12_PB) -> r10"); w("\tcopyb(r10, 8[a + 0]) -> r10"); w("\tcopyb(0, BLS_F12_SC1) -> r11"); w("\tadd(r11, 96) -> r11"); w("\tcopyb(0, 12) -> r12"); w("\tcall bn254_memcpy")
# SC2[0..12] = b22 ; SC2[12..24] = b23
w("\tcopyb(0, BLS_F12_PB) -> r10"); w("\tcopyb(r10, 8[a + 0]) -> r10"); w("\tcopyb(0, BLS_F12_SC2) -> r11"); w("\tcopyb(0, 12) -> r12"); w("\tcall bn254_memcpy")
w("\tcopyb(0, BLS_F12_PB) -> r10"); w("\tcopyb(r10, 8[a + 0]) -> r10"); w("\tadd(r10, 96) -> r10"); w("\tcopyb(0, BLS_F12_SC2) -> r11"); w("\tadd(r11, 96) -> r11"); w("\tcopyb(0, 12) -> r12"); w("\tcall bn254_memcpy")
# c1 = sparse_mulc(a2, SC1) + a1
w(f"\tcopyb(0, {A2}) -> r10"); w("\tcopyb(0, BLS_F12_SC1) -> r11"); w(f"\tcopyb(0, {C1}) -> r12"); w(f"\tcall zisklib_sparse_mulc_fp6{S}")
add6(C1,C1,A1)
# c2 = sparse_mulb(a1, SC2) + a2
w(f"\tcopyb(0, {A1}) -> r10"); w("\tcopyb(0, BLS_F12_SC2) -> r11"); w(f"\tcopyb(0, {C2}) -> r12"); w(f"\tcall zisklib_sparse_mulb_fp6{S}")
add6(C2,C2,A2)
cpout(C1,0,36); cpout(C2,36,36)
w("\tpop r1"); w("\tret"); w("")

# ---- square_fp12 ----
w("; void zisklib_square_fp12_bls12_381(const u64* a, u64* result)  [72,72]")
w("zisklib_square_fp12_bls12_381:")
w("\tpush r1")
save_ptr("BLS_F12_PA","r10"); save_ptr("BLS_F12_PR","r11")
cpin(A1,"BLS_F12_PA",0,36); cpin(A2,"BLS_F12_PA",36,36)
mul6(T[1],A1,A2); mulv6(T[2],A2); mulv6(T[3],T[1]); sub6(T[4],A1,A2); sub6(T[5],A1,T[2])
mul6(C1,T[4],T[5]); add6(C1,C1,T[1]); add6(C1,C1,T[3]); dbl6(C2,T[1])
cpout(C1,0,36); cpout(C2,36,36)
w("\tpop r1"); w("\tret"); w("")

# ---- inv_fp12 ----
w("; void zisklib_inv_fp12_bls12_381(const u64* a, u64* result)  [72,72]")
w("zisklib_inv_fp12_bls12_381:")
w("\tpush r1")
save_ptr("BLS_F12_PA","r10"); save_ptr("BLS_F12_PR","r11")
cpin(A1,"BLS_F12_PA",0,36); cpin(A2,"BLS_F12_PA",36,36)
sq6(T[1],A1); sq6(T[2],A2); mulv6(T[3],T[2]); sub6(T[4],T[1],T[3]); inv6(T[5],T[4])
mul6(C1,A1,T[5]); mul6(C2,A2,T[5]); neg6(C2,C2)
cpout(C1,0,36); cpout(C2,36,36)
w("\tpop r1"); w("\tret"); w("")

# ---- conjugate_fp12 ----
w("; void zisklib_conjugate_fp12_bls12_381(const u64* a, u64* result)  [72,72]")
w("zisklib_conjugate_fp12_bls12_381:")
w("\tpush r1")
save_ptr("BLS_F12_PA","r10"); save_ptr("BLS_F12_PR","r11")
cpin(A1,"BLS_F12_PA",0,36); cpin(A2,"BLS_F12_PA",36,36)
neg6(C2,A2)
cpout(A1,0,36); cpout(C2,36,36)
w("\tpop r1"); w("\tret"); w("")

# ---- frobenius (component-wise; Fp2 = 12 u64, components 12 words apart) ----
def in_ptr(reg, off):  w(f"\tcopyb(0, BLS_F12_IN) -> {reg}"); (w(f"\tadd({reg}, {off*8}) -> {reg}") if off else None)
def out_ptr(reg, off): w(f"\tcopyb(0, BLS_F12_OUT) -> {reg}"); (w(f"\tadd({reg}, {off*8}) -> {reg}") if off else None)
def fr_conj_copy(ci):
    in_ptr("r10",ci*12); out_ptr("r11",ci*12); w(f"\tcall zisklib_conjugate_fp2{S}")
def fr_copy(ci):
    in_ptr("r10",ci*12); out_ptr("r11",ci*12); w("\tcopyb(0, 12) -> r12"); w("\tcall bn254_memcpy")
def fr_conj_mul(ci, gamma):    # out[ci] = conj(in[ci]) · gamma  (Fp2 mul)
    in_ptr("r10",ci*12); w("\tcopyb(0, BLS_F12_TMP2) -> r11"); w(f"\tcall zisklib_conjugate_fp2{S}")
    w("\tcopyb(0, BLS_F12_TMP2) -> r10"); w(f"\tcopyb(0, {gamma}) -> r11"); out_ptr("r12",ci*12); w(f"\tcall zisklib_mul_fp2{S}")
def fr_conj_scalarmul(ci, gamma):  # out[ci] = conj(in[ci]) · gamma  (Fp2 by Fp scalar)
    in_ptr("r10",ci*12); w("\tcopyb(0, BLS_F12_TMP2) -> r11"); w(f"\tcall zisklib_conjugate_fp2{S}")
    w("\tcopyb(0, BLS_F12_TMP2) -> r10"); w(f"\tcopyb(0, {gamma}) -> r11"); out_ptr("r12",ci*12); w(f"\tcall zisklib_scalar_mul_fp2{S}")
def fr_scalar_mul(ci, gamma):  # out[ci] = in[ci] · gamma  (Fp2 by Fp scalar)
    in_ptr("r10",ci*12); w(f"\tcopyb(0, {gamma}) -> r11"); out_ptr("r12",ci*12); w(f"\tcall zisklib_scalar_mul_fp2{S}")

def frob_routine(name, kind):
    w(f"; void zisklib_{name}_bls12_381(const u64* a, u64* result)  [72,72]")
    w(f"zisklib_{name}_bls12_381:")
    w("\tpush r1")
    save_ptr("BLS_F12_PA","r10"); save_ptr("BLS_F12_PR","r11")
    cpin("BLS_F12_IN","BLS_F12_PA",0,72)
    if kind==1:   # conj + gammas 12(mul),14(scalar) / 11,13,15(mul)
        fr_conj_copy(0); fr_conj_mul(1,"BLS_FROBENIUS_GAMMA12"); fr_conj_scalarmul(2,"BLS_FROBENIUS_GAMMA14")
        fr_conj_mul(3,"BLS_FROBENIUS_GAMMA11"); fr_conj_mul(4,"BLS_FROBENIUS_GAMMA13"); fr_conj_mul(5,"BLS_FROBENIUS_GAMMA15")
    else:         # frob2: copy + scalar gammas 22,24 / 21,23,25
        fr_copy(0); fr_scalar_mul(1,"BLS_FROBENIUS_GAMMA22"); fr_scalar_mul(2,"BLS_FROBENIUS_GAMMA24")
        fr_scalar_mul(3,"BLS_FROBENIUS_GAMMA21"); fr_scalar_mul(4,"BLS_FROBENIUS_GAMMA23"); fr_scalar_mul(5,"BLS_FROBENIUS_GAMMA25")
    cpout("BLS_F12_OUT",0,72)
    w("\tpop r1"); w("\tret"); w("")

frob_routine("frobenius1_fp12",1)
frob_routine("frobenius2_fp12",2)

# ---- exp_fp12(e:u64, a[72]) -> result[72] ----
w("; void zisklib_exp_fp12_bls12_381(u64 e, const u64* a, u64* result)  [r10=e, r11=a, r12=result]")
w("zisklib_exp_fp12_bls12_381:")
w("\tpush r1")
w("\tcopyb(0, BLS_F12_E) -> r5"); w("\tcopyb(r5, r10) -> 8[a + 0]")
save_ptr("BLS_F12_EA","r11"); save_ptr("BLS_F12_ER","r12")
w("\tcopyb(0, BLS_F12_E) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r6")
w("\teq(r6, 0), j(lx12_one)")
w("\teq(r6, 1), j(lx12_copya)")
w("\tcopyb(0, BLS_F12_EA) -> r10"); w("\tcopyb(r10, 8[a + 0]) -> r10"); w("\tcopyb(0, BLS_F12_ERES) -> r11"); w("\tcopyb(0, 72) -> r12"); w("\tcall bn254_memcpy")
w("\tcopyb(0, 63) -> r13")
w("lx12_msb:")
w("\tcopyb(0, BLS_F12_E) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r5"); w("\tsrl(r5, r13) -> r5"); w("\tand(r5, 1) -> r5")
w("\tltu(0, r5), j(lx12_loopstart)")
w("\tsub(r13, 1) -> r13"); w("\tjump(lx12_msb)")
w("lx12_loopstart:")
w("\teq(r13, 0), j(lx12_writeres)")
w("\tsub(r13, 1) -> r13")
w("lx12_loop:")
w("\tcopyb(0, BLS_F12_ERES) -> r10"); w("\tcopyb(0, BLS_F12_ERES) -> r11"); w(f"\tcall zisklib_square_fp12{S}")
w("\tcopyb(0, BLS_F12_E) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r5"); w("\tsrl(r5, r13) -> r5"); w("\tand(r5, 1) -> r5")
w("\teq(r5, 0), j(lx12_nomul)")
w("\tcopyb(0, BLS_F12_ERES) -> r10"); w("\tcopyb(0, BLS_F12_EA) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11"); w("\tcopyb(0, BLS_F12_ERES) -> r12"); w(f"\tcall zisklib_mul_fp12{S}")
w("lx12_nomul:")
w("\teq(r13, 0), j(lx12_writeres)")
w("\tsub(r13, 1) -> r13"); w("\tjump(lx12_loop)")
w("lx12_writeres:")
w("\tcopyb(0, BLS_F12_ERES) -> r10"); w("\tcopyb(0, BLS_F12_ER) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11"); w("\tcopyb(0, 72) -> r12"); w("\tcall bn254_memcpy")
w("\tpop r1"); w("\tret")
w("lx12_copya:")
w("\tcopyb(0, BLS_F12_EA) -> r10"); w("\tcopyb(r10, 8[a + 0]) -> r10"); w("\tcopyb(0, BLS_F12_ER) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11"); w("\tcopyb(0, 72) -> r12"); w("\tcall bn254_memcpy")
w("\tpop r1"); w("\tret")
w("lx12_one:")
w("\tcopyb(0, BLS_F12_ER) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11")
for i in range(72):
    w(f"\tcopyb(r11, {1 if i==0 else 0}) -> 8[a + {i*8}]")
w("\tpop r1"); w("\tret"); w("")

open(_OUT+"/bls12_381/fp12.zisk","w").write("\n".join(out)+"\n")
print("wrote bls fp12.zisk,", len(out), "lines")
