#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/fp6.zisk from the exact BLS fp6.rs formulas.
# Fp6 element = 36 u64 = a1‖a2‖a3 (each Fp2 = 12 u64). Tower ξ = 1+u (BLS_XI).
# fp2 ops copy inputs first (alias rule: OUTPUT must not alias operand b -> asserted).
out=[]
def w(s=""): out.append(s)
def binop(op,dst,x,y):
    assert dst!=y, f"{op} {dst}=f({x},{y}): dst aliases y"
    w(f"\tcopyb(0, {x}) -> r10"); w(f"\tcopyb(0, {y}) -> r11"); w(f"\tcopyb(0, {dst}) -> r12")
    w(f"\tcall zisklib_{op}_fp2_bls12_381")
def unop(op,dst,x):
    w(f"\tcopyb(0, {x}) -> r10"); w(f"\tcopyb(0, {dst}) -> r11"); w(f"\tcall zisklib_{op}_fp2_bls12_381")
def add(d,x,y): binop("add",d,x,y)
def sub(d,x,y): binop("sub",d,x,y)
def mul(d,x,y): binop("mul",d,x,y)
def sq(d,x): unop("square",d,x)
def dbl(d,x): unop("dbl",d,x)
def invop(d,x): unop("inv",d,x)

def cpin(slot, ptrslot, off_words):
    w(f"\tcopyb(0, {ptrslot}) -> r10"); w(f"\tcopyb(r10, 8[a + 0]) -> r10")
    if off_words: w(f"\tadd(r10, {off_words*8}) -> r10")
    w(f"\tcopyb(0, {slot}) -> r11"); w("\tcopyb(0, 12) -> r12"); w("\tcall bn254_memcpy")
def cpout(slot, off_words):
    w(f"\tcopyb(0, {slot}) -> r10"); w("\tcopyb(0, BLS_F6_PR) -> r11"); w("\tcopyb(r11, 8[a + 0]) -> r11")
    if off_words: w(f"\tadd(r11, {off_words*8}) -> r11")
    w("\tcopyb(0, 12) -> r12"); w("\tcall bn254_memcpy")
def save_ptr(slot,reg): w(f"\tcopyb(0, {slot}) -> r5"); w(f"\tcopyb(r5, {reg}) -> 8[a + 0]")

w("; ============================================================================")
w("; bls12_381/fp6.zisk - degree-6 extension Fp6 = Fp2[v]/(v³-(1+u)). GENERATED")
w("; from bls12_381/fp6.rs (scripts/legacy/gen_bls_fp6.py). Fp6 = 36 u64 = a1‖a2‖a3")
w("; (each Fp2 = 12 u64). Non-leaf (call fp2). Prefix BLS_F6_. ξ = 1+u = BLS_XI.")
w("; ============================================================================")
w("")
w("u64 BLS_F6_PA[1] = 0")
w("u64 BLS_F6_PB[1] = 0")
w("u64 BLS_F6_PR[1] = 0")
for s in ["A1","A2","A3","B1","B2","B3","C1","C2","C3"]:
    w(f"u64 BLS_F6_{s}[12] = "+", ".join(["0"]*12))
for i in range(1,11):
    w(f"u64 BLS_F6_T{i}[12] = "+", ".join(["0"]*12))
w("")
A1,A2,A3="BLS_F6_A1","BLS_F6_A2","BLS_F6_A3"
B1,B2,B3="BLS_F6_B1","BLS_F6_B2","BLS_F6_B3"
C1,C2,C3="BLS_F6_C1","BLS_F6_C2","BLS_F6_C3"
T=[None]+[f"BLS_F6_T{i}" for i in range(1,11)]
XI="BLS_XI"

def component_routine(name, op, binary):
    w(f"; void zisklib_{name}_bls12_381(const u64* a, {'const u64* b, ' if binary else ''}u64* result)")
    w(f"zisklib_{name}_bls12_381:")
    w("\tpush r1")
    save_ptr("BLS_F6_PA","r10")
    if binary: save_ptr("BLS_F6_PB","r11"); save_ptr("BLS_F6_PR","r12")
    else:      save_ptr("BLS_F6_PR","r11")
    for i in range(3):
        off=i*12
        w("\tcopyb(0, BLS_F6_PA) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r10")
        if off: w(f"\tadd(r10, {off*8}) -> r10")
        if binary:
            w("\tcopyb(0, BLS_F6_PB) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r11")
            if off: w(f"\tadd(r11, {off*8}) -> r11")
            w("\tcopyb(0, BLS_F6_PR) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r12")
            if off: w(f"\tadd(r12, {off*8}) -> r12")
        else:
            w("\tcopyb(0, BLS_F6_PR) -> r5"); w("\tcopyb(r5, 8[a + 0]) -> r11")
            if off: w(f"\tadd(r11, {off*8}) -> r11")
        w(f"\tcall zisklib_{op}_fp2_bls12_381")
    w("\tpop r1"); w("\tret"); w("")

component_routine("add_fp6","add",True)
component_routine("sub_fp6","sub",True)
component_routine("dbl_fp6","dbl",False)
component_routine("neg_fp6","neg",False)

def open_routine(sig, name, copy_b=None):
    w(f"; {sig}")
    w(f"zisklib_{name}_bls12_381:")
    w("\tpush r1")
    save_ptr("BLS_F6_PA","r10")
    if copy_b is not None: save_ptr("BLS_F6_PB","r11"); save_ptr("BLS_F6_PR","r12")
    else: save_ptr("BLS_F6_PR","r11")
    cpin(A1,"BLS_F6_PA",0); cpin(A2,"BLS_F6_PA",12); cpin(A3,"BLS_F6_PA",24)
    if copy_b=="fp6":   cpin(B1,"BLS_F6_PB",0); cpin(B2,"BLS_F6_PB",12); cpin(B3,"BLS_F6_PB",24)
    elif copy_b=="fp2": cpin(B1,"BLS_F6_PB",0)
    elif copy_b=="fp2x2": cpin(B1,"BLS_F6_PB",0); cpin(B2,"BLS_F6_PB",12)
def close_routine():
    cpout(C1,0); cpout(C2,12); cpout(C3,24)
    w("\tpop r1"); w("\tret"); w("")

# ---- mul_fp6 (schoolbook) ----
open_routine("void zisklib_mul_fp6_bls12_381(const u64* a, const u64* b, u64* result)  [36,36,36]",
             "mul_fp6", "fp6")
# c1 = a1b1 + (a2b3+a3b2)·ξ
mul(T[1],A2,B3); mul(T[2],A3,B2); add(T[1],T[1],T[2]); mul(T[1],T[1],XI); mul(C1,A1,B1); add(C1,C1,T[1])
# c2 = a1b2 + a2b1 + a3b3·ξ
mul(T[1],A3,B3); mul(T[1],T[1],XI); mul(C2,A1,B2); add(C2,C2,T[1]); mul(T[1],A2,B1); add(C2,C2,T[1])
# c3 = a1b3 + a2b2 + a3b1
mul(C3,A1,B3); mul(T[1],A2,B2); add(C3,C3,T[1]); mul(T[1],A3,B1); add(C3,C3,T[1])
close_routine()

# ---- square_fp6 ----
open_routine("void zisklib_square_fp6_bls12_381(const u64* a, u64* result)  [36,36]",
             "square_fp6", None)
# c1 = a1² + 2·a2·a3·ξ
sq(C1,A1); mul(T[1],A2,A3); dbl(T[1],T[1]); mul(T[1],T[1],XI); add(C1,C1,T[1])
# c2 = a3²·ξ + 2·a1·a2
sq(C2,A3); mul(C2,C2,XI); mul(T[1],A1,A2); dbl(T[1],T[1]); add(C2,C2,T[1])
# c3 = a2² + 2·a1·a3
sq(C3,A2); mul(T[1],A1,A3); dbl(T[1],T[1]); add(C3,C3,T[1])
close_routine()

# ---- inv_fp6 (same shape as bn254) ----
open_routine("void zisklib_inv_fp6_bls12_381(const u64* a, u64* result)  [36,36]",
             "inv_fp6", None)
sq(T[1],A1); sq(T[2],A2); sq(T[3],A3)
mul(T[4],A1,A2); mul(T[5],A1,A3); mul(T[6],A2,A3)
mul(T[7],T[6],XI); sub(C1,T[1],T[7])      # c1mid = a1²-ξ·a2a3
mul(T[7],T[3],XI); sub(C2,T[7],T[4])      # c2mid = ξ·a3²-a1a2
sub(C3,T[2],T[5])                          # c3mid = a2²-a1a3
mul(T[8],A1,C1)                            # im
mul(T[9],A3,C2); mul(T[10],A2,C3); add(T[9],T[9],T[10]); mul(T[9],T[9],XI); add(T[9],T[9],T[8]); invop(T[9],T[9])
mul(C1,C1,T[9]); mul(C2,C2,T[9]); mul(C3,C3,T[9])
close_routine()

# ---- sparse_mula: b = b2·v (b2 in Fp2 [12]) -> B1=b2 ----
open_routine("void zisklib_sparse_mula_fp6_bls12_381(const u64* a, const u64* b2, u64* result)  [36,12,36]",
             "sparse_mula_fp6", "fp2")
mul(C1,B1,A3); mul(C1,C1,XI)               # c1 = a3·b2·ξ
mul(C2,B1,A1)                              # c2 = a1·b2
mul(C3,B1,A2)                              # c3 = a2·b2
close_routine()

# ---- sparse_mulb: b = b2·v + b3·v² (b[24] = b2‖b3) -> B1=b2, B2=b3 ----
open_routine("void zisklib_sparse_mulb_fp6_bls12_381(const u64* a, const u64* b, u64* result)  [36,24,36]",
             "sparse_mulb_fp6", "fp2x2")
# c1 = (a2·b3 + a3·b2)·ξ
mul(C1,A2,B2); mul(T[1],A3,B1); add(C1,C1,T[1]); mul(C1,C1,XI)
# c2 = a1·b2 + a3·b3·ξ
mul(C2,A3,B2); mul(C2,C2,XI); mul(T[1],A1,B1); add(C2,C2,T[1])
# c3 = a1·b3 + a2·b2
mul(C3,A1,B2); mul(T[1],A2,B1); add(C3,C3,T[1])
close_routine()

# ---- sparse_mulc: b = b1 + b3·v² (b[24] = b1‖b3) -> B1=b1, B2=b3 ----
open_routine("void zisklib_sparse_mulc_fp6_bls12_381(const u64* a, const u64* b, u64* result)  [36,24,36]",
             "sparse_mulc_fp6", "fp2x2")
# c1 = a1·b1 + a2·b3·ξ
mul(T[1],A2,B2); mul(T[1],T[1],XI); mul(C1,A1,B1); add(C1,C1,T[1])
# c2 = a2·b1 + a3·b3·ξ
mul(T[1],A3,B2); mul(T[1],T[1],XI); mul(C2,A2,B1); add(C2,C2,T[1])
# c3 = a1·b3 + a3·b1
mul(C3,A1,B2); mul(T[1],A3,B1); add(C3,C3,T[1])
close_routine()

open(_OUT+"/bls12_381/fp6.zisk","w").write("\n".join(out)+"\n")
print("wrote bls fp6.zisk,", len(out), "lines")
