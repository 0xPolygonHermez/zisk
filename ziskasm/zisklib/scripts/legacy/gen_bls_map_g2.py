#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/map_g2.zisk (G2 map_to_curve): Fp2 SSWU +
# 3-isogeny + clear_cofactor_twist. Mirrors map_to_curve.rs G2 path. Prefix lm2_.
import re
RS=_REPO+"/ziskos/entrypoint/src/zisklib/lib/bls12_381/constants.rs"
rs=open(RS).read()
def hx(s): return int(s.replace("_","").replace("0x",""),16)
def const12(name):
    m=re.search(r"pub const %s:\s*\[u64;\s*12\]\s*=\s*(?:\[([^\]]+)\]|([^;]+));"%name, rs, re.S)
    body=m.group(1) if m.group(1) else m.group(2)
    nums=re.findall(r"0x[0-9A-Fa-f_]+|\b\d+\b", body)
    return [hx(x) if x.lower().startswith("0x") else int(x) for x in nums][:12]
def table12(name):
    m=re.search(r"pub const %s:\s*\[\[u64;\s*12\];\s*\d+\]\s*=\s*\[(.*?)\];"%name, rs, re.S)
    nums=re.findall(r"0x[0-9A-Fa-f_]+|\b\d+\b", m.group(1))
    v=[hx(x) if x.lower().startswith("0x") else int(x) for x in nums]
    assert len(v)%12==0, (name,len(v))
    return v

out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)
S="_bls12_381"
def emitc(name,vals): w(f"const u64 {name}[{len(vals)}] = "+", ".join("0x%016x"%x for x in vals))

w("; ============================================================================")
w("; bls12_381/map_g2.zisk - map_to_curve for G2 (EIP-2537 MAP_FP2_TO_G2). GENERATED")
w("; (scripts/legacy/gen_bls_map_g2.py). Fp2 SSWU -> 3-isogeny -> clear_cofactor_twist.")
w("; Prefix lm2_.")
w("; ============================================================================")
w("")
emitc("BLS_ISO_A_G2",const12("ISO_A_G2"))
emitc("BLS_ISO_B_G2",const12("ISO_B_G2"))
# SWU_Z_G2 = -(2+u) = (P-2) + (P-1)*u ; constants.rs defines it via P[i] exprs (regex can't parse)
PLIMB=[0xB9FEFFFFFFFFAAAB,0x1EABFFFEB153FFFF,0x6730D2A0F6B0F624,0x64774B84F38512BF,0x4B1BA7B6434BACD7,0x1A0111EA397FE69A]
emitc("BLS_SWU_Z_G2",[PLIMB[0]-2]+PLIMB[1:]+[PLIMB[0]-1]+PLIMB[1:])
emitc("BLS_ISO_X_NUM_G2",table12("ISO_X_NUM_G2"))
emitc("BLS_ISO_X_DEN_G2",table12("ISO_X_DEN_G2"))
emitc("BLS_ISO_Y_NUM_G2",table12("ISO_Y_NUM_G2"))
emitc("BLS_ISO_Y_DEN_G2",table12("ISO_Y_DEN_G2"))
NXN=len(table12("ISO_X_NUM_G2"))//12; NXD=len(table12("ISO_X_DEN_G2"))//12
NYN=len(table12("ISO_Y_NUM_G2"))//12; NYD=len(table12("ISO_Y_DEN_G2"))//12
w("const u64 BLS_F2_ONE12[12] = 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0")
w("")
for nm in ["M2_U","M2_X1","M2_GX","M2_Y","M2_X2","M2_ZU2","M2_T","M2_T2","M2_Z2",
           "M2_POLYR","M2_XNUM","M2_XDEN","M2_YNUM","M2_YDEN","M2_PX","M2_PY","M2_NBA"]:
    w(f"u64 BLS_{nm}[12] = "+", ".join(["0"]*12))
for nm in ["M2_PP","M2_ISOP"]:
    w(f"u64 BLS_{nm}[24] = "+", ".join(["0"]*24))
for nm in ["M2_A","M2_B","M2_C","M2_D","M2_E","M2_F","M2_TMP"]:
    w(f"u64 BLS_{nm}[24] = "+", ".join(["0"]*24))
for nm in ["M2_CO","M2_XP","M2_N","M2_I","M2_SGNU","M2_RESP","M2_P","M2_S0","M2_S1"]:
    w(f"u64 BLS_{nm}[1] = 0")
w("")

def cp(src,dst,n): raw(f"copyb(0, {src}) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"copyb(0, {n}) -> r12"); raw("call bn254_memcpy")
def mul(d,a,b):
    if d==b: a,b=b,a          # mul_fp2 requires output != b; mul is commutative
    assert d!=b, f"mul {d}: output aliases b"
    raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {b}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_mul_fp2{S}")
def add(d,a,b):
    if d==b: a,b=b,a          # add_fp2 requires output != b; add is commutative
    assert d!=b, f"add {d}: output aliases b"
    raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {b}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_add_fp2{S}")
def sq(d,a): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {d}) -> r11"); raw(f"call zisklib_square_fp2{S}")
def inv(d,a): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {d}) -> r11"); raw(f"call zisklib_inv_fp2{S}")
def neg(d,a): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {d}) -> r11"); raw(f"call zisklib_neg_fp2{S}")

# ---------- sgn0_fp2(x r10) -> r10 ----------
w("; u64 zisklib_sgn0_fp2_bls12_381(const u64* x)  [r10=x] -> sign_0 | (zero_0 & sign_1)")
w("zisklib_sgn0_fp2_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_M2_CO) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")   # save x ptr
raw("copyb(r10, 8[a + 0]) -> r6"); raw("and(r6, 1) -> r6")           # sign_0
raw("copyb(0, BLS_M2_S0) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("copyb(0, BLS_M2_CO) -> r5"); raw("copyb(r5, 8[a + 0]) -> r5"); raw("add(r5, 48) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("and(r6, 1) -> r6")  # sign_1
raw("copyb(0, BLS_M2_S1) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("copyb(0, BLS_M2_CO) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_ZERO) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_eqn")   # r10 = zero_0
raw("copyb(0, BLS_M2_S1) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("and(r10, r6) -> r10")   # zero_0 & sign_1
raw("copyb(0, BLS_M2_S0) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r10, r6) -> r10")   # + sign_0 (mutually exclusive)
raw("pop r1"); raw("ret"); w("")

# ---------- eval_poly_fp2(coeffs r10, n r11, x r12) -> BLS_M2_POLYR ----------
w("; Horner Fp2 poly. r10=coeffs, r11=n, r12=x -> BLS_M2_POLYR")
w("zisklib_eval_poly_fp2_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_M2_CO) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_M2_N) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
raw("copyb(0, BLS_M2_XP) -> r5"); raw("copyb(r5, r12) -> 8[a + 0]")
raw("sub(r11, 1) -> r6")
raw("sll(r6, 6) -> r7"); raw("sll(r6, 5) -> r5"); raw("add(r7, r5) -> r7")   # (n-1)*96
raw("add(r10, r7) -> r10"); raw("copyb(0, BLS_M2_POLYR) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_M2_N) -> r5"); raw("copyb(r5, 8[a + 0]) -> r5"); raw("sub(r5, 2) -> r6")
raw("copyb(0, BLS_M2_I) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")
w("lm2_ep_loop:")
raw("copyb(0, BLS_M2_XP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r11")
raw("copyb(0, BLS_M2_POLYR) -> r10"); raw("copyb(0, BLS_M2_POLYR) -> r12"); raw(f"call zisklib_mul_fp2{S}")
raw("copyb(0, BLS_M2_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("copyb(0, BLS_M2_CO) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("sll(r13, 6) -> r7"); raw("sll(r13, 5) -> r5"); raw("add(r7, r5) -> r7"); raw("add(r10, r7) -> r11")
raw("copyb(0, BLS_M2_POLYR) -> r10"); raw("copyb(0, BLS_M2_POLYR) -> r12"); raw(f"call zisklib_add_fp2{S}")
raw("copyb(0, BLS_M2_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("eq(r13, 0), j(lm2_ep_done)")
raw("sub(r13, 1) -> r13"); raw("copyb(0, BLS_M2_I) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]"); raw("jump(lm2_ep_loop)")
w("lm2_ep_done:")
raw("pop r1"); raw("ret"); w("")

# ---------- compute_y2_iso_g2(x r10) -> BLS_M2_GX ----------
w("; y2 = x^3 + A'x + B' (Fp2). r10=x -> BLS_M2_GX")
w("zisklib_compute_y2_iso_g2_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_M2_XP) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_M2_XP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_M2_T) -> r11"); raw(f"call zisklib_square_fp2{S}")   # x^2
raw("copyb(0, BLS_M2_T) -> r10"); raw("copyb(0, BLS_M2_XP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_M2_T) -> r12"); raw(f"call zisklib_mul_fp2{S}")   # x^3
raw("copyb(0, BLS_ISO_A_G2) -> r10"); raw("copyb(0, BLS_M2_XP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_M2_T2) -> r12"); raw(f"call zisklib_mul_fp2{S}")  # A'x
add("BLS_M2_T","BLS_M2_T","BLS_M2_T2")
add("BLS_M2_GX","BLS_M2_T","BLS_ISO_B_G2")
raw("pop r1"); raw("ret"); w("")

# ---------- isogeny_map_g2(p r10) -> BLS_M2_ISOP[24] ----------
w("; 3-isogeny E'->E (Fp2). r10=p(24) -> BLS_M2_ISOP(24)")
w("zisklib_isogeny_map_g2_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_M2_XP) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")    # save p ptr in M2_XP
raw("copyb(0, BLS_M2_XP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_M2_PX) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_M2_XP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 96) -> r10"); raw("copyb(0, BLS_M2_PY) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
def evalpoly(tbl,n,dst):
    raw(f"copyb(0, {tbl}) -> r10"); raw(f"copyb(0, {n}) -> r11"); raw("copyb(0, BLS_M2_PX) -> r12"); raw(f"call zisklib_eval_poly_fp2{S}")
    cp("BLS_M2_POLYR",dst,12)
evalpoly("BLS_ISO_X_NUM_G2",NXN,"BLS_M2_XNUM")
evalpoly("BLS_ISO_X_DEN_G2",NXD,"BLS_M2_XDEN")
evalpoly("BLS_ISO_Y_NUM_G2",NYN,"BLS_M2_YNUM")
evalpoly("BLS_ISO_Y_DEN_G2",NYD,"BLS_M2_YDEN")
inv("BLS_M2_T","BLS_M2_XDEN"); mul("BLS_M2_T","BLS_M2_XNUM","BLS_M2_T")
raw("copyb(0, BLS_M2_T) -> r10"); raw("copyb(0, BLS_M2_ISOP) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
inv("BLS_M2_T","BLS_M2_YDEN"); mul("BLS_M2_T","BLS_M2_YNUM","BLS_M2_T"); mul("BLS_M2_T","BLS_M2_PY","BLS_M2_T")
raw("copyb(0, BLS_M2_T) -> r10"); raw("copyb(0, BLS_M2_ISOP) -> r11"); raw("add(r11, 96) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ---------- simple_swu_g2 (u in BLS_M2_U) -> BLS_M2_PP[24] ----------
w("; SSWU G2: u(12) in BLS_M2_U -> point on E' (BLS_M2_PP[24])")
w("zisklib_simple_swu_g2_bls12_381:")
raw("push r1")
sq("BLS_M2_T","BLS_M2_U")                    # u2
mul("BLS_M2_ZU2","BLS_SWU_Z_G2","BLS_M2_T")  # z_u2
sq("BLS_M2_T2","BLS_M2_T")                   # u4
sq("BLS_M2_Z2","BLS_SWU_Z_G2")               # z2
mul("BLS_M2_T2","BLS_M2_Z2","BLS_M2_T2")     # z2_u4
add("BLS_M2_T","BLS_M2_T2","BLS_M2_ZU2")     # denom
raw("copyb(0, BLS_M2_T) -> r10"); raw("copyb(0, BLS_ZERO) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_eqn")
raw("ltu(0, r10), j(lm2_swu_tv0)")
inv("BLS_M2_T","BLS_M2_T")                   # tv1
neg("BLS_M2_T2","BLS_ISO_B_G2")              # -B
inv("BLS_M2_Y","BLS_ISO_A_G2")               # 1/A
mul("BLS_M2_NBA","BLS_M2_T2","BLS_M2_Y")     # -B/A
add("BLS_M2_Y","BLS_F2_ONE12","BLS_M2_T")    # 1+tv1
mul("BLS_M2_X1","BLS_M2_NBA","BLS_M2_Y")
raw("jump(lm2_swu_havex1)")
w("lm2_swu_tv0:")
mul("BLS_M2_T","BLS_SWU_Z_G2","BLS_ISO_A_G2")
inv("BLS_M2_T","BLS_M2_T")
mul("BLS_M2_X1","BLS_ISO_B_G2","BLS_M2_T")
w("lm2_swu_havex1:")
raw("copyb(0, BLS_M2_X1) -> r10"); raw(f"call zisklib_compute_y2_iso_g2{S}")   # -> M2_GX
raw("copyb(0, BLS_M2_GX) -> r10"); raw("copyb(0, BLS_M2_Y) -> r11"); raw(f"call zisklib_sqrt_fp2{S}")
raw("eq(r10, 0), j(lm2_swu_nqr)")
cp("BLS_M2_X1","BLS_M2_PX",12)
raw("jump(lm2_swu_sgn)")
w("lm2_swu_nqr:")
mul("BLS_M2_X2","BLS_M2_ZU2","BLS_M2_X1")
raw("copyb(0, BLS_M2_X2) -> r10"); raw(f"call zisklib_compute_y2_iso_g2{S}")
raw("copyb(0, BLS_M2_GX) -> r10"); raw("copyb(0, BLS_M2_Y) -> r11"); raw(f"call zisklib_sqrt_fp2{S}")
cp("BLS_M2_X2","BLS_M2_PX",12)
w("lm2_swu_sgn:")
raw("copyb(0, BLS_M2_U) -> r10"); raw(f"call zisklib_sgn0_fp2{S}"); raw("copyb(0, BLS_M2_SGNU) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_M2_Y) -> r10"); raw(f"call zisklib_sgn0_fp2{S}")
raw("copyb(0, BLS_M2_SGNU) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("eq(r10, r6), j(lm2_swu_noflip)")
neg("BLS_M2_Y","BLS_M2_Y")
w("lm2_swu_noflip:")
raw("copyb(0, BLS_M2_PX) -> r10"); raw("copyb(0, BLS_M2_PP) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_M2_Y) -> r10"); raw("copyb(0, BLS_M2_PP) -> r11"); raw("add(r11, 96) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ---------- clear_cofactor_twist(p r10) -> BLS_M2_A (final) ----------
w("; clear_cofactor_twist(p r24) -> BLS_M2_A. psi=utf, psi2=utf∘utf.")
w("zisklib_clear_cofactor_twist_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_M2_P) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")    # save p ptr
def util_call(fn,src,dst): raw(f"copyb(0, {src}) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"call zisklib_{fn}{S}")
def util_p(fn,srcslot,dst): raw(f"copyb(0, {srcslot}) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"call zisklib_{fn}{S}")
def comb(fn,a,b,d): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {b}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_{fn}{S}")
def comb_p(fn,a,pslot,d): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {pslot}) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_{fn}{S}")
# A = scalar_mul_by_abs_x(p) ; B = neg(A)   (t1 = B)
util_p("scalar_mul_by_abs_x_complete_twist","BLS_M2_P","BLS_M2_A")
util_call("neg_twist","BLS_M2_A","BLS_M2_B")
# C = utf(p)   (t2 = C)
util_p("utf_endomorphism_twist","BLS_M2_P","BLS_M2_C")
# D = dbl_complete(p) ; E=utf(D); D=utf(E)   (t3 = D = psi2(2p))
util_p("dbl_complete_twist","BLS_M2_P","BLS_M2_D")
util_call("utf_endomorphism_twist","BLS_M2_D","BLS_M2_E")
util_call("utf_endomorphism_twist","BLS_M2_E","BLS_M2_D")
# E = sub_complete(D, C)   (t3 = E)
comb("sub_complete_twist","BLS_M2_D","BLS_M2_C","BLS_M2_E")
# F = add_complete(B, C)   (t2 = F)
comb("add_complete_twist","BLS_M2_B","BLS_M2_C","BLS_M2_F")
# A = scalar_mul_by_abs_x(F) ; C = neg(A)   (t2 = C)
util_call("scalar_mul_by_abs_x_complete_twist","BLS_M2_F","BLS_M2_A")
util_call("neg_twist","BLS_M2_A","BLS_M2_C")
# D = add_complete(E, C)   (t3 = D)
comb("add_complete_twist","BLS_M2_E","BLS_M2_C","BLS_M2_D")
# A = sub_complete(D, B)   (t3 = A)
comb("sub_complete_twist","BLS_M2_D","BLS_M2_B","BLS_M2_A")
# A = sub_complete(A, p)  -> use TMP then copy
comb_p("sub_complete_twist","BLS_M2_A","BLS_M2_P","BLS_M2_TMP")
cp("BLS_M2_TMP","BLS_M2_A",24)
raw("pop r1"); raw("ret"); w("")

# ---------- map_to_curve_g2(u r10, result r11) -> r10 status ----------
w("; u64 zisklib_map_to_curve_g2_bls12_381(const u64* u, u64* result)  [r10=u(12), r11=result(24)] -> r10 status")
w("zisklib_map_to_curve_g2_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_M2_RESP) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
raw("copyb(0, BLS_M2_CO) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")   # save u ptr
# field check: u0<P and u1<P
raw(f"copyb(0, BLS_P) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_ltn")
raw("eq(r10, 0), j(lm2_mtc_err)")
raw("copyb(0, BLS_M2_CO) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 48) -> r10"); raw("copyb(0, BLS_P) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_ltn")
raw("eq(r10, 0), j(lm2_mtc_err)")
# u -> BLS_M2_U (12)
raw("copyb(0, BLS_M2_CO) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_M2_U) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
# swu -> PP ; isogeny -> ISOP ; clear_cofactor -> A ; result = A
raw(f"call zisklib_simple_swu_g2{S}")
raw("copyb(0, BLS_M2_PP) -> r10"); raw(f"call zisklib_isogeny_map_g2{S}")
raw("copyb(0, BLS_M2_ISOP) -> r10"); raw(f"call zisklib_clear_cofactor_twist{S}")
raw("copyb(0, BLS_M2_A) -> r10"); raw("copyb(0, BLS_M2_RESP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret")
w("lm2_mtc_err:")
raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret"); w("")

open(_OUT+"/bls12_381/map_g2.zisk","w").write("\n".join(out)+"\n")
print("wrote map_g2.zisk", len(out), "lines; NXN",NXN,"NXD",NXD,"NYN",NYN,"NYD",NYD)
