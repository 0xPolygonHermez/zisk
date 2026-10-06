#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bn254/twist.zisk from twist.rs. G2 point = 16 u64 =
# x(Fp2 8)‖y(Fp2 8). Affine add/dbl via inv_fp2 (no line-coeff fcalls here; those
# are in the Miller loop). Scratch hierarchy (each level's live state in slots the
# callees never touch): TW_* (add/dbl/utf/is_on_curve arith) < AC_/DC_ (complete
# add/dbl ptr saves) < SMX_ (scalar_mul_by_x) < SUB_ (is_on_subgroup).
out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)

# fp2-op emitters on 8-word STATIC slot labels (alias rule: dst != y).
def f2bin(op,dst,x,y):
    assert dst!=y, f"{op} {dst}: aliases y"
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {y}) -> r11"); raw(f"copyb(0, {dst}) -> r12")
    raw(f"call zisklib_{op}_fp2_bn254")
def f2un(op,dst,x):
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"call zisklib_{op}_fp2_bn254")
def add2(d,x,y): f2bin("add",d,x,y)
def sub2(d,x,y): f2bin("sub",d,x,y)
def mul2(d,x,y): f2bin("mul",d,x,y)
def sq2(d,x): f2un("square",d,x)
def dbl2(d,x): f2un("dbl",d,x)
def neg2(d,x): f2un("neg",d,x)
def inv2(d,x): f2un("inv",d,x)
def conj2(d,x): f2un("conjugate",d,x)
def sm2(d,x,s):  # scalar_mul_fp2(x, s[4]) -> d
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {s}) -> r11"); raw(f"copyb(0, {d}) -> r12")
    raw("call zisklib_scalar_mul_fp2_bn254")

def ptr_comp(reg, ptrslot, comp):   # reg = [ptrslot] + comp*64 bytes (comp 0=x,1=y)
    raw(f"copyb(0, {ptrslot}) -> {reg}"); raw(f"copyb({reg}, 8[a + 0]) -> {reg}")
    if comp: raw(f"add({reg}, {comp*64}) -> {reg}")
def load_slot(slot, ptrslot, comp):  # slot(8) = point component
    ptr_comp("r10", ptrslot, comp); raw(f"copyb(0, {slot}) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_memcpy")
def store_slot(slot, ptrslot, comp): # point component = slot(8)
    raw(f"copyb(0, {slot}) -> r10"); ptr_comp("r11", ptrslot, comp); raw("copyb(0, 8) -> r12"); raw("call bn254_memcpy")
def save_ptr(slot,reg): raw(f"copyb(0, {slot}) -> r5"); raw(f"copyb(r5, {reg}) -> 8[a + 0]")
def memcpy16(src_ptrslot, dst_ptrslot):   # copy 16 u64 [src]->[dst]
    raw(f"copyb(0, {src_ptrslot}) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
    raw(f"copyb(0, {dst_ptrslot}) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
    raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
def zero16_at(ptrslot):
    raw(f"copyb(0, {ptrslot}) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
    for i in range(16): raw(f"copyb(r11, 0) -> 8[a + {i*8}]")

w("; ============================================================================")
w("; bn254/twist.zisk - G2 point arithmetic on the twist E'/Fp2: y²=x³+3/(9+u).")
w("; GENERATED from twist.rs (scripts/legacy/gen_twist.py). G2 point = 16 u64 =")
w("; x(Fp2)‖y(Fp2). Identity = all-zero. Affine add/dbl via inv_fp2.")
w("; ============================================================================")
w("")
# scratch
for s in ["X1","Y1","X2","Y2","X3","Y3","L","DEN","T1","T2"]:
    w(f"u64 TW_{s}[8] = 0, 0, 0, 0, 0, 0, 0, 0")
w("u64 TW_P1[1] = 0"); w("u64 TW_P2[1] = 0"); w("u64 TW_PR[1] = 0")
w("u64 AC_P1[1] = 0"); w("u64 AC_P2[1] = 0"); w("u64 AC_PR[1] = 0")
w("u64 DC_P[1] = 0");  w("u64 DC_PR[1] = 0")
w("u64 SMX_P[1] = 0"); w("u64 SMX_R[16] = "+", ".join(["0"]*16)); w("u64 SMX_RR[16] = "+", ".join(["0"]*16))
w("u64 SUB_P[1] = 0")
for s in ["XP","X1P","PSI1","PSI2","LHS","RHS"]:
    w(f"u64 SUB_{s}[16] = "+", ".join(["0"]*16))
w("u64 OCT_T[8] = 0, 0, 0, 0, 0, 0, 0, 0")
w("")

# ---- neg_twist ----
w("; void zisklib_neg_twist_bn254(const u64* p, u64* result)  [16,16]")
w("zisklib_neg_twist_bn254:")
raw("push r1")
save_ptr("TW_P1","r10"); save_ptr("TW_PR","r11")
load_slot("TW_X1","TW_P1",0); load_slot("TW_Y1","TW_P1",1)
neg2("TW_Y3","TW_Y1")
store_slot("TW_X1","TW_PR",0); store_slot("TW_Y3","TW_PR",1)
raw("pop r1"); raw("ret"); w("")

# ---- utf_endomorphism ----
w("; void zisklib_utf_endomorphism_twist_bn254(const u64* p, u64* result)  [16,16]")
w("zisklib_utf_endomorphism_twist_bn254:")
raw("push r1")
save_ptr("TW_P1","r10"); save_ptr("TW_PR","r11")
load_slot("TW_X1","TW_P1",0); load_slot("TW_Y1","TW_P1",1)
conj2("TW_X2","TW_X1"); conj2("TW_Y2","TW_Y1")
mul2("TW_X3","BN_FROBENIUS_GAMMA12","TW_X2")
mul2("TW_Y3","BN_FROBENIUS_GAMMA13","TW_Y2")
store_slot("TW_X3","TW_PR",0); store_slot("TW_Y3","TW_PR",1)
raw("pop r1"); raw("ret"); w("")

# ---- is_on_curve_twist -> r10 bool ----
w("; u64 zisklib_is_on_curve_twist_bn254(const u64* p)  [r10=&p] -> r10 (1 if on twist)")
w("zisklib_is_on_curve_twist_bn254:")
raw("push r1")
save_ptr("TW_P1","r10")
load_slot("TW_X1","TW_P1",0); load_slot("TW_Y1","TW_P1",1)
sq2("TW_T1","TW_X1")                 # x²
mul2("TW_T2","TW_T1","TW_X1")        # x³
add2("TW_X3","TW_T2","BN_ETWISTED_B")# x³+b'
sq2("TW_Y3","TW_Y1")                 # y²
raw("copyb(0, TW_X3) -> r10"); raw("copyb(0, TW_Y3) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_eqn")
raw("ltu(0, r10), j(twoc_true)")
# else p == identity ?
raw("copyb(0, TW_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BN_G2_IDENTITY) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret")
w("twoc_true:"); raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret"); w("")

# ---- dbl_twist (non-complete) ----
w("; void zisklib_dbl_twist_bn254(const u64* p, u64* result)  [16,16] (p non-identity)")
w("zisklib_dbl_twist_bn254:")
raw("push r1")
save_ptr("TW_P1","r10"); save_ptr("TW_PR","r11")
load_slot("TW_X1","TW_P1",0); load_slot("TW_Y1","TW_P1",1)
dbl2("TW_L","TW_Y1")                 # 2y
inv2("TW_L","TW_L")                  # 1/2y
sm2("TW_L","TW_L","BN_E_B")          # 3/2y
mul2("TW_L","TW_L","TW_X1")          # 3x/2y
mul2("TW_L","TW_L","TW_X1")          # 3x²/2y
sq2("TW_X3","TW_L"); sub2("TW_X3","TW_X3","TW_X1"); sub2("TW_X3","TW_X3","TW_X1")
sub2("TW_Y3","TW_X1","TW_X3"); mul2("TW_T2","TW_L","TW_Y3"); sub2("TW_Y3","TW_T2","TW_Y1")
store_slot("TW_X3","TW_PR",0); store_slot("TW_Y3","TW_PR",1)
raw("pop r1"); raw("ret"); w("")

# ---- add_twist (non-complete) ----
w("; void zisklib_add_twist_bn254(const u64* p1, const u64* p2, u64* result)  [16,16,16]")
w("zisklib_add_twist_bn254:")
raw("push r1")
save_ptr("TW_P1","r10"); save_ptr("TW_P2","r11"); save_ptr("TW_PR","r12")
# x1 == x2 ?
ptr_comp("r10","TW_P1",0); ptr_comp("r11","TW_P2",0); raw("copyb(0, 8) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(twadd_generic)")
# y1 == y2 ?
ptr_comp("r10","TW_P1",1); ptr_comp("r11","TW_P2",1); raw("copyb(0, 8) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(twadd_inf)")
# doubling
raw("copyb(0, TW_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, TW_PR) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("call zisklib_dbl_twist_bn254"); raw("pop r1"); raw("ret")
w("twadd_inf:")
zero16_at("TW_PR"); raw("pop r1"); raw("ret")
w("twadd_generic:")
load_slot("TW_X1","TW_P1",0); load_slot("TW_Y1","TW_P1",1)
load_slot("TW_X2","TW_P2",0); load_slot("TW_Y2","TW_P2",1)
sub2("TW_DEN","TW_X2","TW_X1"); inv2("TW_DEN","TW_DEN")
sub2("TW_L","TW_Y2","TW_Y1"); mul2("TW_L","TW_L","TW_DEN")
sq2("TW_X3","TW_L"); sub2("TW_X3","TW_X3","TW_X1"); sub2("TW_X3","TW_X3","TW_X2")
sub2("TW_Y3","TW_X1","TW_X3"); mul2("TW_T2","TW_L","TW_Y3"); sub2("TW_Y3","TW_T2","TW_Y1")
store_slot("TW_X3","TW_PR",0); store_slot("TW_Y3","TW_PR",1)
raw("pop r1"); raw("ret"); w("")

# ---- add_complete_twist ----
w("; void zisklib_add_complete_twist_bn254(const u64* p1, const u64* p2, u64* result)")
w("zisklib_add_complete_twist_bn254:")
raw("push r1")
save_ptr("AC_P1","r10"); save_ptr("AC_P2","r11"); save_ptr("AC_PR","r12")
# p1 == identity ? -> result = p2
raw("copyb(0, AC_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BN_G2_IDENTITY) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(act_chk2)")
memcpy16("AC_P2","AC_PR"); raw("pop r1"); raw("ret")
w("act_chk2:")
# p2 == identity ? -> result = p1
raw("copyb(0, AC_P2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BN_G2_IDENTITY) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(act_add)")
memcpy16("AC_P1","AC_PR"); raw("pop r1"); raw("ret")
w("act_add:")
raw("copyb(0, AC_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, AC_P2) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("copyb(0, AC_PR) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12")
raw("call zisklib_add_twist_bn254"); raw("pop r1"); raw("ret"); w("")

# ---- dbl_complete_twist ----
w("; void zisklib_dbl_complete_twist_bn254(const u64* p, u64* result)")
w("zisklib_dbl_complete_twist_bn254:")
raw("push r1")
save_ptr("DC_P","r10"); save_ptr("DC_PR","r11")
raw("copyb(0, DC_P) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BN_G2_IDENTITY) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(dct_dbl)")
zero16_at("DC_PR"); raw("pop r1"); raw("ret")
w("dct_dbl:")
raw("copyb(0, DC_P) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, DC_PR) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("call zisklib_dbl_twist_bn254"); raw("pop r1"); raw("ret"); w("")

# ---- scalar_mul_by_x_complete_twist ----
# r = p; for bit in X_BIN_BE[1..63]: r = dbl_complete(r); if bit: r = add_complete(r, p)
w("; void zisklib_scalar_mul_by_x_complete_twist_bn254(const u64* p, u64* result)  [16,16]")
w("zisklib_scalar_mul_by_x_complete_twist_bn254:")
raw("push r1")
save_ptr("SMX_P","r10")
# result ptr saved via DC? no—save into a slot not touched by callees. Use SMX slot? add_complete uses AC_, dbl uses DC_. Use a fresh SMX_PR.
w("\tcopyb(0, SMX_PRSLOT) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
# identity? -> result = identity
raw("copyb(0, SMX_P) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BN_G2_IDENTITY) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(smx_run)")
raw("copyb(0, SMX_PRSLOT) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
for i in range(16): raw(f"copyb(r11, 0) -> 8[a + {i*8}]")
raw("pop r1"); raw("ret")
w("smx_run:")
# SMX_R = p
raw("copyb(0, SMX_P) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, SMX_R) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
# loop index r13 = 1 .. 62 (X_BIN_BE has 63 entries, skip [0])
raw("copyb(0, 1) -> r13")
w("smx_loop:")
raw("eq(r13, 63), j(smx_done)")
# SMX_RR = dbl_complete(SMX_R)
raw("copyb(0, SMX_R) -> r10"); raw("copyb(0, SMX_RR) -> r11"); raw("call zisklib_dbl_complete_twist_bn254")
# copy SMX_RR -> SMX_R
raw("copyb(0, SMX_RR) -> r10"); raw("copyb(0, SMX_R) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
# bit = X_BIN_BE[r13]
# NOTE: ziskasm stores u8-array elements word-padded (8 bytes each), so element
# r13 is at byte offset r13*8, not r13.
raw("copyb(0, BN_X_BIN_BE) -> r5"); raw("sll(r13, 3) -> r6"); raw("add(r5, r6) -> r5"); raw("copyb(r5, 1[a + 0]) -> r6")
raw("eq(r6, 0), j(smx_next)")
# SMX_RR = add_complete(SMX_R, p)
raw("copyb(0, SMX_R) -> r10"); raw("copyb(0, SMX_P) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("copyb(0, SMX_RR) -> r12"); raw("call zisklib_add_complete_twist_bn254")
raw("copyb(0, SMX_RR) -> r10"); raw("copyb(0, SMX_R) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
w("smx_next:")
raw("add(r13, 1) -> r13"); raw("jump(smx_loop)")
w("smx_done:")
raw("copyb(0, SMX_R) -> r10"); raw("copyb(0, SMX_PRSLOT) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ---- is_on_subgroup_twist -> r10 bool ----
w("; u64 zisklib_is_on_subgroup_twist_bn254(const u64* p)  [r10=&p] -> r10 bool")
w("zisklib_is_on_subgroup_twist_bn254:")
raw("push r1")
save_ptr("SUB_P","r10")
# xp = x·p
raw("copyb(0, SUB_P) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, SUB_XP) -> r11"); raw("call zisklib_scalar_mul_by_x_complete_twist_bn254")
# x1p = p + xp
raw("copyb(0, SUB_P) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, SUB_XP) -> r11"); raw("copyb(0, SUB_X1P) -> r12"); raw("call zisklib_add_complete_twist_bn254")
# psi1 = utf(xp) ; psi2 = utf(psi1)
raw("copyb(0, SUB_XP) -> r10"); raw("copyb(0, SUB_PSI1) -> r11"); raw("call zisklib_utf_endomorphism_twist_bn254")
raw("copyb(0, SUB_PSI1) -> r10"); raw("copyb(0, SUB_PSI2) -> r11"); raw("call zisklib_utf_endomorphism_twist_bn254")
# lhs = x1p + psi1 ; lhs = lhs + psi2
raw("copyb(0, SUB_X1P) -> r10"); raw("copyb(0, SUB_PSI1) -> r11"); raw("copyb(0, SUB_LHS) -> r12"); raw("call zisklib_add_complete_twist_bn254")
raw("copyb(0, SUB_LHS) -> r10"); raw("copyb(0, SUB_PSI2) -> r11"); raw("copyb(0, SUB_LHS) -> r12"); raw("call zisklib_add_complete_twist_bn254")
# rhs = utf(utf(utf(dbl(xp))))
raw("copyb(0, SUB_XP) -> r10"); raw("copyb(0, SUB_RHS) -> r11"); raw("call zisklib_dbl_complete_twist_bn254")
raw("copyb(0, SUB_RHS) -> r10"); raw("copyb(0, SUB_RHS) -> r11"); raw("call zisklib_utf_endomorphism_twist_bn254")
raw("copyb(0, SUB_RHS) -> r10"); raw("copyb(0, SUB_RHS) -> r11"); raw("call zisklib_utf_endomorphism_twist_bn254")
raw("copyb(0, SUB_RHS) -> r10"); raw("copyb(0, SUB_RHS) -> r11"); raw("call zisklib_utf_endomorphism_twist_bn254")
# return lhs == rhs
raw("copyb(0, SUB_LHS) -> r10"); raw("copyb(0, SUB_RHS) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret"); w("")

# ---- jacobian_to_affine_twist ----
w("; void zisklib_jacobian_to_affine_twist_bn254(const u64* p, u64* result)  [24,16]")
w("zisklib_jacobian_to_affine_twist_bn254:")
raw("push r1")
save_ptr("TW_P1","r10"); save_ptr("TW_PR","r11")
# z = p[16..24] ; z == 0 ? (Fp2 all zero)
raw("copyb(0, TW_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 128) -> r10")
raw("copyb(0, BN_G2_IDENTITY) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(j2at_nz)")
zero16_at("TW_PR"); raw("pop r1"); raw("ret")
w("j2at_nz:")
load_slot("TW_X1","TW_P1",0); load_slot("TW_Y1","TW_P1",1)
raw("copyb(0, TW_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 128) -> r10")
raw("copyb(0, TW_X2) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_memcpy")   # TW_X2 = z
inv2("TW_DEN","TW_X2")               # zinv
sq2("TW_L","TW_DEN")                 # zinv²
mul2("TW_X3","TW_X1","TW_L")         # x·zinv²
mul2("TW_Y3","TW_Y1","TW_L")         # y·zinv²
mul2("TW_Y3","TW_Y3","TW_DEN")       # ·zinv
store_slot("TW_X3","TW_PR",0); store_slot("TW_Y3","TW_PR",1)
raw("pop r1"); raw("ret"); w("")

txt="\n".join(out)+"\n"
# SMX result-ptr slot declaration
txt = txt.replace("u64 SUB_P[1] = 0", "u64 SMX_PRSLOT[1] = 0\nu64 SUB_P[1] = 0")
open(_OUT+"/bn254/twist.zisk","w").write(txt)
print("wrote twist.zisk,", len(out), "lines")
