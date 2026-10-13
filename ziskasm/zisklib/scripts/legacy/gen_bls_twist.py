#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/twist.zisk (M5 core). G2 point = 24 u64 =
# x(Fp2 12)‖y(Fp2 12). Affine add/dbl via inv_fp2 (same as bn254, 12-limb).
# Includes a general scalar_mul_twist (double-and-add with complete add/dbl).
# ψ/subgroup deferred to M5b. Prefix ltw_.
out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)
S="_bls12_381"
def f2bin(op,dst,x,y):
    assert dst!=y, f"{op} {dst}: aliases y"
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {y}) -> r11"); raw(f"copyb(0, {dst}) -> r12"); raw(f"call zisklib_{op}_fp2{S}")
def f2un(op,dst,x):
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"call zisklib_{op}_fp2{S}")
def add2(d,x,y): f2bin("add",d,x,y)
def sub2(d,x,y): f2bin("sub",d,x,y)
def mul2(d,x,y): f2bin("mul",d,x,y)
def sq2(d,x): f2un("square",d,x)
def dbl2(d,x): f2un("dbl",d,x)
def neg2(d,x): f2un("neg",d,x)
def inv2(d,x): f2un("inv",d,x)
def sm2(d,x,s):  # scalar_mul_fp2(x, s[6]) -> d
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {s}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_scalar_mul_fp2{S}")

def ptr_comp(reg, ptrslot, comp):   # reg = [ptrslot] + comp*96 bytes (0=x,1=y)
    raw(f"copyb(0, {ptrslot}) -> {reg}"); raw(f"copyb({reg}, 8[a + 0]) -> {reg}")
    if comp: raw(f"add({reg}, {comp*96}) -> {reg}")
def load_slot(slot, ptrslot, comp):
    ptr_comp("r10", ptrslot, comp); raw(f"copyb(0, {slot}) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
def store_slot(slot, ptrslot, comp):
    raw(f"copyb(0, {slot}) -> r10"); ptr_comp("r11", ptrslot, comp); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
def save_ptr(slot,reg): raw(f"copyb(0, {slot}) -> r5"); raw(f"copyb(r5, {reg}) -> 8[a + 0]")
def memcpy24(src_ptrslot, dst_ptrslot):
    raw(f"copyb(0, {src_ptrslot}) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
    raw(f"copyb(0, {dst_ptrslot}) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
    raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
def zero24_at(ptrslot):
    raw(f"copyb(0, {ptrslot}) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
    for i in range(24): raw(f"copyb(r11, 0) -> 8[a + {i*8}]")

w("; ============================================================================")
w("; bls12_381/twist.zisk - G2 point arithmetic on E'/Fp2: y²=x³+4(1+u). GENERATED")
w("; (scripts/legacy/gen_bls_twist.py). G2 point = 24 u64 = x(Fp2)‖y(Fp2). Affine")
w("; add/dbl via inv_fp2. General scalar_mul_twist. Prefix ltw_. (ψ/subgroup: M5b)")
w("; ============================================================================")
w("")
for s in ["X1","Y1","X2","Y2","X3","Y3","L","DEN","T1","T2"]:
    w(f"u64 BLS_TW_{s}[12] = "+", ".join(["0"]*12))
w("u64 BLS_TW_P1[1] = 0"); w("u64 BLS_TW_P2[1] = 0"); w("u64 BLS_TW_PR[1] = 0")
w("u64 BLS_AC_P1[1] = 0"); w("u64 BLS_AC_P2[1] = 0"); w("u64 BLS_AC_PR[1] = 0")
w("u64 BLS_DC_P[1] = 0"); w("u64 BLS_DC_PR[1] = 0")
w("u64 BLS_SUB2_NEG[24] = "+", ".join(["0"]*24)); w("u64 BLS_SUB2_P1[1] = 0"); w("u64 BLS_SUB2_PR[1] = 0")
w("u64 BLS_SMT_P[1] = 0"); w("u64 BLS_SMT_PR[1] = 0"); w("u64 BLS_SMT_K[4] = 0, 0, 0, 0")
w("u64 BLS_SMT_ACC[24] = "+", ".join(["0"]*24)); w("u64 BLS_SMT_ACC2[24] = "+", ".join(["0"]*24))
w("")

# ---- neg_twist ----
w("; void zisklib_neg_twist_bls12_381(const u64* p, u64* result)  [24,24]")
w("zisklib_neg_twist_bls12_381:")
raw("push r1")
save_ptr("BLS_TW_P1","r10"); save_ptr("BLS_TW_PR","r11")
load_slot("BLS_TW_X1","BLS_TW_P1",0); load_slot("BLS_TW_Y1","BLS_TW_P1",1)
neg2("BLS_TW_Y3","BLS_TW_Y1")
store_slot("BLS_TW_X1","BLS_TW_PR",0); store_slot("BLS_TW_Y3","BLS_TW_PR",1)
raw("pop r1"); raw("ret"); w("")

# ---- is_on_curve_twist -> r10 ----
w("; u64 zisklib_is_on_curve_twist_bls12_381(const u64* p)  [r10=&p] -> r10 bool")
w("zisklib_is_on_curve_twist_bls12_381:")
raw("push r1")
save_ptr("BLS_TW_P1","r10")
load_slot("BLS_TW_X1","BLS_TW_P1",0); load_slot("BLS_TW_Y1","BLS_TW_P1",1)
sq2("BLS_TW_T1","BLS_TW_X1"); mul2("BLS_TW_T2","BLS_TW_T1","BLS_TW_X1"); add2("BLS_TW_X3","BLS_TW_T2","BLS_ETWISTED_B")
sq2("BLS_TW_Y3","BLS_TW_Y1")
raw("copyb(0, BLS_TW_X3) -> r10"); raw("copyb(0, BLS_TW_Y3) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("ltu(0, r10), j(ltw_oc_true)")
raw("copyb(0, BLS_TW_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_G2_IDENTITY) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret")
w("ltw_oc_true:"); raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret"); w("")

# ---- dbl_twist (non-complete) ----
w("; void zisklib_dbl_twist_bls12_381(const u64* p, u64* result)  [24,24]")
w("zisklib_dbl_twist_bls12_381:")
raw("push r1")
save_ptr("BLS_TW_P1","r10"); save_ptr("BLS_TW_PR","r11")
load_slot("BLS_TW_X1","BLS_TW_P1",0); load_slot("BLS_TW_Y1","BLS_TW_P1",1)
dbl2("BLS_TW_L","BLS_TW_Y1"); inv2("BLS_TW_L","BLS_TW_L"); sm2("BLS_TW_L","BLS_TW_L","BLS_THREE")
mul2("BLS_TW_L","BLS_TW_L","BLS_TW_X1"); mul2("BLS_TW_L","BLS_TW_L","BLS_TW_X1")
sq2("BLS_TW_X3","BLS_TW_L"); sub2("BLS_TW_X3","BLS_TW_X3","BLS_TW_X1"); sub2("BLS_TW_X3","BLS_TW_X3","BLS_TW_X1")
sub2("BLS_TW_Y3","BLS_TW_X1","BLS_TW_X3"); mul2("BLS_TW_T2","BLS_TW_L","BLS_TW_Y3"); sub2("BLS_TW_Y3","BLS_TW_T2","BLS_TW_Y1")
store_slot("BLS_TW_X3","BLS_TW_PR",0); store_slot("BLS_TW_Y3","BLS_TW_PR",1)
raw("pop r1"); raw("ret"); w("")

# ---- add_twist (non-complete) ----
w("; void zisklib_add_twist_bls12_381(const u64* p1, const u64* p2, u64* result)  [24,24,24]")
w("zisklib_add_twist_bls12_381:")
raw("push r1")
save_ptr("BLS_TW_P1","r10"); save_ptr("BLS_TW_P2","r11"); save_ptr("BLS_TW_PR","r12")
ptr_comp("r10","BLS_TW_P1",0); ptr_comp("r11","BLS_TW_P2",0); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(ltw_add_generic)")
ptr_comp("r10","BLS_TW_P1",1); ptr_comp("r11","BLS_TW_P2",1); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(ltw_add_inf)")
raw("copyb(0, BLS_TW_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_TW_PR) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("call zisklib_dbl_twist_bls12_381"); raw("pop r1"); raw("ret")
w("ltw_add_inf:")
zero24_at("BLS_TW_PR"); raw("pop r1"); raw("ret")
w("ltw_add_generic:")
load_slot("BLS_TW_X1","BLS_TW_P1",0); load_slot("BLS_TW_Y1","BLS_TW_P1",1)
load_slot("BLS_TW_X2","BLS_TW_P2",0); load_slot("BLS_TW_Y2","BLS_TW_P2",1)
sub2("BLS_TW_DEN","BLS_TW_X2","BLS_TW_X1"); inv2("BLS_TW_DEN","BLS_TW_DEN")
sub2("BLS_TW_L","BLS_TW_Y2","BLS_TW_Y1"); mul2("BLS_TW_L","BLS_TW_L","BLS_TW_DEN")
sq2("BLS_TW_X3","BLS_TW_L"); sub2("BLS_TW_X3","BLS_TW_X3","BLS_TW_X1"); sub2("BLS_TW_X3","BLS_TW_X3","BLS_TW_X2")
sub2("BLS_TW_Y3","BLS_TW_X1","BLS_TW_X3"); mul2("BLS_TW_T2","BLS_TW_L","BLS_TW_Y3"); sub2("BLS_TW_Y3","BLS_TW_T2","BLS_TW_Y1")
store_slot("BLS_TW_X3","BLS_TW_PR",0); store_slot("BLS_TW_Y3","BLS_TW_PR",1)
raw("pop r1"); raw("ret"); w("")

# ---- add_complete_twist ----
w("; void zisklib_add_complete_twist_bls12_381(const u64* p1, const u64* p2, u64* result)")
w("zisklib_add_complete_twist_bls12_381:")
raw("push r1")
save_ptr("BLS_AC_P1","r10"); save_ptr("BLS_AC_P2","r11"); save_ptr("BLS_AC_PR","r12")
raw("copyb(0, BLS_AC_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_G2_IDENTITY) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(ltw_ac_chk2)")
memcpy24("BLS_AC_P2","BLS_AC_PR"); raw("pop r1"); raw("ret")
w("ltw_ac_chk2:")
raw("copyb(0, BLS_AC_P2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_G2_IDENTITY) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(ltw_ac_add)")
memcpy24("BLS_AC_P1","BLS_AC_PR"); raw("pop r1"); raw("ret")
w("ltw_ac_add:")
raw("copyb(0, BLS_AC_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_AC_P2) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("copyb(0, BLS_AC_PR) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12")
raw("call zisklib_add_twist_bls12_381"); raw("pop r1"); raw("ret"); w("")

# ---- dbl_complete_twist ----
w("; void zisklib_dbl_complete_twist_bls12_381(const u64* p, u64* result)")
w("zisklib_dbl_complete_twist_bls12_381:")
raw("push r1")
save_ptr("BLS_DC_P","r10"); save_ptr("BLS_DC_PR","r11")
raw("copyb(0, BLS_DC_P) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_G2_IDENTITY) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(ltw_dc_dbl)")
zero24_at("BLS_DC_PR"); raw("pop r1"); raw("ret")
w("ltw_dc_dbl:")
raw("copyb(0, BLS_DC_P) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_DC_PR) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("call zisklib_dbl_twist_bls12_381"); raw("pop r1"); raw("ret"); w("")

# ---- sub_complete_twist = add_complete(p1, neg(p2)) ----
w("; void zisklib_sub_complete_twist_bls12_381(const u64* p1, const u64* p2, u64* result)")
w("zisklib_sub_complete_twist_bls12_381:")
raw("push r1")
save_ptr("BLS_SUB2_P1","r10"); save_ptr("BLS_SUB2_PR","r12")
raw("copyb(0, r11) -> r10")   # r10 = p2
raw("copyb(0, BLS_SUB2_NEG) -> r11"); raw("call zisklib_neg_twist_bls12_381")   # neg(p2) -> NEG
raw("copyb(0, BLS_SUB2_P1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_SUB2_NEG) -> r11")
raw("copyb(0, BLS_SUB2_PR) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12")
raw("call zisklib_add_complete_twist_bls12_381"); raw("pop r1"); raw("ret"); w("")

# ---- scalar_mul_twist (double-and-add, all 256 bits, complete ops) ----
w("; u64 zisklib_scalar_mul_twist_bls12_381(const u64* p, const u64* k, u64* result)")
w(";   [r10=p, r11=k, r12=result] -> r10 = 1 if result is identity. k reduced mod R.")
w("zisklib_scalar_mul_twist_bls12_381:")
raw("push r1")
save_ptr("BLS_SMT_P","r10"); save_ptr("BLS_SMT_PR","r12")
raw("copyb(0, r11) -> r10"); raw("copyb(0, BLS_SMT_K) -> r11"); raw("call zisklib_reduce_fr_bls12_381")
# acc = identity
raw("copyb(0, BLS_SMT_ACC) -> r11")
for i in range(24): raw(f"copyb(r11, 0) -> 8[a + {i*8}]")
raw("copyb(0, 255) -> r13")
w("ltw_sm_loop:")
# acc = dbl_complete(acc)
raw("copyb(0, BLS_SMT_ACC) -> r10"); raw("copyb(0, BLS_SMT_ACC2) -> r11"); raw("call zisklib_dbl_complete_twist_bls12_381")
memcpy24("BLS_SMT_ACC2_addr","BLS_SMT_ACC_addr") if False else None
raw("copyb(0, BLS_SMT_ACC2) -> r10"); raw("copyb(0, BLS_SMT_ACC) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
# bit r13 of k
raw("srl(r13, 6) -> r5"); raw("and(r13, 63) -> r6"); raw("sll(r5, 3) -> r5")
raw("copyb(0, BLS_SMT_K) -> r7"); raw("add(r7, r5) -> r7"); raw("copyb(r7, 8[a + 0]) -> r7")
raw("srl(r7, r6) -> r7"); raw("and(r7, 1) -> r7")
raw("eq(r7, 0), j(ltw_sm_noadd)")
raw("copyb(0, BLS_SMT_ACC) -> r10"); raw("copyb(0, BLS_SMT_P) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("copyb(0, BLS_SMT_ACC2) -> r12"); raw("call zisklib_add_complete_twist_bls12_381")
raw("copyb(0, BLS_SMT_ACC2) -> r10"); raw("copyb(0, BLS_SMT_ACC) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
w("ltw_sm_noadd:")
raw("eq(r13, 0), j(ltw_sm_done)")
raw("sub(r13, 1) -> r13"); raw("jump(ltw_sm_loop)")
w("ltw_sm_done:")
raw("copyb(0, BLS_SMT_ACC) -> r10"); raw("copyb(0, BLS_SMT_PR) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_SMT_ACC) -> r10"); raw("copyb(0, BLS_G2_IDENTITY) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret"); w("")

open(_OUT+"/bls12_381/twist.zisk","w").write("\n".join(out)+"\n")
print("wrote bls twist.zisk,", len(out), "lines")
