#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/cyclotomic.zisk from bls12_381/cyclotomic.rs.
# GΦ6(p²) ⊂ Fp12. Compressed = 4 Fp2 = 48 u64 [a2,a3,a4,a5]. Fp12 = 72 u64.
# ξ=1+u (BLS_XI), (2+u)=BLS_XI2. exp_cyclo LSB-first (multiply-then-square).
out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)
S="_bls12_381"
def f2bin(op,dst,x,y):
    assert dst!=y, f"{op} {dst}: aliases y"
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {y}) -> r11"); raw(f"copyb(0, {dst}) -> r12"); raw(f"call zisklib_{op}_fp2{S}")
def f2un(op,dst,x): raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"call zisklib_{op}_fp2{S}")
def add2(d,x,y): f2bin("add",d,x,y)
def sub2(d,x,y): f2bin("sub",d,x,y)
def mul2(d,x,y): f2bin("mul",d,x,y)
def sq2(d,x): f2un("square",d,x)
def dbl2(d,x): f2un("dbl",d,x)
def inv2(d,x): f2un("inv",d,x)
def sm2(d,x,s):
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {s}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_scalar_mul_fp2{S}")
def cpin(slot, ptrslot, off_words, n=12):
    raw(f"copyb(0, {ptrslot}) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
    if off_words: raw(f"add(r10, {off_words*8}) -> r10")
    raw(f"copyb(0, {slot}) -> r11"); raw(f"copyb(0, {n}) -> r12"); raw("call bn254_memcpy")
def cpout(slot, ptrslot, off_words, n=12):
    raw(f"copyb(0, {slot}) -> r10"); raw(f"copyb(0, {ptrslot}) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
    if off_words: raw(f"add(r11, {off_words*8}) -> r11")
    raw(f"copyb(0, {n}) -> r12"); raw("call bn254_memcpy")
def save_ptr(slot,reg): raw(f"copyb(0, {slot}) -> r5"); raw(f"copyb(r5, {reg}) -> 8[a + 0]")

w("; ============================================================================")
w("; bls12_381/cyclotomic.zisk - GΦ6(p²) subgroup arithmetic. GENERATED")
w("; (scripts/legacy/gen_bls_cyclo.py). Compressed = 48 u64 [a2,a3,a4,a5]; Fp12 = 72.")
w("; ξ=1+u=BLS_XI, (2+u)=BLS_XI2. Prefix lcy_.")
w("; ============================================================================")
w("")
w("const u64 BLS_XI2[12] = 2, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0")   # 2 + u
w("const u64 BLS_FOUR[6] = 4, 0, 0, 0, 0, 0")
# bit-arrays (u8, LSB-first)
_XABS=0xd201000000010000
XA=[(_XABS>>i)&1 for i in range(64)]
XO=[((_XABS+1)>>i)&1 for i in range(64)]
XD=[(((_XABS+1)//3)>>i)&1 for i in range(63)]
w("const u8 BLS_X_ABS_BIN[64] = "+", ".join(str(b) for b in XA))
w("const u8 BLS_XONE_ABS_BIN[64] = "+", ".join(str(b) for b in XO))
w("const u8 BLS_XDIV3_ABS_BIN[63] = "+", ".join(str(b) for b in XD))
w("")
for s in ["A2","A3","A4","A5","B2","B3","B4","B5","A0","A1"]:
    w(f"u64 BLS_CY_{s}[12] = "+", ".join(["0"]*12))
for i in range(1,8):
    w(f"u64 BLS_CY_T{i}[12] = "+", ".join(["0"]*12))
w("u64 BLS_CY_PA[1] = 0"); w("u64 BLS_CY_PR[1] = 0")
w("u64 BLS_EXPX_A[1] = 0"); w("u64 BLS_EXPX_R[1] = 0"); w("u64 BLS_EXPX_BITS[1] = 0"); w("u64 BLS_EXPX_N[1] = 0")
w("u64 BLS_EXPX_RES[72] = "+", ".join(["0"]*72))
w("u64 BLS_EXPX_COMP[48] = "+", ".join(["0"]*48))
w("u64 BLS_EXPX_COMP2[48] = "+", ".join(["0"]*48))
w("u64 BLS_EXPX_DEC[72] = "+", ".join(["0"]*72))
w("")
A2,A3,A4,A5="BLS_CY_A2","BLS_CY_A3","BLS_CY_A4","BLS_CY_A5"
B2,B3,B4,B5="BLS_CY_B2","BLS_CY_B3","BLS_CY_B4","BLS_CY_B5"
A0,A1="BLS_CY_A0","BLS_CY_A1"
T=[None]+[f"BLS_CY_T{i}" for i in range(1,8)]
XI="BLS_XI"; XI2="BLS_XI2"; ONE="BLS_F2_ONE"

# ---- compress(a[72]) -> [a2,a3,a4,a5][48].  a: a0@0 a4@12 a3@24 a2@36 a1@48 a5@60 ----
w("; void zisklib_compress_cyclo_bls12_381(const u64* a, u64* result)  [72,48]")
w("zisklib_compress_cyclo_bls12_381:")
raw("push r1")
save_ptr("BLS_CY_PA","r10"); save_ptr("BLS_CY_PR","r11")
cpin(A2,"BLS_CY_PA",36); cpin(A3,"BLS_CY_PA",24); cpin(A4,"BLS_CY_PA",12); cpin(A5,"BLS_CY_PA",60)
cpout(A2,"BLS_CY_PR",0); cpout(A3,"BLS_CY_PR",12); cpout(A4,"BLS_CY_PR",24); cpout(A5,"BLS_CY_PR",36)
raw("pop r1"); raw("ret"); w("")

# ---- square_cyclo(comp[48]) -> [48] ----
w("; void zisklib_square_cyclo_bls12_381(const u64* a, u64* result)  [48,48]")
w("zisklib_square_cyclo_bls12_381:")
raw("push r1")
save_ptr("BLS_CY_PA","r10"); save_ptr("BLS_CY_PR","r11")
cpin(A2,"BLS_CY_PA",0); cpin(A3,"BLS_CY_PA",12); cpin(A4,"BLS_CY_PA",24); cpin(A5,"BLS_CY_PA",36)
mul2(T[1],A2,A3); mul2(T[2],A4,A5)
mul2(T[3],A3,XI); add2(T[4],A2,A3); add2(T[5],A2,T[3]); mul2(T[6],T[4],T[5])   # a23
mul2(T[3],A5,XI); add2(T[4],A4,A5); add2(T[5],A4,T[3]); mul2(T[7],T[4],T[5])   # a45
mul2(T[3],T[2],XI); sm2(T[3],T[3],"BLS_THREE"); add2(B2,T[3],A2); dbl2(B2,B2)              # b2
mul2(T[3],T[2],XI2); sub2(T[4],T[7],T[3]); sm2(T[4],T[4],"BLS_THREE"); dbl2(T[5],A3); sub2(B3,T[4],T[5])  # b3
mul2(T[3],T[1],XI2); sub2(T[4],T[6],T[3]); sm2(T[4],T[4],"BLS_THREE"); dbl2(T[5],A4); sub2(B4,T[4],T[5])  # b4
sm2(T[3],T[1],"BLS_THREE"); add2(B5,T[3],A5); dbl2(B5,B5)                                  # b5
cpout(B2,"BLS_CY_PR",0); cpout(B3,"BLS_CY_PR",12); cpout(B4,"BLS_CY_PR",24); cpout(B5,"BLS_CY_PR",36)
raw("pop r1"); raw("ret"); w("")

# ---- decompress(comp[48]) -> [72] ----
w("; void zisklib_decompress_cyclo_bls12_381(const u64* a, u64* result)  [48,72]")
w("zisklib_decompress_cyclo_bls12_381:")
raw("push r1")
save_ptr("BLS_CY_PA","r10"); save_ptr("BLS_CY_PR","r11")
cpin(A2,"BLS_CY_PA",0); cpin(A3,"BLS_CY_PA",12); cpin(A4,"BLS_CY_PA",24); cpin(A5,"BLS_CY_PA",36)
raw(f"copyb(0, {A2}) -> r10"); raw("copyb(0, BLS_ZERO) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(lcy_dec_gen)")
# a2==0: a1=(2·a4·a5)/a3 ; a0=(2·a1²-3·a3·a4)ξ+1
inv2(T[1],A3); mul2(A1,A4,A5); dbl2(A1,A1); mul2(A1,A1,T[1])
mul2(T[2],A3,A4); sq2(A0,A1); dbl2(A0,A0); sm2(T[3],T[2],"BLS_THREE"); sub2(A0,A0,T[3]); mul2(A0,A0,XI); add2(A0,A0,ONE)
raw("jump(lcy_dec_out)")
w("lcy_dec_gen:")
# a2!=0: a1=(a5²ξ+3a4²-2a3)/(4a2) ; a0=(2a1²+a2a5-3a3a4)ξ+1
sm2(T[1],A2,"BLS_FOUR"); inv2(T[1],T[1])
sq2(T[2],A4); sq2(A1,A5); mul2(A1,A1,XI); sm2(T[3],T[2],"BLS_THREE"); add2(A1,A1,T[3]); dbl2(T[4],A3); sub2(A1,A1,T[4]); mul2(A1,A1,T[1])
mul2(T[2],A3,A4); mul2(T[3],A2,A5)
sq2(A0,A1); dbl2(A0,A0); add2(A0,A0,T[3]); sm2(T[4],T[2],"BLS_THREE"); sub2(A0,A0,T[4]); mul2(A0,A0,XI); add2(A0,A0,ONE)
w("lcy_dec_out:")
# result: a0@0 a4@12 a3@24 a2@36 a1@48 a5@60
cpout(A0,"BLS_CY_PR",0); cpout(A4,"BLS_CY_PR",12); cpout(A3,"BLS_CY_PR",24); cpout(A2,"BLS_CY_PR",36); cpout(A1,"BLS_CY_PR",48); cpout(A5,"BLS_CY_PR",60)
raw("pop r1"); raw("ret"); w("")

# ---- exp_cyclo(a, bits, nbits, result) : result = a^(bits, LSB-first) ----
w("; void zisklib_exp_cyclo_bls12_381(const u64* a, const u8* bits, u64 nbits, u64* result)")
w(";   [r10=a, r11=bits, r12=nbits, r13=result]  result=1; comp=C(a);")
w(";   for i in 0..nbits: if bits[i]: result*=D(comp); comp=square(comp).")
w("zisklib_exp_cyclo_bls12_381:")
raw("push r1")
save_ptr("BLS_EXPX_A","r10"); save_ptr("BLS_EXPX_BITS","r11"); save_ptr("BLS_EXPX_N","r12"); save_ptr("BLS_EXPX_R","r13")
# result = one12
raw("copyb(0, BLS_EXPX_RES) -> r11")
for i in range(72): raw(f"copyb(r11, {1 if i==0 else 0}) -> 8[a + {i*8}]")
# comp = compress(a)
raw("copyb(0, BLS_EXPX_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_EXPX_COMP) -> r11"); raw("call zisklib_compress_cyclo_bls12_381")
raw("copyb(0, 0) -> r13")
w("lcy_exp_loop:")
raw("copyb(0, BLS_EXPX_N) -> r5"); raw("copyb(r5, 8[a + 0]) -> r5"); raw("eq(r13, r5), j(lcy_exp_done)")
# bit = bits[r13] (u8 word-padded)
raw("copyb(0, BLS_EXPX_BITS) -> r5"); raw("copyb(r5, 8[a + 0]) -> r5"); raw("sll(r13, 3) -> r6"); raw("add(r5, r6) -> r5"); raw("copyb(r5, 1[a + 0]) -> r6")
raw("eq(r6, 0), j(lcy_exp_sq)")
# result *= decompress(comp)
raw("copyb(0, BLS_EXPX_COMP) -> r10"); raw("copyb(0, BLS_EXPX_DEC) -> r11"); raw("call zisklib_decompress_cyclo_bls12_381")
raw("copyb(0, BLS_EXPX_RES) -> r10"); raw("copyb(0, BLS_EXPX_DEC) -> r11"); raw("copyb(0, BLS_EXPX_RES) -> r12"); raw("call zisklib_mul_fp12_bls12_381")
w("lcy_exp_sq:")
# comp = square_cyclo(comp)
raw("copyb(0, BLS_EXPX_COMP) -> r10"); raw("copyb(0, BLS_EXPX_COMP2) -> r11"); raw("call zisklib_square_cyclo_bls12_381")
raw("copyb(0, BLS_EXPX_COMP2) -> r10"); raw("copyb(0, BLS_EXPX_COMP) -> r11"); raw("copyb(0, 48) -> r12"); raw("call bn254_memcpy")
raw("add(r13, 1) -> r13"); raw("jump(lcy_exp_loop)")
w("lcy_exp_done:")
raw("copyb(0, BLS_EXPX_RES) -> r10"); raw("copyb(0, BLS_EXPX_R) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 72) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ---- wrappers: exp_by_x / exp_by_xone / exp_by_xdiv3 ----
def wrapper(name, bitslabel, nbits):
    w(f"; void zisklib_{name}_bls12_381(const u64* a, u64* result)  [72,72]")
    w(f"zisklib_{name}_bls12_381:")
    raw("push r1")
    raw("copyb(0, r11) -> r13")           # result -> r13
    raw(f"copyb(0, {bitslabel}) -> r11")  # bits ptr
    raw(f"copyb(0, {nbits}) -> r12")      # nbits
    raw("call zisklib_exp_cyclo_bls12_381")
    raw("pop r1"); raw("ret"); w("")
wrapper("exp_by_x_cyclo","BLS_X_ABS_BIN",64)
wrapper("exp_by_xone_cyclo","BLS_XONE_ABS_BIN",64)
wrapper("exp_by_xdiv3_cyclo","BLS_XDIV3_ABS_BIN",63)

open(_OUT+"/bls12_381/cyclotomic.zisk","w").write("\n".join(out)+"\n")
print("wrote bls cyclotomic.zisk,", len(out), "lines")
