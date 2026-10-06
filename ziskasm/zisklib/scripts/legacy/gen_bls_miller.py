#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/miller_loop.zisk (single). Loop over binary
# |x| (X_ABS_BIN_BE), then final conjugate. Line coeffs hinted (fcalls 14/15) +
# is_tangent/is_line checks. line_eval: coeff1=μ·(-yp'), coeff2=λ·xp' (mul_fp2,
# xp'/yp' are Fp2). Prefix lml_.
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
def sm2(d,x,s):
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {s}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_scalar_mul_fp2{S}")
def ld(slot, ptrslot, off_words):   # slot(12) = [ptrslot] + off
    raw(f"copyb(0, {ptrslot}) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
    if off_words: raw(f"add(r10, {off_words*8}) -> r10")
    raw(f"copyb(0, {slot}) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
def save_ptr(slot,reg): raw(f"copyb(0, {slot}) -> r5"); raw(f"copyb(r5, {reg}) -> 8[a + 0]")
def loadp(reg, ptrslot): raw(f"copyb(0, {ptrslot}) -> {reg}"); raw(f"copyb({reg}, 8[a + 0]) -> {reg}")

w("; ============================================================================")
w("; bls12_381/miller_loop.zisk - optimal-ate Miller loop (single pair). GENERATED")
w("; (scripts/legacy/gen_bls_miller.py). Loop over binary |x| then conjugate. Line")
w("; coeffs hinted (fcalls 14/15) + is_tangent/is_line checks. Prefix lml_.")
w("; ============================================================================")
w("")
XB=[1,1,0,1,0,0,1,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
w("const u64 BLS_ML_LOOP[64] = "+", ".join(str(b) for b in XB))
w("const u64 BLS_ML_THREE[6] = 3, 0, 0, 0, 0, 0")
w("")
w("u64 BLS_ML_P[1] = 0"); w("u64 BLS_ML_Q[1] = 0"); w("u64 BLS_ML_PR[1] = 0")
w("u64 BLS_ML_WP1[1] = 0"); w("u64 BLS_ML_WP2[1] = 0")
w("u64 BLS_ML_XP[6] = 0,0,0,0,0,0"); w("u64 BLS_ML_YP[6] = 0,0,0,0,0,0")
w("u64 BLS_ML_XPP[12] = "+", ".join(["0"]*12)); w("u64 BLS_ML_YPP[12] = "+", ".join(["0"]*12)); w("u64 BLS_ML_NEGYPP[12] = "+", ".join(["0"]*12))
w("u64 BLS_ML_R[24] = "+", ".join(["0"]*24)); w("u64 BLS_ML_R2[24] = "+", ".join(["0"]*24))
w("u64 BLS_ML_F[72] = "+", ".join(["0"]*72))
w("u64 BLS_ML_LAMMU[24] = "+", ".join(["0"]*24))
w("u64 BLS_ML_L[24] = "+", ".join(["0"]*24))
for s in ["X3","Y3","T1","T2","C1","C2"]:
    w(f"u64 BLS_ML_{s}[12] = "+", ".join(["0"]*12))
w("u64 BLS_ML_ILQ1[1] = 0"); w("u64 BLS_ML_ILQ2[1] = 0")
w("")
LAM="BLS_ML_LAMMU"

# ---- fcall coeff routines ----
w("; void zisklib_ml_dbl_coeffs_bls12_381(const u64* r)  -> BLS_ML_LAMMU[24]=(lam,mu)")
w("zisklib_ml_dbl_coeffs_bls12_381:")
raw("fcall_param(24, r10) -> r14")
raw("fcall(FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS, 0) -> r14")
raw("copyb(0, BLS_ML_LAMMU) -> r6")
raw("copyb(0, [FREE_INPUT]) -> r7"); raw("copyb(r6, r7) -> 8[a + 0]"); raw("add(r6, 8) -> r6")
raw("copyb(0, 23) -> r15")
w("lml_dc_rd:")
raw("eq(r15, 0), j(lml_dc_done)")
raw("fcall_get(0, 0) -> r14"); raw("copyb(0, [FREE_INPUT]) -> r7"); raw("copyb(r6, r7) -> 8[a + 0]"); raw("add(r6, 8) -> r6"); raw("sub(r15, 1) -> r15"); raw("jump(lml_dc_rd)")
w("lml_dc_done:")
raw("ret"); w("")

w("; void zisklib_ml_add_coeffs_bls12_381(const u64* r, const u64* q)  -> BLS_ML_LAMMU")
w("zisklib_ml_add_coeffs_bls12_381:")
raw("fcall_param(24, r10) -> r14")
raw("fcall_param(24, r11) -> r14")
raw("fcall(FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS, 0) -> r14")
raw("copyb(0, BLS_ML_LAMMU) -> r6")
raw("copyb(0, [FREE_INPUT]) -> r7"); raw("copyb(r6, r7) -> 8[a + 0]"); raw("add(r6, 8) -> r6")
raw("copyb(0, 23) -> r15")
w("lml_ac_rd:")
raw("eq(r15, 0), j(lml_ac_done)")
raw("fcall_get(0, 0) -> r14"); raw("copyb(0, [FREE_INPUT]) -> r7"); raw("copyb(r6, r7) -> 8[a + 0]"); raw("add(r6, 8) -> r6"); raw("sub(r15, 1) -> r15"); raw("jump(lml_ac_rd)")
w("lml_ac_done:")
raw("ret"); w("")

# ---- line_eval: L = [μ·(-YPP)] ‖ [λ·XPP] ----
w("; void zisklib_ml_line_eval_bls12_381()  uses LAMMU, XPP, NEGYPP -> BLS_ML_L")
w("zisklib_ml_line_eval_bls12_381:")
raw("push r1")
# coeff1 = mul_fp2(mu, NEGYPP) -> L[0..12]   (mu = LAMMU+12)
raw("copyb(0, BLS_ML_LAMMU) -> r10"); raw("add(r10, 96) -> r10"); raw("copyb(0, BLS_ML_NEGYPP) -> r11"); raw("copyb(0, BLS_ML_L) -> r12"); raw(f"call zisklib_mul_fp2{S}")
# coeff2 = mul_fp2(lam, XPP) -> L[12..24]
raw("copyb(0, BLS_ML_LAMMU) -> r10"); raw("copyb(0, BLS_ML_XPP) -> r11"); raw("copyb(0, BLS_ML_L) -> r12"); raw("add(r12, 96) -> r12"); raw(f"call zisklib_mul_fp2{S}")
raw("pop r1"); raw("ret"); w("")

# ---- dbl_wh(q) using LAMMU -> R2 ----
w("; void zisklib_ml_dbl_wh_bls12_381(const u64* q)  uses LAMMU -> BLS_ML_R2")
w("zisklib_ml_dbl_wh_bls12_381:")
raw("push r1")
save_ptr("BLS_ML_WP1","r10")
ld("BLS_ML_T1","BLS_ML_WP1",0)   # x
sq2("BLS_ML_X3",LAM); dbl2("BLS_ML_T2","BLS_ML_T1"); sub2("BLS_ML_X3","BLS_ML_X3","BLS_ML_T2")
mul2("BLS_ML_Y3",LAM,"BLS_ML_X3")
raw("copyb(0, BLS_ML_LAMMU) -> r10"); raw("add(r10, 96) -> r10"); raw("copyb(0, BLS_ML_Y3) -> r11"); raw("copyb(0, BLS_ML_T2) -> r12"); raw(f"call zisklib_add_fp2{S}")
neg2("BLS_ML_Y3","BLS_ML_T2")
raw("copyb(0, BLS_ML_X3) -> r10"); raw("copyb(0, BLS_ML_R2) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_ML_Y3) -> r10"); raw("copyb(0, BLS_ML_R2) -> r11"); raw("add(r11, 96) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ---- add_wh(q1, q2) using LAMMU -> R2 ----
w("; void zisklib_ml_add_wh_bls12_381(const u64* q1, const u64* q2)  uses LAMMU -> BLS_ML_R2")
w("zisklib_ml_add_wh_bls12_381:")
raw("push r1")
save_ptr("BLS_ML_WP1","r10"); save_ptr("BLS_ML_WP2","r11")
ld("BLS_ML_T1","BLS_ML_WP1",0)   # x1
ld("BLS_ML_T2","BLS_ML_WP2",0)   # x2
sq2("BLS_ML_X3",LAM); sub2("BLS_ML_X3","BLS_ML_X3","BLS_ML_T1"); sub2("BLS_ML_X3","BLS_ML_X3","BLS_ML_T2")
mul2("BLS_ML_Y3",LAM,"BLS_ML_X3")
raw("copyb(0, BLS_ML_LAMMU) -> r10"); raw("add(r10, 96) -> r10"); raw("copyb(0, BLS_ML_Y3) -> r11"); raw("copyb(0, BLS_ML_T2) -> r12"); raw(f"call zisklib_add_fp2{S}")
neg2("BLS_ML_Y3","BLS_ML_T2")
raw("copyb(0, BLS_ML_X3) -> r10"); raw("copyb(0, BLS_ML_R2) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_ML_Y3) -> r10"); raw("copyb(0, BLS_ML_R2) -> r11"); raw("add(r11, 96) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ---- line_check(q) -> r10 : y == lam*x + mu ----
w("; u64 zisklib_ml_line_check_bls12_381(const u64* q)  uses LAMMU -> r10")
w("zisklib_ml_line_check_bls12_381:")
raw("push r1")
save_ptr("BLS_ML_WP1","r10")
ld("BLS_ML_C1","BLS_ML_WP1",0)   # x
ld("BLS_ML_C2","BLS_ML_WP1",12)  # y
mul2("BLS_ML_T1",LAM,"BLS_ML_C1")
raw("copyb(0, BLS_ML_T1) -> r10"); raw("copyb(0, BLS_ML_LAMMU) -> r11"); raw("add(r11, 96) -> r11"); raw("copyb(0, BLS_ML_T1) -> r12"); raw(f"call zisklib_add_fp2{S}")
raw("copyb(0, BLS_ML_T1) -> r10"); raw("copyb(0, BLS_ML_C2) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret"); w("")

# ---- is_tangent(q) -> r10 ----
w("; u64 zisklib_ml_is_tangent_bls12_381(const u64* q)  uses LAMMU -> r10")
w("zisklib_ml_is_tangent_bls12_381:")
raw("push r1")
save_ptr("BLS_ML_WP2","r10")
# y == 0 ? -> false
raw("copyb(0, BLS_ML_WP2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 96) -> r10")
raw("copyb(0, BLS_ZERO) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("ltu(0, r10), j(lml_it_false)")
# c = line_check(q)
raw("copyb(0, BLS_ML_WP2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_ml_line_check_bls12_381")
raw("eq(r10, 0), j(lml_it_false)")
# lhs = 2*lam*y ; rhs = 3*x^2
ld("BLS_ML_C1","BLS_ML_WP2",0); ld("BLS_ML_C2","BLS_ML_WP2",12)
mul2("BLS_ML_T1",LAM,"BLS_ML_C2"); dbl2("BLS_ML_T1","BLS_ML_T1")
sq2("BLS_ML_C2","BLS_ML_C1"); sm2("BLS_ML_C2","BLS_ML_C2","BLS_ML_THREE")
raw("copyb(0, BLS_ML_T1) -> r10"); raw("copyb(0, BLS_ML_C2) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret")
w("lml_it_false:")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret"); w("")

# ---- is_line(q1, q2) -> r10 ----
w("; u64 zisklib_ml_is_line_bls12_381(const u64* q1, const u64* q2)  uses LAMMU -> r10")
w("zisklib_ml_is_line_bls12_381:")
raw("push r1")
save_ptr("BLS_ML_ILQ1","r10"); save_ptr("BLS_ML_ILQ2","r11")
raw("copyb(0, BLS_ML_ILQ1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_ML_ILQ2) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("ltu(0, r10), j(lml_il_false)")
raw("copyb(0, BLS_ML_ILQ1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_ml_line_check_bls12_381")
raw("eq(r10, 0), j(lml_il_false)")
raw("copyb(0, BLS_ML_ILQ2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_ml_line_check_bls12_381")
raw("pop r1"); raw("ret")
w("lml_il_false:")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret"); w("")

# ================= main =================
w("; void zisklib_miller_loop_bls12_381(const u64* p, const u64* q, u64* result)  [12,24,72]")
w("zisklib_miller_loop_bls12_381:")
raw("push r1")
save_ptr("BLS_ML_Q","r11"); save_ptr("BLS_ML_PR","r12")
raw("copyb(0, BLS_ML_P) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")   # save p
# ypfp = inv_fp(p.y)
loadp("r10","BLS_ML_P"); raw("add(r10, 48) -> r10"); raw("copyb(0, BLS_ML_YP) -> r11"); raw(f"call zisklib_inv_fp{S}")
# xpfp = neg_fp(p.x) ; xpfp = mul_fp(xpfp, ypfp)
loadp("r10","BLS_ML_P"); raw("copyb(0, BLS_ML_XP) -> r11"); raw(f"call zisklib_neg_fp{S}")
raw("copyb(0, BLS_ML_XP) -> r10"); raw("copyb(0, BLS_ML_YP) -> r11"); raw("copyb(0, BLS_ML_XP) -> r12"); raw(f"call zisklib_mul_fp{S}")
# XPP = scalar_mul_fp2(EXT_U_INV, xpfp) ; YPP = scalar_mul_fp2(EXT_U_INV, ypfp)
raw("copyb(0, BLS_EXT_U_INV) -> r10"); raw("copyb(0, BLS_ML_XP) -> r11"); raw("copyb(0, BLS_ML_XPP) -> r12"); raw(f"call zisklib_scalar_mul_fp2{S}")
raw("copyb(0, BLS_EXT_U_INV) -> r10"); raw("copyb(0, BLS_ML_YP) -> r11"); raw("copyb(0, BLS_ML_YPP) -> r12"); raw(f"call zisklib_scalar_mul_fp2{S}")
neg2("BLS_ML_NEGYPP","BLS_ML_YPP")
# r = q
loadp("r10","BLS_ML_Q"); raw("copyb(0, BLS_ML_R) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
# f = one
raw("copyb(0, BLS_ML_F) -> r11")
for i in range(72): raw(f"copyb(r11, {1 if i==0 else 0}) -> 8[a + {i*8}]")
raw("copyb(0, 1) -> r13")
w("lml_loop:")
raw("eq(r13, 64), j(lml_final)")
# dbl_coeffs(r) ; assert is_tangent
raw("copyb(0, BLS_ML_R) -> r10"); raw("call zisklib_ml_dbl_coeffs_bls12_381")
raw("copyb(0, BLS_ML_R) -> r10"); raw("call zisklib_ml_is_tangent_bls12_381"); raw("eq(r10, 0), j(lml_bad)")
# f = f^2 ; l = line_eval ; f = sparse_mul(f, l)
raw("copyb(0, BLS_ML_F) -> r10"); raw("copyb(0, BLS_ML_F) -> r11"); raw(f"call zisklib_square_fp12{S}")
raw("call zisklib_ml_line_eval_bls12_381")
raw("copyb(0, BLS_ML_F) -> r10"); raw("copyb(0, BLS_ML_L) -> r11"); raw("copyb(0, BLS_ML_F) -> r12"); raw(f"call zisklib_sparse_mul_fp12{S}")
# r = dbl_wh(r) ; copy R2->R
raw("copyb(0, BLS_ML_R) -> r10"); raw("call zisklib_ml_dbl_wh_bls12_381")
raw("copyb(0, BLS_ML_R2) -> r10"); raw("copyb(0, BLS_ML_R) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
# bit = LOOP[r13]
raw("copyb(0, BLS_ML_LOOP) -> r5"); raw("sll(r13, 3) -> r6"); raw("add(r5, r6) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("eq(r6, 0), j(lml_next)")
# add_coeffs(r, q) ; assert is_line ; l=line_eval ; f=sparse_mul ; r=add_wh(r,q)
raw("copyb(0, BLS_ML_R) -> r10"); raw("copyb(0, BLS_ML_Q) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("call zisklib_ml_add_coeffs_bls12_381")
raw("copyb(0, BLS_ML_R) -> r10"); raw("copyb(0, BLS_ML_Q) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("call zisklib_ml_is_line_bls12_381"); raw("eq(r10, 0), j(lml_bad)")
raw("call zisklib_ml_line_eval_bls12_381")
raw("copyb(0, BLS_ML_F) -> r10"); raw("copyb(0, BLS_ML_L) -> r11"); raw("copyb(0, BLS_ML_F) -> r12"); raw(f"call zisklib_sparse_mul_fp12{S}")
raw("copyb(0, BLS_ML_R) -> r10"); raw("copyb(0, BLS_ML_Q) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("call zisklib_ml_add_wh_bls12_381")
raw("copyb(0, BLS_ML_R2) -> r10"); raw("copyb(0, BLS_ML_R) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
w("lml_next:")
raw("add(r13, 1) -> r13"); raw("jump(lml_loop)")
w("lml_final:")
# result = conjugate(f)
raw("copyb(0, BLS_ML_F) -> r10"); raw("copyb(0, BLS_ML_PR) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw(f"call zisklib_conjugate_fp12{S}")
raw("pop r1"); raw("ret")
w("lml_bad:")
raw("copyb(0, 0) -> r10, end"); w("")

open(_OUT+"/bls12_381/miller_loop.zisk","w").write("\n".join(out)+"\n")
print("wrote bls miller_loop.zisk,", len(out), "lines")
