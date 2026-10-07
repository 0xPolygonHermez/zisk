#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bn254/fp12.zisk from fp12.rs. Fp12 = Fp6[w]/(w²-v),
# element = 48 u64 = a1‖a2 (each Fp6 = 24 u64). fp6 routines copy inputs before
# writing output => no alias restriction at this level. fp2 leaves keep the
# out!=b rule (only used in frobenius, where in/out buffers are distinct).
out=[]
def w(s=""): out.append(s)

def f6bin(op,dst,x,y):
    w(f"\tcopyb(0, {x}) -> r10"); w(f"\tcopyb(0, {y}) -> r11"); w(f"\tcopyb(0, {dst}) -> r12")
    w(f"\tcall zisklib_{op}_fp6_bn254")
def f6un(op,dst,x):
    w(f"\tcopyb(0, {x}) -> r10"); w(f"\tcopyb(0, {dst}) -> r11")
    w(f"\tcall zisklib_{op}_fp6_bn254")
def mulv6(dst,x):    # dst = x·v  = sparse_mula_fp6(x, [1,0..])
    w(f"\tcopyb(0, {x}) -> r10"); w(f"\tcopyb(0, BN_FP2_ONE) -> r11"); w(f"\tcopyb(0, {dst}) -> r12")
    w("\tcall zisklib_sparse_mula_fp6_bn254")
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
    w(f"\tcopyb(0, {slot}) -> r10"); w(f"\tcopyb(0, F12_PR) -> r11"); w(f"\tcopyb(r11, 8[a + 0]) -> r11")
    if off_words: w(f"\tadd(r11, {off_words*8}) -> r11")
    w(f"\tcopyb(0, {n}) -> r12"); w("\tcall bn254_memcpy")
def save_ptr(slot,reg): w(f"\tcopyb(0, {slot}) -> r5"); w(f"\tcopyb(r5, {reg}) -> 8[a + 0]")

A1,A2,B1,B2="F12_A1","F12_A2","F12_B1","F12_B2"
C1,C2="F12_C1","F12_C2"
T=[None]+[f"F12_T{i}" for i in range(1,7)]

w("; ============================================================================")
w("; bn254/fp12.zisk - degree-12 extension Fp12 = Fp6[w]/(w²-v) for BN254.")
w("; GENERATED from fp12.rs (scripts/legacy/gen_fp12.py). Fp12 element = 48 u64 =")
w("; a1‖a2 (each Fp6 = 24 u64). Non-leaf (call fp6/fp2). Prefix bx12_.")
w("; ============================================================================")
w("")
w("const u64 BN_FP2_ONE[8] = 1, 0, 0, 0, 0, 0, 0, 0")
w("")
w("u64 F12_PA[1] = 0")
w("u64 F12_PB[1] = 0")
w("u64 F12_PR[1] = 0")
for s in ["A1","A2","B1","B2","C1","C2"] + [f"T{i}" for i in range(1,7)]:
    w(f"u64 F12_{s}[24] = "+", ".join(["0"]*24))
w("u64 F12_IN[48] = "+", ".join(["0"]*48))
w("u64 F12_OUT[48] = "+", ".join(["0"]*48))
w("u64 F12_TMP2[8] = 0, 0, 0, 0, 0, 0, 0, 0")
w("u64 F12_ERES[48] = "+", ".join(["0"]*48))
w("u64 F12_E[1] = 0")
w("u64 F12_EA[1] = 0")   # exp: base ptr (square/mul clobber F12_PA, so keep it here)
w("u64 F12_ER[1] = 0")   # exp: result ptr
w("")

# ---- mul_fp12 ----
w("; void zisklib_mul_fp12_bn254(const u64* a, const u64* b, u64* result)  [48,48,48]")
w("zisklib_mul_fp12_bn254:")
w("\tpush r1")
save_ptr("F12_PA","r10"); save_ptr("F12_PB","r11"); save_ptr("F12_PR","r12")
cpin(A1,"F12_PA",0,24); cpin(A2,"F12_PA",24,24); cpin(B1,"F12_PB",0,24); cpin(B2,"F12_PB",24,24)
mul6(T[1],A1,B1)          # a1b1
mul6(T[2],A2,B2)          # a2b2
mulv6(T[3],T[2])          # a2b2·v
add6(C1,T[1],T[3])        # c1 = a1b1 + a2b2v
add6(T[4],A1,A2); add6(T[5],B1,B2)
mul6(C2,T[4],T[5]); sub6(C2,C2,T[1]); sub6(C2,C2,T[2])
cpout(C1,0,24); cpout(C2,24,24)
w("\tpop r1"); w("\tret"); w("")

# ---- sparse_mul_fp12 ----
w("; void zisklib_sparse_mul_fp12_bn254(const u64* a, const u64* b, u64* result)  [48,16,48]")
w("zisklib_sparse_mul_fp12_bn254:")
w("\tpush r1")
save_ptr("F12_PA","r10"); save_ptr("F12_PB","r11"); save_ptr("F12_PR","r12")
cpin(A1,"F12_PA",0,24); cpin(A2,"F12_PA",24,24)
# c1 = sparse_mulc_fp6(a2, b) + a1
w(f"\tcopyb(0, {A2}) -> r10"); w("\tcopyb(0, F12_PB) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11")
w(f"\tcopyb(0, {C1}) -> r12"); w("\tcall zisklib_sparse_mulc_fp6_bn254")
add6(C1,C1,A1)
# c2 = sparse_mulb_fp6(a1, b) + a2
w(f"\tcopyb(0, {A1}) -> r10"); w("\tcopyb(0, F12_PB) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11")
w(f"\tcopyb(0, {C2}) -> r12"); w("\tcall zisklib_sparse_mulb_fp6_bn254")
add6(C2,C2,A2)
cpout(C1,0,24); cpout(C2,24,24)
w("\tpop r1"); w("\tret"); w("")

# ---- square_fp12 ----
w("; void zisklib_square_fp12_bn254(const u64* a, u64* result)  [48,48]")
w("zisklib_square_fp12_bn254:")
w("\tpush r1")
save_ptr("F12_PA","r10"); save_ptr("F12_PR","r11")
cpin(A1,"F12_PA",0,24); cpin(A2,"F12_PA",24,24)
mul6(T[1],A1,A2)          # a1a2
mulv6(T[2],A2)            # a2v
mulv6(T[3],T[1])          # a1a2v
sub6(T[4],A1,A2)          # a1-a2
sub6(T[5],A1,T[2])        # a1-a2v
mul6(C1,T[4],T[5]); add6(C1,C1,T[1]); add6(C1,C1,T[3])
dbl6(C2,T[1])
cpout(C1,0,24); cpout(C2,24,24)
w("\tpop r1"); w("\tret"); w("")

# ---- inv_fp12 ----
w("; void zisklib_inv_fp12_bn254(const u64* a, u64* result)  [48,48]")
w("zisklib_inv_fp12_bn254:")
w("\tpush r1")
save_ptr("F12_PA","r10"); save_ptr("F12_PR","r11")
cpin(A1,"F12_PA",0,24); cpin(A2,"F12_PA",24,24)
sq6(T[1],A1)              # a1²
sq6(T[2],A2)              # a2²
mulv6(T[3],T[2])          # a2²v
sub6(T[4],T[1],T[3])      # a1²-a2²v
inv6(T[5],T[4])           # inv
mul6(C1,A1,T[5])
mul6(C2,A2,T[5]); neg6(C2,C2)
cpout(C1,0,24); cpout(C2,24,24)
w("\tpop r1"); w("\tret"); w("")

# ---- conjugate_fp12 ----
w("; void zisklib_conjugate_fp12_bn254(const u64* a, u64* result)  [48,48]")
w("zisklib_conjugate_fp12_bn254:")
w("\tpush r1")
save_ptr("F12_PA","r10"); save_ptr("F12_PR","r11")
cpin(A1,"F12_PA",0,24); cpin(A2,"F12_PA",24,24)
neg6(C2,A2)
cpout(A1,0,24); cpout(C2,24,24)
w("\tpop r1"); w("\tret"); w("")

# ---- frobenius helpers (component-wise on F12_IN -> F12_OUT) ----
def in_ptr(reg, off):  w(f"\tcopyb(0, F12_IN) -> {reg}"); (w(f"\tadd({reg}, {off*8}) -> {reg}") if off else None)
def out_ptr(reg, off): w(f"\tcopyb(0, F12_OUT) -> {reg}"); (w(f"\tadd({reg}, {off*8}) -> {reg}") if off else None)
# ci = Fp2 component index (0..5); each component is 8 words apart.
def fr_conj_copy(ci):    # out[ci] = conj_fp2(in[ci])
    in_ptr("r10",ci*8); out_ptr("r11",ci*8); w("\tcall zisklib_conjugate_fp2_bn254")
def fr_copy8(ci):        # out[ci] = in[ci]
    in_ptr("r10",ci*8); out_ptr("r11",ci*8); w("\tcopyb(0, 8) -> r12"); w("\tcall bn254_memcpy")
def fr_conj_mul(ci, gamma):  # out[ci] = conj(in[ci]) · gamma  (Fp2 mul)
    in_ptr("r10",ci*8); w("\tcopyb(0, F12_TMP2) -> r11"); w("\tcall zisklib_conjugate_fp2_bn254")
    w("\tcopyb(0, F12_TMP2) -> r10"); w(f"\tcopyb(0, {gamma}) -> r11"); out_ptr("r12",ci*8)
    w("\tcall zisklib_mul_fp2_bn254")
def fr_scalar_mul(ci, gamma4):  # out[ci] = in[ci] · gamma4  (Fp2 by Fp scalar)
    in_ptr("r10",ci*8); w(f"\tcopyb(0, {gamma4}) -> r11"); out_ptr("r12",ci*8)
    w("\tcall zisklib_scalar_mul_fp2_bn254")

def frob_routine(name, kind):
    w(f"; void zisklib_{name}_bn254(const u64* a, u64* result)  [48,48]")
    w(f"zisklib_{name}_bn254:")
    w("\tpush r1")
    save_ptr("F12_PA","r10"); save_ptr("F12_PR","r11")
    cpin("F12_IN","F12_PA",0,48)
    if kind==1:   # frobenius1: conj + gammas 12,14 / 11,13,15
        fr_conj_copy(0); fr_conj_mul(1,"BN_FROBENIUS_GAMMA12"); fr_conj_mul(2,"BN_FROBENIUS_GAMMA14")
        fr_conj_mul(3,"BN_FROBENIUS_GAMMA11"); fr_conj_mul(4,"BN_FROBENIUS_GAMMA13"); fr_conj_mul(5,"BN_FROBENIUS_GAMMA15")
    elif kind==2: # frobenius2: copy + scalar gammas 22,24 / 21,23,25
        fr_copy8(0); fr_scalar_mul(1,"BN_FROBENIUS_GAMMA22"); fr_scalar_mul(2,"BN_FROBENIUS_GAMMA24")
        fr_scalar_mul(3,"BN_FROBENIUS_GAMMA21"); fr_scalar_mul(4,"BN_FROBENIUS_GAMMA23"); fr_scalar_mul(5,"BN_FROBENIUS_GAMMA25")
    else:         # frobenius3: conj + gammas 32,34 / 31,33,35
        fr_conj_copy(0); fr_conj_mul(1,"BN_FROBENIUS_GAMMA32"); fr_conj_mul(2,"BN_FROBENIUS_GAMMA34")
        fr_conj_mul(3,"BN_FROBENIUS_GAMMA31"); fr_conj_mul(4,"BN_FROBENIUS_GAMMA33"); fr_conj_mul(5,"BN_FROBENIUS_GAMMA35")
    cpout("F12_OUT",0,48)
    w("\tpop r1"); w("\tret"); w("")

frob_routine("frobenius1_fp12",1)
frob_routine("frobenius2_fp12",2)
frob_routine("frobenius3_fp12",3)

# ---- exp_fp12(e:u64, a[48]) -> result[48] ----
w("; void zisklib_exp_fp12_bn254(u64 e, const u64* a, u64* result)  [r10=e, r11=a, r12=result]")
w("zisklib_exp_fp12_bn254:")
w("\tpush r1")
w("\tcopyb(0, F12_E) -> r5"); w("\tcopyb(r5, r10) -> 8[a + 0]")      # save e
save_ptr("F12_EA","r11"); save_ptr("F12_ER","r12")
# e == 0 -> one ; e == 1 -> a  (also a==0/one handled by returning a which is correct for e>=1; ziskos special-cases a but result equals a^e anyway for these)
w("\tcopyb(0, F12_E) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r6")
w("\teq(r6, 0), j(bx12_one)")
w("\teq(r6, 1), j(bx12_copya)")
# ERES = a
w(f"\tcopyb(0, F12_EA) -> r10"); w("\tcopyb(r10, 8[a + 0]) -> r10"); w("\tcopyb(0, F12_ERES) -> r11"); w("\tcopyb(0, 48) -> r12"); w("\tcall bn254_memcpy")
# find MSB of e (scan 63..0)
w("\tcopyb(0, 63) -> r13")
w("bx12_msb:")
w("\tcopyb(0, F12_E) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r5"); w("\tsrl(r5, r13) -> r5"); w("\tand(r5, 1) -> r5")
w("\tltu(0, r5), j(bx12_loopstart)")
w("\tsub(r13, 1) -> r13"); w("\tjump(bx12_msb)")
w("bx12_loopstart:")
w("\teq(r13, 0), j(bx12_writeres)")   # e==1 (MSB at 0) already handled; guard
w("\tsub(r13, 1) -> r13")
w("bx12_loop:")
# ERES = ERES²
w("\tcopyb(0, F12_ERES) -> r10"); w("\tcopyb(0, F12_ERES) -> r11"); w("\tcall zisklib_square_fp12_bn254")
# bit r13 of e
w("\tcopyb(0, F12_E) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r5"); w("\tsrl(r5, r13) -> r5"); w("\tand(r5, 1) -> r5")
w("\teq(r5, 0), j(bx12_nomul)")
w("\tcopyb(0, F12_ERES) -> r10"); w("\tcopyb(0, F12_EA) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11"); w("\tcopyb(0, F12_ERES) -> r12"); w("\tcall zisklib_mul_fp12_bn254")
w("bx12_nomul:")
w("\teq(r13, 0), j(bx12_writeres)")
w("\tsub(r13, 1) -> r13"); w("\tjump(bx12_loop)")
w("bx12_writeres:")
w("\tcopyb(0, F12_ERES) -> r10"); w("\tcopyb(0, F12_ER) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11"); w("\tcopyb(0, 48) -> r12"); w("\tcall bn254_memcpy")
w("\tpop r1"); w("\tret")
w("bx12_copya:")
w(f"\tcopyb(0, F12_EA) -> r10"); w("\tcopyb(r10, 8[a + 0]) -> r10"); w("\tcopyb(0, F12_ER) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11"); w("\tcopyb(0, 48) -> r12"); w("\tcall bn254_memcpy")
w("\tpop r1"); w("\tret")
w("bx12_one:")
w("\tcopyb(0, F12_ER) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11")
for i in range(48):
    w(f"\tcopyb(r11, {1 if i==0 else 0}) -> 8[a + {i*8}]")
w("\tpop r1"); w("\tret"); w("")

open(_OUT+"/bn254/fp12.zisk","w").write("\n".join(out)+"\n")
print("wrote fp12.zisk,", len(out), "lines")
