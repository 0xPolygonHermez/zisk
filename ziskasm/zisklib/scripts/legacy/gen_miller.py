#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bn254/miller_loop.zisk (single miller_loop_bn254).
# Uses hinted line coeffs (fcalls 8/9). NOTE: hint-verification checks
# (is_tangent/is_line) are NOT yet emitted here (added in a later pass).
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
def neg2(d,x): f2un("neg",d,x)
def sm2(d,x,s):  # scalar_mul_fp2(x, s4) -> d
    raw(f"copyb(0, {x}) -> r10"); raw(f"copyb(0, {s}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw("call zisklib_scalar_mul_fp2_bn254")
def ld(slot, ptrslot, off_words):   # slot(8) = [ptrslot] + off
    raw(f"copyb(0, {ptrslot}) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
    if off_words: raw(f"add(r10, {off_words*8}) -> r10")
    raw(f"copyb(0, {slot}) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_memcpy")
def save_ptr(slot,reg): raw(f"copyb(0, {slot}) -> r5"); raw(f"copyb(r5, {reg}) -> 8[a + 0]")
def loadp(reg, ptrslot): raw(f"copyb(0, {ptrslot}) -> {reg}"); raw(f"copyb({reg}, 8[a + 0]) -> {reg}")

w("; ============================================================================")
w("; bn254/miller_loop.zisk - optimal-ate Miller loop over BN254 (single pair).")
w("; GENERATED (scripts/legacy/gen_miller.py). Uses hinted line coeffs (fcalls 8/9).")
w("; Prefix ml_. (hint-verification checks added in a later pass.)")
w("; ============================================================================")
w("")
# LOOP_LENGTH remapped: 0->0, 1->1, -1->2
LL=[1,1,0,1,0,0,-1,0,1,1,0,0,0,-1,0,0,1,1,0,0,-1,0,0,0,0,0,1,0,0,-1,0,0,1,1,1,0,0,0,0,-1,0,1,0,0,-1,0,1,1,0,0,1,0,0,-1,1,0,0,-1,0,1,0,1,0,0,0]
rm=[0 if b==0 else (1 if b==1 else 2) for b in LL]
w("const u64 ML_LOOP[65] = "+", ".join(str(v) for v in rm))
w("")
w("u64 ML_P[1] = 0"); w("u64 ML_Q[1] = 0"); w("u64 ML_PR[1] = 0")
w("u64 ML_WP1[1] = 0"); w("u64 ML_WP2[1] = 0")
w("u64 ML_XPP[4] = 0,0,0,0"); w("u64 ML_YPP[4] = 0,0,0,0"); w("u64 ML_NEGYPP[4] = 0,0,0,0")
w("u64 ML_TMP4[4] = 0,0,0,0")
w("u64 ML_R[16] = "+", ".join(["0"]*16)); w("u64 ML_R2[16] = "+", ".join(["0"]*16))
w("u64 ML_F[48] = "+", ".join(["0"]*48))
w("u64 ML_LAMMU[16] = "+", ".join(["0"]*16))
w("u64 ML_L[16] = "+", ".join(["0"]*16))
w("u64 ML_QP[16] = "+", ".join(["0"]*16)); w("u64 ML_QF[16] = "+", ".join(["0"]*16)); w("u64 ML_QF2[16] = "+", ".join(["0"]*16))
w("u64 ML_UT[16] = "+", ".join(["0"]*16))
for s in ["X3","Y3","T1","T2","C1","C2","C3"]:
    w(f"u64 ML_{s}[8] = 0,0,0,0,0,0,0,0")
w("u64 ML_ILQ1[1] = 0"); w("u64 ML_ILQ2[1] = 0")
w("")
LAM="ML_LAMMU"; MU="ML_LAMMU8"   # mu = ML_LAMMU + 8 words; we pass a pointer with offset
# helper: pointer to mu (ML_LAMMU+64 bytes) -- we make a label via add at call sites

# ---- fcall coeff routines ----
w("; void zisklib_ml_dbl_coeffs_bn254(const u64* r)  [r10=&r(16)] -> ML_LAMMU[16]=(lam,mu)")
w("zisklib_ml_dbl_coeffs_bn254:")
raw("fcall_param(16, r10) -> r14")
raw("fcall(FCALL_BN254_TWIST_DBL_LINE_COEFFS, 0) -> r14")
raw("copyb(0, ML_LAMMU) -> r6")
raw("copyb(0, [FREE_INPUT]) -> r7"); raw("copyb(r6, r7) -> 8[a + 0]"); raw("add(r6, 8) -> r6")
raw("copyb(0, 15) -> r15")
w("mldc_rd:")
raw("eq(r15, 0), j(mldc_done)")
raw("fcall_get(0, 0) -> r14"); raw("copyb(0, [FREE_INPUT]) -> r7"); raw("copyb(r6, r7) -> 8[a + 0]"); raw("add(r6, 8) -> r6"); raw("sub(r15, 1) -> r15"); raw("jump(mldc_rd)")
w("mldc_done:")
raw("ret"); w("")

w("; void zisklib_ml_add_coeffs_bn254(const u64* r, const u64* qp)  [r10,r11] -> ML_LAMMU")
w("zisklib_ml_add_coeffs_bn254:")
raw("fcall_param(16, r10) -> r14")
raw("fcall_param(16, r11) -> r14")
raw("fcall(FCALL_BN254_TWIST_ADD_LINE_COEFFS, 0) -> r14")
raw("copyb(0, ML_LAMMU) -> r6")
raw("copyb(0, [FREE_INPUT]) -> r7"); raw("copyb(r6, r7) -> 8[a + 0]"); raw("add(r6, 8) -> r6")
raw("copyb(0, 15) -> r15")
w("mlac_rd:")
raw("eq(r15, 0), j(mlac_done)")
raw("fcall_get(0, 0) -> r14"); raw("copyb(0, [FREE_INPUT]) -> r7"); raw("copyb(r6, r7) -> 8[a + 0]"); raw("add(r6, 8) -> r6"); raw("sub(r15, 1) -> r15"); raw("jump(mlac_rd)")
w("mlac_done:")
raw("ret"); w("")

# ---- line_eval: out16 = (lam*xp) ‖ (mu*negyp) ----
w("; void zisklib_ml_line_eval_bn254()  uses ML_LAMMU, ML_XPP, ML_NEGYPP -> ML_L")
w("zisklib_ml_line_eval_bn254:")
raw("push r1")
# coeff1 = scalar_mul_fp2(lam, xpp) -> ML_L[0..8]
raw("copyb(0, ML_LAMMU) -> r10"); raw("copyb(0, ML_XPP) -> r11"); raw("copyb(0, ML_L) -> r12"); raw("call zisklib_scalar_mul_fp2_bn254")
# coeff2 = scalar_mul_fp2(mu, negyp) -> ML_L[8..16]
raw("copyb(0, ML_LAMMU) -> r10"); raw("add(r10, 64) -> r10"); raw("copyb(0, ML_NEGYPP) -> r11"); raw("copyb(0, ML_L) -> r12"); raw("add(r12, 64) -> r12"); raw("call zisklib_scalar_mul_fp2_bn254")
raw("pop r1"); raw("ret"); w("")

# ---- dbl_wh(q) using ML_LAMMU -> ML_R2 ----
w("; void zisklib_ml_dbl_wh_bn254(const u64* q)  [r10=&q] uses ML_LAMMU -> ML_R2")
w("zisklib_ml_dbl_wh_bn254:")
raw("push r1")
save_ptr("ML_WP1","r10")
ld("ML_T1","ML_WP1",0)             # x = q[0..8]
# x3 = lam² - 2x
sq2("ML_X3","ML_LAMMU"); dbl2("ML_T2","ML_T1"); sub2("ML_X3","ML_X3","ML_T2")
# y3 = -(mu + lam*x3)
mul2("ML_Y3","ML_LAMMU","ML_X3")
raw("copyb(0, ML_LAMMU) -> r10"); raw("add(r10, 64) -> r10"); raw("copyb(0, ML_Y3) -> r11"); raw("copyb(0, ML_T2) -> r12"); raw("call zisklib_add_fp2_bn254")  # T2 = mu + y3
neg2("ML_Y3","ML_T2")
raw("copyb(0, ML_X3) -> r10"); raw("copyb(0, ML_R2) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, ML_Y3) -> r10"); raw("copyb(0, ML_R2) -> r11"); raw("add(r11, 64) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ---- add_wh(q1, q2) using ML_LAMMU -> ML_R2 ----
w("; void zisklib_ml_add_wh_bn254(const u64* q1, const u64* q2)  [r10,r11] uses ML_LAMMU -> ML_R2")
w("zisklib_ml_add_wh_bn254:")
raw("push r1")
save_ptr("ML_WP1","r10"); save_ptr("ML_WP2","r11")
ld("ML_T1","ML_WP1",0)             # x1
ld("ML_T2","ML_WP2",0)             # x2
sq2("ML_X3","ML_LAMMU"); sub2("ML_X3","ML_X3","ML_T1"); sub2("ML_X3","ML_X3","ML_T2")
mul2("ML_Y3","ML_LAMMU","ML_X3")
raw("copyb(0, ML_LAMMU) -> r10"); raw("add(r10, 64) -> r10"); raw("copyb(0, ML_Y3) -> r11"); raw("copyb(0, ML_T2) -> r12"); raw("call zisklib_add_fp2_bn254")
neg2("ML_Y3","ML_T2")
raw("copyb(0, ML_X3) -> r10"); raw("copyb(0, ML_R2) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, ML_Y3) -> r10"); raw("copyb(0, ML_R2) -> r11"); raw("add(r11, 64) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")


# ---- hint-verification checks ----
w("; u64 zisklib_ml_line_check_bn254(const u64* q)  [r10=&q] uses ML_LAMMU -> r10 (y==lam*x+mu)")
w("zisklib_ml_line_check_bn254:")
raw("push r1")
save_ptr("ML_WP1","r10")
ld("ML_C1","ML_WP1",0)   # x
ld("ML_C2","ML_WP1",8)   # y
mul2("ML_C3","ML_LAMMU","ML_C1")
raw("copyb(0, ML_C3) -> r10"); raw("copyb(0, ML_LAMMU) -> r11"); raw("add(r11, 64) -> r11"); raw("copyb(0, ML_C3) -> r12"); raw("call zisklib_add_fp2_bn254")
raw("copyb(0, ML_C3) -> r10"); raw("copyb(0, ML_C2) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret"); w("")

w("; u64 zisklib_ml_is_tangent_bn254(const u64* q)  [r10=&q] uses ML_LAMMU -> r10 bool")
w("zisklib_ml_is_tangent_bn254:")
raw("push r1")
save_ptr("ML_WP2","r10")
# y == 0 ? -> false
raw("copyb(0, ML_WP2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 64) -> r10")
raw("copyb(0, BN_G1_IDENTITY) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_eqn")
raw("ltu(0, r10), j(mit_false)")
# c = line_check(q)
raw("copyb(0, ML_WP2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_ml_line_check_bn254")
raw("eq(r10, 0), j(mit_false)")
# lhs = 2*lam*y ; rhs = 3*x^2
ld("ML_C1","ML_WP2",0); ld("ML_C2","ML_WP2",8)
mul2("ML_C3","ML_LAMMU","ML_C2"); dbl2("ML_C3","ML_C3")
sq2("ML_C2","ML_C1"); sm2("ML_C2","ML_C2","BN_THREE")
raw("copyb(0, ML_C3) -> r10"); raw("copyb(0, ML_C2) -> r11"); raw("copyb(0, 8) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret")
w("mit_false:")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret"); w("")

w("; u64 zisklib_ml_is_line_bn254(const u64* q1, const u64* q2)  [r10,r11] uses ML_LAMMU -> r10 bool")
w("zisklib_ml_is_line_bn254:")
raw("push r1")
save_ptr("ML_ILQ1","r10"); save_ptr("ML_ILQ2","r11")
# q1.x == q2.x ? -> false
raw("copyb(0, ML_ILQ1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, ML_ILQ2) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("copyb(0, 8) -> r12"); raw("call bn254_eqn")
raw("ltu(0, r10), j(mil_false)")
raw("copyb(0, ML_ILQ1) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_ml_line_check_bn254")
raw("eq(r10, 0), j(mil_false)")
raw("copyb(0, ML_ILQ2) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_ml_line_check_bn254")
raw("pop r1"); raw("ret")
w("mil_false:")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret"); w("")

# ================= main =================
w("; void zisklib_miller_loop_bn254(const u64* p, const u64* q, u64* result)  [8,16,48]")
w("zisklib_miller_loop_bn254:")
raw("push r1")
save_ptr("ML_Q","r11"); save_ptr("ML_PR","r12")
# ypp = inv_fp(p.y)
loadp("r10","ML_P") if False else None
raw("copyb(0, ML_P) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")  # save p ptr (r10 still = p at entry)
loadp("r10","ML_P"); raw("add(r10, 32) -> r10"); raw("copyb(0, ML_YPP) -> r11"); raw("call zisklib_inv_fp_bn254")
# xpp = neg_fp(p.x); xpp = mul_fp(xpp, ypp)
loadp("r10","ML_P"); raw("copyb(0, ML_XPP) -> r11"); raw("call zisklib_neg_fp_bn254")
raw("copyb(0, ML_XPP) -> r10"); raw("copyb(0, ML_YPP) -> r11"); raw("copyb(0, ML_XPP) -> r12"); raw("call zisklib_mul_fp_bn254")
# negypp = neg_fp(ypp)
raw("copyb(0, ML_YPP) -> r10"); raw("copyb(0, ML_NEGYPP) -> r11"); raw("call zisklib_neg_fp_bn254")
# r = q
loadp("r10","ML_Q"); raw("copyb(0, ML_R) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
# f = one
raw("copyb(0, ML_F) -> r11")
for i in range(48): raw(f"copyb(r11, {1 if i==0 else 0}) -> 8[a + {i*8}]")
# loop
raw("copyb(0, 1) -> r13")
w("ml_loop:")
raw("eq(r13, 65), j(ml_final)")
# dbl_coeffs(r)
raw("copyb(0, ML_R) -> r10"); raw("call zisklib_ml_dbl_coeffs_bn254")
# assert is_tangent(r)
raw("copyb(0, ML_R) -> r10"); raw("call zisklib_ml_is_tangent_bn254"); raw("eq(r10, 0), j(ml_bad)")
# f = f^2
raw("copyb(0, ML_F) -> r10"); raw("copyb(0, ML_F) -> r11"); raw("call zisklib_square_fp12_bn254")
# l = line_eval ; f = sparse_mul(f, l)
raw("call zisklib_ml_line_eval_bn254")
raw("copyb(0, ML_F) -> r10"); raw("copyb(0, ML_L) -> r11"); raw("copyb(0, ML_F) -> r12"); raw("call zisklib_sparse_mul_fp12_bn254")
# r = dbl_wh(r) ; copy R2->R
raw("copyb(0, ML_R) -> r10"); raw("call zisklib_ml_dbl_wh_bn254")
raw("copyb(0, ML_R2) -> r10"); raw("copyb(0, ML_R) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
# bit = ML_LOOP[r13]
raw("copyb(0, ML_LOOP) -> r5"); raw("sll(r13, 3) -> r6"); raw("add(r5, r6) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("eq(r6, 0), j(ml_next)")
# qp = (bit==1)? q : neg_twist(q)
raw("eq(r6, 1), j(ml_qpos)")
loadp("r10","ML_Q"); raw("copyb(0, ML_QP) -> r11"); raw("call zisklib_neg_twist_bn254"); raw("jump(ml_qdone)")
w("ml_qpos:")
loadp("r10","ML_Q"); raw("copyb(0, ML_QP) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
w("ml_qdone:")
# add_coeffs(r, qp); l=line_eval; f=sparse_mul; r=add_wh(r,qp)
raw("copyb(0, ML_R) -> r10"); raw("copyb(0, ML_QP) -> r11"); raw("call zisklib_ml_add_coeffs_bn254")
raw("copyb(0, ML_R) -> r10"); raw("copyb(0, ML_QP) -> r11"); raw("call zisklib_ml_is_line_bn254"); raw("eq(r10, 0), j(ml_bad)")
raw("call zisklib_ml_line_eval_bn254")
raw("copyb(0, ML_F) -> r10"); raw("copyb(0, ML_L) -> r11"); raw("copyb(0, ML_F) -> r12"); raw("call zisklib_sparse_mul_fp12_bn254")
raw("copyb(0, ML_R) -> r10"); raw("copyb(0, ML_QP) -> r11"); raw("call zisklib_ml_add_wh_bn254")
raw("copyb(0, ML_R2) -> r10"); raw("copyb(0, ML_R) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
w("ml_next:")
raw("add(r13, 1) -> r13"); raw("jump(ml_loop)")
w("ml_final:")
# qf = utf(q)
loadp("r10","ML_Q"); raw("copyb(0, ML_QF) -> r11"); raw("call zisklib_utf_endomorphism_twist_bn254")
# add_coeffs(r, qf); line; sparse; r = add_wh(r, qf)
raw("copyb(0, ML_R) -> r10"); raw("copyb(0, ML_QF) -> r11"); raw("call zisklib_ml_add_coeffs_bn254")
raw("copyb(0, ML_R) -> r10"); raw("copyb(0, ML_QF) -> r11"); raw("call zisklib_ml_is_line_bn254"); raw("eq(r10, 0), j(ml_bad)")
raw("call zisklib_ml_line_eval_bn254")
raw("copyb(0, ML_F) -> r10"); raw("copyb(0, ML_L) -> r11"); raw("copyb(0, ML_F) -> r12"); raw("call zisklib_sparse_mul_fp12_bn254")
raw("copyb(0, ML_R) -> r10"); raw("copyb(0, ML_QF) -> r11"); raw("call zisklib_ml_add_wh_bn254")
raw("copyb(0, ML_R2) -> r10"); raw("copyb(0, ML_R) -> r11"); raw("copyb(0, 16) -> r12"); raw("call bn254_memcpy")
# qf2 = neg_twist(utf(qf))
raw("copyb(0, ML_QF) -> r10"); raw("copyb(0, ML_UT) -> r11"); raw("call zisklib_utf_endomorphism_twist_bn254")
raw("copyb(0, ML_UT) -> r10"); raw("copyb(0, ML_QF2) -> r11"); raw("call zisklib_neg_twist_bn254")
# add_coeffs(r, qf2); line; sparse
raw("copyb(0, ML_R) -> r10"); raw("copyb(0, ML_QF2) -> r11"); raw("call zisklib_ml_add_coeffs_bn254")
raw("copyb(0, ML_R) -> r10"); raw("copyb(0, ML_QF2) -> r11"); raw("call zisklib_ml_is_line_bn254"); raw("eq(r10, 0), j(ml_bad)")
raw("call zisklib_ml_line_eval_bn254")
raw("copyb(0, ML_F) -> r10"); raw("copyb(0, ML_L) -> r11"); raw("copyb(0, ML_F) -> r12"); raw("call zisklib_sparse_mul_fp12_bn254")
# result = f
raw("copyb(0, ML_F) -> r10"); raw("copyb(0, ML_PR) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 48) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret")
w("ml_bad:")
raw("copyb(0, 0) -> r10, end"); w("")

open(_OUT+"/bn254/miller_loop.zisk","w").write("\n".join(out)+"\n")
print("wrote miller_loop.zisk,", len(out), "lines")
