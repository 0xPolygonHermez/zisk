#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bn254/fp6.zisk from the exact Rust formulas in fp6.rs.
# Fp6 element = 24 u64 = a1‖a2‖a3 (each Fp2 = 8 u64). ops call the fp2 leaves.
# Alias rule of the fp2 leaves: for add/sub/mul the OUTPUT must not alias operand b
# (they copy a->out first, then op with b in place). Enforced by asserts below.

out = []
def w(s=""): out.append(s)

# --- primitive fp2-op call emitters (operands are STATIC slot label names) ---
def binop(op, dst, x, y):
    assert dst != y, f"{op} {dst}=f({x},{y}): dst aliases y (forbidden)"
    w(f"\tcopyb(0, {x}) -> r10")
    w(f"\tcopyb(0, {y}) -> r11")
    w(f"\tcopyb(0, {dst}) -> r12")
    w(f"\tcall zisklib_{op}_fp2_bn254")
def unop(op, dst, x):
    w(f"\tcopyb(0, {x}) -> r10")
    w(f"\tcopyb(0, {dst}) -> r11")
    w(f"\tcall zisklib_{op}_fp2_bn254")

def add(d,x,y): binop("add", d, x, y)
def sub(d,x,y): binop("sub", d, x, y)
def mul(d,x,y): binop("mul", d, x, y)
def sq(d,x):    unop("square", d, x)
def dbl(d,x):   unop("dbl", d, x)
def negop(d,x): unop("neg", d, x)
def invop(d,x): unop("inv", d, x)

# copy component `off_words` of the saved base pointer PTR into slot (8 u64)
def cpin(slot, ptrslot, off_words):
    w(f"\tcopyb(0, {ptrslot}) -> r10")
    w(f"\tcopyb(r10, 8[a + 0]) -> r10")
    if off_words: w(f"\tadd(r10, {off_words*8}) -> r10")
    w(f"\tcopyb(0, {slot}) -> r11")
    w(f"\tcopyb(0, 8) -> r12")
    w(f"\tcall bn254_memcpy")
def cpout(slot, off_words):
    w(f"\tcopyb(0, {slot}) -> r10")
    w(f"\tcopyb(0, F6_PR) -> r11")
    w(f"\tcopyb(r11, 8[a + 0]) -> r11")
    if off_words: w(f"\tadd(r11, {off_words*8}) -> r11")
    w(f"\tcopyb(0, 8) -> r12")
    w(f"\tcall bn254_memcpy")

def save_ptr(slot, reg):
    w(f"\tcopyb(0, {slot}) -> r5")
    w(f"\tcopyb(r5, {reg}) -> 8[a + 0]")

# --- header ---
w("; ============================================================================")
w("; bn254/fp6.zisk - degree-6 extension Fp6 = Fp2[v]/(v³-(9+u)) for BN254.")
w("; GENERATED from fp6.rs formulas (see scripts/legacy/gen_fp6.py). Fp6 element = 24")
w("; u64 = a1‖a2‖a3 (each Fp2 = 8 u64). All routines are non-leaf (call the fp2")
w("; leaves) and share the static scratch pool below (no fp6 routine calls another")
w("; fp6 routine, so sharing is safe). Prefix `f6_`.")
w("; ============================================================================")
w("")
w("const u64 BN_XI[8] = 9, 0, 0, 0, 1, 0, 0, 0")   # ξ = 9 + u
w("")
w("u64 F6_PA[1] = 0")
w("u64 F6_PB[1] = 0")
w("u64 F6_PR[1] = 0")
for s in ["A1","A2","A3","B1","B2","B3","C1","C2","C3"]:
    w(f"u64 F6_{s}[8] = 0, 0, 0, 0, 0, 0, 0, 0")
for i in range(1,11):
    w(f"u64 F6_T{i}[8] = 0, 0, 0, 0, 0, 0, 0, 0")
w("")

A1,A2,A3="F6_A1","F6_A2","F6_A3"
B1,B2,B3="F6_B1","F6_B2","F6_B3"
C1,C2,C3="F6_C1","F6_C2","F6_C3"
T=[None]+[f"F6_T{i}" for i in range(1,11)]
XI="BN_XI"

# ---- component-wise routines (add/sub/dbl/neg): offset-based, no copy-in ----
def component_routine(name, op, binary):
    w(f"; void zisklib_{name}_bn254(const u64* a, {'const u64* b, ' if binary else ''}u64* result)")
    w(f"zisklib_{name}_bn254:")
    w("\tpush r1")
    save_ptr("F6_PA","r10")
    if binary: save_ptr("F6_PB","r11"); save_ptr("F6_PR","r12")
    else:      save_ptr("F6_PR","r11")
    for i in range(3):
        off=i*8
        w(f"\tcopyb(0, F6_PA) -> r5")
        w(f"\tcopyb(r5, 8[a + 0]) -> r10")
        if off: w(f"\tadd(r10, {off*8}) -> r10")
        if binary:
            w(f"\tcopyb(0, F6_PB) -> r5")
            w(f"\tcopyb(r5, 8[a + 0]) -> r11")
            if off: w(f"\tadd(r11, {off*8}) -> r11")
            w(f"\tcopyb(0, F6_PR) -> r5")
            w(f"\tcopyb(r5, 8[a + 0]) -> r12")
            if off: w(f"\tadd(r12, {off*8}) -> r12")
        else:
            w(f"\tcopyb(0, F6_PR) -> r5")
            w(f"\tcopyb(r5, 8[a + 0]) -> r11")
            if off: w(f"\tadd(r11, {off*8}) -> r11")
        w(f"\tcall zisklib_{op}_fp2_bn254")
    w("\tpop r1")
    w("\tret")
    w("")

component_routine("add_fp6","add",True)
component_routine("sub_fp6","sub",True)
component_routine("dbl_fp6","dbl",False)
component_routine("neg_fp6","neg",False)

# ---- helper to open a copy-in routine ----
def open_routine(sig, name, copy_a=True, copy_b=None):
    # copy_b: None (no b), "fp6" (24u64 -> B1,B2,B3), "fp2" (8 -> B1), "fp2x2" (16 -> B1,B2)
    w(f"; {sig}")
    w(f"zisklib_{name}_bn254:")
    w("\tpush r1")
    save_ptr("F6_PA","r10")
    if copy_b is not None:
        save_ptr("F6_PB","r11")
        save_ptr("F6_PR","r12")
    else:
        save_ptr("F6_PR","r11")
    if copy_a:
        cpin(A1,"F6_PA",0); cpin(A2,"F6_PA",8); cpin(A3,"F6_PA",16)
    if copy_b=="fp6":
        cpin(B1,"F6_PB",0); cpin(B2,"F6_PB",8); cpin(B3,"F6_PB",16)
    elif copy_b=="fp2":
        cpin(B1,"F6_PB",0)
    elif copy_b=="fp2x2":
        cpin(B1,"F6_PB",0); cpin(B2,"F6_PB",8)
def close_routine():
    cpout(C1,0); cpout(C2,8); cpout(C3,16)
    w("\tpop r1")
    w("\tret")
    w("")

# ---- mul_fp6 ----
open_routine("void zisklib_mul_fp6_bn254(const u64* a, const u64* b, u64* result)  [24,24,24]",
             "mul_fp6", True, "fp6")
mul(T[1],A1,B1); mul(T[2],A2,B2); mul(T[3],A3,B3); mul(T[4],T[3],XI)
add(T[5],A2,A3); add(T[6],B2,B3); add(T[7],A1,A2); add(T[8],B1,B2); add(T[9],A1,A3); add(T[10],B1,B3)
mul(C1,T[5],T[6]); sub(C1,C1,T[2]); sub(C1,C1,T[3]); mul(C1,C1,XI); add(C1,C1,T[1])
mul(C2,T[7],T[8]); sub(C2,C2,T[1]); sub(C2,C2,T[2]); add(C2,C2,T[4])
mul(C3,T[9],T[10]); sub(C3,C3,T[1]); add(C3,C3,T[2]); sub(C3,C3,T[3])
close_routine()

# ---- square_fp6 ----
open_routine("void zisklib_square_fp6_bn254(const u64* a, u64* result)  [24,24]",
             "square_fp6", True, None)
# needs PR saved from r11 -> already done (copy_b None -> save PR from r11). good.
mul(T[1],A1,A2); dbl(T[1],T[1])          # two_a1a2
sq(T[2],A3)                               # a3sq
mul(C2,T[2],XI); add(C2,C2,T[1])          # c2 = a3sq*XI + two_a1a2
sq(T[3],A1)                               # a1sq
sub(T[4],A1,A2); add(T[4],T[4],A3); sq(T[4],T[4])   # (a1-a2+a3)^2
mul(T[5],A2,A3); dbl(T[5],T[5])           # two_a2a3
mul(C1,T[5],XI); add(C1,C1,T[3])          # c1 = two_a2a3*XI + a1sq
sub(C3,T[1],T[2]); add(C3,C3,T[4]); add(C3,C3,T[5]); sub(C3,C3,T[3])  # c3
close_routine()

# ---- inv_fp6 ----
open_routine("void zisklib_inv_fp6_bn254(const u64* a, u64* result)  [24,24]",
             "inv_fp6", True, None)
sq(T[1],A1); sq(T[2],A2); sq(T[3],A3)
mul(T[4],A1,A2); mul(T[5],A1,A3); mul(T[6],A2,A3)
mul(T[7],T[6],XI); sub(C1,T[1],T[7])      # c1mid = a1sq - XI*a2a3
mul(T[7],T[3],XI); sub(C2,T[7],T[4])      # c2mid = XI*a3sq - a1a2
sub(C3,T[2],T[5])                          # c3mid = a2sq - a1a3
mul(T[8],A1,C1)                            # im = a1*c1mid
mul(T[9],A3,C2); mul(T[10],A2,C3); add(T[9],T[9],T[10]); mul(T[9],T[9],XI); add(T[9],T[9],T[8]); invop(T[9],T[9])
mul(C1,C1,T[9]); mul(C2,C2,T[9]); mul(C3,C3,T[9])
close_routine()

# ---- sparse_mula_fp6: b = b2·v (b2 in Fp2) ----
open_routine("void zisklib_sparse_mula_fp6_bn254(const u64* a, const u64* b2, u64* result)  [24,8,24]",
             "sparse_mula_fp6", True, "fp2")   # b2 -> B1
mul(C1,B1,A3); mul(C1,C1,XI)               # c1 = b2*a3*XI
mul(C2,B1,A1)                              # c2 = b2*a1
mul(C3,B1,A2)                              # c3 = b2*a2
close_routine()

# ---- sparse_mulb_fp6: b = b1 + b2·v (b[16]) ----
open_routine("void zisklib_sparse_mulb_fp6_bn254(const u64* a, const u64* b, u64* result)  [24,16,24]",
             "sparse_mulb_fp6", True, "fp2x2")  # B1=b1, B2=b2
mul(T[2],B2,XI); mul(T[1],A3,T[2]); mul(C1,A1,B1); add(C1,C1,T[1])   # c1=a1b1+a3*(b2*XI)
mul(C2,A1,B2); mul(T[1],A2,B1); add(C2,C2,T[1])                       # c2=a1b2+a2b1
mul(C3,A2,B2); mul(T[1],A3,B1); add(C3,C3,T[1])                       # c3=a2b2+a3b1
close_routine()

# ---- sparse_mulc_fp6: b = b2·v + b3·v² (b[16]: B1=b2, B2=b3) ----
open_routine("void zisklib_sparse_mulc_fp6_bn254(const u64* a, const u64* b, u64* result)  [24,16,24]",
             "sparse_mulc_fp6", True, "fp2x2")  # B1=b2, B2=b3
mul(C1,A2,B2); mul(T[1],A3,B1); add(C1,C1,T[1]); mul(C1,C1,XI)       # c1=(a2b3+a3b2)*XI
mul(C2,A3,B2); mul(C2,C2,XI); mul(T[1],A1,B1); add(C2,C2,T[1])       # c2=a1b2+a3b3*XI
mul(C3,A1,B2); mul(T[1],A2,B1); add(C3,C3,T[1])                       # c3=a1b3+a2b2
close_routine()

open(_OUT+"/bn254/fp6.zisk","w").write("\n".join(out)+"\n")
print("wrote fp6.zisk,", len(out), "lines")
