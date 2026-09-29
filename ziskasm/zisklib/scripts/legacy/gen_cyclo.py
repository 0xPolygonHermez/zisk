#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bn254/cyclotomic.zisk from cyclotomic.rs.
# GΦ6(p²) ⊂ Fp12. Compressed form = 4 Fp2 = 32 u64 [a2,a3,a4,a5].
# Fp12 element a = (a0 + a4v + a3v²) + (a2 + a1v + a5v²)w = 48 u64.
out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)
def f2bin(op,dst,x,y):
    assert dst!=y, f"{op} {dst}: aliases y"
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {y}) -> r11"); raw(f"copyb(0, {dst}) -> r12"); raw(f"call zisklib_{op}_fp2_bn254")
def f2un(op,dst,x): raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"call zisklib_{op}_fp2_bn254")
def add2(d,x,y): f2bin("add",d,x,y)
def sub2(d,x,y): f2bin("sub",d,x,y)
def mul2(d,x,y): f2bin("mul",d,x,y)
def sq2(d,x): f2un("square",d,x)
def dbl2(d,x): f2un("dbl",d,x)
def inv2(d,x): f2un("inv",d,x)
def sm2(d,x,s):  # scalar_mul_fp2(x, s[4]) -> d (d may == x)
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {s}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw("call zisklib_scalar_mul_fp2_bn254")
def cpin(slot, ptrslot, off_words, n=8):
    raw(f"copyb(0, {ptrslot}) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
    if off_words: raw(f"add(r10, {off_words*8}) -> r10")
    raw(f"copyb(0, {slot}) -> r11"); raw(f"copyb(0, {n}) -> r12"); raw("call bn254_memcpy")
def cpout(slot, ptrslot, off_words, n=8):
    raw(f"copyb(0, {slot}) -> r10"); raw(f"copyb(0, {ptrslot}) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
    if off_words: raw(f"add(r11, {off_words*8}) -> r11")
    raw(f"copyb(0, {n}) -> r12"); raw("call bn254_memcpy")
def save_ptr(slot,reg): raw(f"copyb(0, {slot}) -> r5"); raw(f"copyb(r5, {reg}) -> 8[a + 0]")

w("; ============================================================================")
w("; bn254/cyclotomic.zisk - GΦ6(p²) subgroup arithmetic (compressed cyclotomic).")
w("; GENERATED from cyclotomic.rs (scripts/legacy/gen_cyclo.py). Compressed = 32 u64")
w("; [a2,a3,a4,a5]; Fp12 = 48 u64. Prefixes cy_/dec_/expx_.")
w("; ============================================================================")
w("")
w("const u64 BN_XI10[8] = 10, 0, 0, 0, 1, 0, 0, 0")   # 10 + u
w("const u64 BN_THREE[4] = 3, 0, 0, 0")
w("const u64 BN_FOUR[4] = 4, 0, 0, 0")
w("const u64 BN_X_BIN_LE[63] =")
xble=[1,0,0,0,1,1,1,1,1,0,0,1,0,0,0,0,1,0,0,1,0,1,1,0,0,1,0,1,0,0,1,0,0,0,1,0,1,1,0,1,0,1,0,0,1,0,0,1,1,0,0,1,0,1,1,1,0,0,1,0,0,0,1]
w("\t"+", ".join(str(b) for b in xble))
w("")
for s in ["A2","A3","A4","A5","B2","B3","B4","B5","A0","A1"]:
    w(f"u64 CY_{s}[8] = 0, 0, 0, 0, 0, 0, 0, 0")
for i in range(1,8):
    w(f"u64 CY_T{i}[8] = 0, 0, 0, 0, 0, 0, 0, 0")
w("u64 CY_PA[1] = 0"); w("u64 CY_PR[1] = 0")
w("u64 EXPX_A[1] = 0"); w("u64 EXPX_R[1] = 0")
w("u64 EXPX_RES[48] = "+", ".join(["0"]*48))
w("u64 EXPX_COMP[32] = "+", ".join(["0"]*32))
w("u64 EXPX_COMP2[32] = "+", ".join(["0"]*32))
w("u64 EXPX_DEC[48] = "+", ".join(["0"]*48))
w("")
A2,A3,A4,A5="CY_A2","CY_A3","CY_A4","CY_A5"
B2,B3,B4,B5="CY_B2","CY_B3","CY_B4","CY_B5"
A0,A1="CY_A0","CY_A1"
T=[None]+[f"CY_T{i}" for i in range(1,8)]
XI="BN_XI"; XI10="BN_XI10"; ONE="BN_FP2_ONE"

# ---- compress_cyclo(a[48]) -> result[32] : [a2,a3,a4,a5] ----
# a layout: a0=[0..8] a4=[8..16] a3=[16..24] a2=[24..32] a1=[32..40] a5=[40..48]
w("; void zisklib_compress_cyclo_bn254(const u64* a, u64* result)  [48,32]")
w("zisklib_compress_cyclo_bn254:")
raw("push r1")
save_ptr("CY_PA","r10"); save_ptr("CY_PR","r11")
# result[0..8]=a2(a@24), [8..16]=a3(a@16), [16..24]=a4(a@8), [24..32]=a5(a@40)
cpin(A2,"CY_PA",24); cpin(A3,"CY_PA",16); cpin(A4,"CY_PA",8); cpin(A5,"CY_PA",40)
cpout(A2,"CY_PR",0); cpout(A3,"CY_PR",8); cpout(A4,"CY_PR",16); cpout(A5,"CY_PR",24)
raw("pop r1"); raw("ret"); w("")

# ---- square_cyclo(comp[32]) -> result[32] ----
w("; void zisklib_square_cyclo_bn254(const u64* a, u64* result)  [32,32]")
w("zisklib_square_cyclo_bn254:")
raw("push r1")
save_ptr("CY_PA","r10"); save_ptr("CY_PR","r11")
cpin(A2,"CY_PA",0); cpin(A3,"CY_PA",8); cpin(A4,"CY_PA",16); cpin(A5,"CY_PA",24)
mul2(T[1],A2,A3)                 # b23
mul2(T[2],A4,A5)                 # b45
mul2(T[3],A3,XI); add2(T[4],A2,A3); add2(T[5],A2,T[3]); mul2(T[6],T[4],T[5])   # a23 -> T6
mul2(T[3],A5,XI); add2(T[4],A4,A5); add2(T[5],A4,T[3]); mul2(T[7],T[4],T[5])   # a45 -> T7
# b2 = 2(a2 + 3·XI·b45)
mul2(T[3],T[2],XI); sm2(T[3],T[3],"BN_THREE"); add2(B2,T[3],A2); dbl2(B2,B2)
# b3 = 3·(a45 - (10+u)·b45) - 2·a3
mul2(T[3],T[2],XI10); sub2(T[4],T[7],T[3]); sm2(T[4],T[4],"BN_THREE"); dbl2(T[5],A3); sub2(B3,T[4],T[5])
# b4 = 3·(a23 - (10+u)·b23) - 2·a4
mul2(T[3],T[1],XI10); sub2(T[4],T[6],T[3]); sm2(T[4],T[4],"BN_THREE"); dbl2(T[5],A4); sub2(B4,T[4],T[5])
# b5 = 2(a5 + 3·b23)
sm2(T[3],T[1],"BN_THREE"); add2(B5,T[3],A5); dbl2(B5,B5)
cpout(B2,"CY_PR",0); cpout(B3,"CY_PR",8); cpout(B4,"CY_PR",16); cpout(B5,"CY_PR",24)
raw("pop r1"); raw("ret"); w("")

# ---- decompress_cyclo(comp[32]) -> result[48] ----
w("; void zisklib_decompress_cyclo_bn254(const u64* a, u64* result)  [32,48]")
w("zisklib_decompress_cyclo_bn254:")
raw("push r1")
save_ptr("CY_PA","r10"); save_ptr("CY_PR","r11")
cpin(A2,"CY_PA",0); cpin(A3,"CY_PA",8); cpin(A4,"CY_PA",16); cpin(A5,"CY_PA",24)
# a2 == 0 ?
raw(f"copyb(0, {A2}) -> r10"); raw("copyb(0, BN_G1_IDENTITY) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(dec_gen)")
# --- a2 == 0 branch ---
# a1 = (2·a4·a5)/a3
inv2(T[1],A3)                    # a3inv
mul2(A1,A4,A5); dbl2(A1,A1); mul2(A1,A1,T[1])
# a0 = (2·a1² - 3·a3·a4)(9+u) + 1
mul2(T[2],A3,A4)                 # a3a4
sq2(A0,A1); dbl2(A0,A0); sm2(T[3],T[2],"BN_THREE"); sub2(A0,A0,T[3]); mul2(A0,A0,XI); add2(A0,A0,ONE)
raw("jump(dec_out)")
w("dec_gen:")
# --- a2 != 0 branch ---
sm2(T[1],A2,"BN_FOUR"); inv2(T[1],T[1])   # a2inv = inv(4a2)
sq2(T[2],A4)                              # a4sq
sq2(A1,A5); mul2(A1,A1,XI); sm2(T[3],T[2],"BN_THREE"); add2(A1,A1,T[3]); dbl2(T[4],A3); sub2(A1,A1,T[4]); mul2(A1,A1,T[1])
mul2(T[2],A3,A4)                          # a3a4
mul2(T[3],A2,A5)                          # a2a5
sq2(A0,A1); dbl2(A0,A0); add2(A0,A0,T[3]); sm2(T[4],T[2],"BN_THREE"); sub2(A0,A0,T[4]); mul2(A0,A0,XI); add2(A0,A0,ONE)
w("dec_out:")
# result: a0=[0..8] a4=[8..16] a3=[16..24] a2=[24..32] a1=[32..40] a5=[40..48]
cpout(A0,"CY_PR",0); cpout(A4,"CY_PR",8); cpout(A3,"CY_PR",16); cpout(A2,"CY_PR",24); cpout(A1,"CY_PR",32); cpout(A5,"CY_PR",40)
raw("pop r1"); raw("ret"); w("")

# ---- exp_by_x_cyclo(a[48]) -> result[48] ----
w("; void zisklib_exp_by_x_cyclo_bn254(const u64* a, u64* result)  [48,48]")
w("zisklib_exp_by_x_cyclo_bn254:")
raw("push r1")
save_ptr("EXPX_A","r10"); save_ptr("EXPX_R","r11")
# EXPX_RES = a
raw("copyb(0, EXPX_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, EXPX_RES) -> r11"); raw("copyb(0, 48) -> r12"); raw("call bn254_memcpy")
# EXPX_COMP = compress(a)
raw("copyb(0, EXPX_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, EXPX_COMP) -> r11"); raw("call zisklib_compress_cyclo_bn254")
raw("copyb(0, 1) -> r13")
w("expx_loop:")
raw("eq(r13, 63), j(expx_done)")
# comp = square_cyclo(comp)
raw("copyb(0, EXPX_COMP) -> r10"); raw("copyb(0, EXPX_COMP2) -> r11"); raw("call zisklib_square_cyclo_bn254")
raw("copyb(0, EXPX_COMP2) -> r10"); raw("copyb(0, EXPX_COMP) -> r11"); raw("copyb(0, 32) -> r12"); raw("call bn254_memcpy")
# bit = X_BIN_LE[r13] (u8 word-padded: offset r13*8)
raw("copyb(0, BN_X_BIN_LE) -> r5"); raw("sll(r13, 3) -> r6"); raw("add(r5, r6) -> r5"); raw("copyb(r5, 1[a + 0]) -> r6")
raw("eq(r6, 0), j(expx_next)")
# dec = decompress(comp) ; res = mul_fp12(res, dec)
raw("copyb(0, EXPX_COMP) -> r10"); raw("copyb(0, EXPX_DEC) -> r11"); raw("call zisklib_decompress_cyclo_bn254")
raw("copyb(0, EXPX_RES) -> r10"); raw("copyb(0, EXPX_DEC) -> r11"); raw("copyb(0, EXPX_RES) -> r12"); raw("call zisklib_mul_fp12_bn254")
w("expx_next:")
raw("add(r13, 1) -> r13"); raw("jump(expx_loop)")
w("expx_done:")
raw("copyb(0, EXPX_RES) -> r10"); raw("copyb(0, EXPX_R) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 48) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

open(_OUT+"/bn254/cyclotomic.zisk","w").write("\n".join(out)+"\n")
print("wrote cyclotomic.zisk,", len(out), "lines")
