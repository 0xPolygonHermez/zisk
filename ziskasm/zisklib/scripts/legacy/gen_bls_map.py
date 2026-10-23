#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/map.zisk (G1 map_to_curve): SSWU + 11-isogeny
# + cofactor mul by (1-x). Mirrors map_to_curve.rs G1 path. Prefix lmp_.
import re
RS=_REPO+"/ziskos/entrypoint/src/zisklib/lib/bls12_381/constants.rs"
rs=open(RS).read()
def hx(s): return int(s.replace("_","").replace("0x",""),16)
def const6(name):
    m=re.search(r"pub const %s:\s*\[u64;\s*6\]\s*=\s*\[([^\]]+)\];"%name, rs)
    vals=[v.strip() for v in m.group(1).split(",") if v.strip()!=""]
    return [hx(v) if v.lower().startswith("0x") else int(v) for v in vals]
def table(name):
    m=re.search(r"pub const %s:\s*\[\[u64;\s*6\];\s*\d+\]\s*=\s*\[(.*?)\];"%name, rs, re.S)
    nums=re.findall(r"0x[0-9A-Fa-f_]+|\b\d+\b", m.group(1))
    v=[hx(x) if x.lower().startswith("0x") else int(x) for x in nums]
    assert len(v)%6==0, (name,len(v))
    return v  # flattened

out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)
S="_bls12_381"
def emitc(name,vals): w(f"const u64 {name}[{len(vals)}] = "+", ".join("0x%016x"%x for x in vals))

w("; ============================================================================")
w("; bls12_381/map.zisk - map_to_curve for G1 (EIP-2537 MAP_FP_TO_G1). GENERATED")
w("; (scripts/legacy/gen_bls_map.py). SSWU -> 11-isogeny -> cofactor·(1-x). lmp_.")
w("; ============================================================================")
w("")
emitc("BLS_ISO_A_G1",const6("ISO_A_G1"))
emitc("BLS_ISO_B_G1",const6("ISO_B_G1"))
emitc("BLS_SWU_Z_G1",const6("SWU_Z_G1"))
emitc("BLS_SWU_Z2_G1",const6("SWU_Z2_G1"))
emitc("BLS_ISO_X_NUM_G1",table("ISO_X_NUM_G1"))
emitc("BLS_ISO_X_DEN_G1",table("ISO_X_DEN_G1"))
emitc("BLS_ISO_Y_NUM_G1",table("ISO_Y_NUM_G1"))
emitc("BLS_ISO_Y_DEN_G1",table("ISO_Y_DEN_G1"))
NX_NUM=len(table("ISO_X_NUM_G1"))//6; NX_DEN=len(table("ISO_X_DEN_G1"))//6
NY_NUM=len(table("ISO_Y_NUM_G1"))//6; NY_DEN=len(table("ISO_Y_DEN_G1"))//6
# COFACTOR_G1 = 0xD201000000010001, bit-array MSB-first (64 bits)
COF=0xD201000000010001
cof_be=[(COF>>(63-i))&1 for i in range(64)]
w("const u64 BLS_COFACTOR_G1_BE[64] = "+", ".join(str(b) for b in cof_be))
w("const u64 BLS_FP_ONE_6[6] = 1, 0, 0, 0, 0, 0")
w("")
# scratch
for nm in ["MP_U","MP_R","MP_X1","MP_GX","MP_Y","MP_X2","MP_ZU2","MP_T","MP_T2",
           "MP_POLYR","MP_XNUM","MP_XDEN","MP_YNUM","MP_YDEN","MP_PX","MP_PY"]:
    w(f"u64 BLS_{nm}[6] = 0, 0, 0, 0, 0, 0")
for nm in ["MP_PP","MP_ISOP","MP_CR","MP_CR2"]:
    w(f"u64 BLS_{nm}[12] = "+", ".join(["0"]*12))
for nm in ["MP_CO","MP_XP","MP_RP","MP_N","MP_I","MP_SGNU","MP_RESP"]:
    w(f"u64 BLS_{nm}[1] = 0")
w("")

def cp6(src,dst): raw(f"copyb(0, {src}) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
def mul(d,a,b): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {b}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_mul_fp{S}")
def add(d,a,b): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {b}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_add_fp{S}")
def sq(d,a): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {d}) -> r11"); raw(f"call zisklib_square_fp{S}")
def inv(d,a): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {d}) -> r11"); raw(f"call zisklib_inv_fp{S}")
def neg(d,a): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {d}) -> r11"); raw(f"call zisklib_neg_fp{S}")

# ---------- eval_poly_fp(coeffs r10, n r11, x r12) -> BLS_MP_POLYR ----------
w("; Horner eval of a degree-(n-1) Fp poly. r10=coeffs, r11=n, r12=x -> BLS_MP_POLYR")
w("zisklib_eval_poly_fp_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_MP_CO) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_MP_N) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
raw("copyb(0, BLS_MP_XP) -> r5"); raw("copyb(r5, r12) -> 8[a + 0]")
# result = coeffs[n-1]
raw("sub(r11, 1) -> r6")           # n-1
raw("sll(r6, 5) -> r7"); raw("sll(r6, 4) -> r5"); raw("add(r7, r5) -> r7")   # (n-1)*48
raw("add(r10, r7) -> r10"); raw("copyb(0, BLS_MP_POLYR) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
# idx = n-2, stored in MP_I (authoritative; r13 is scratch reloaded each use)
raw("copyb(0, BLS_MP_N) -> r5"); raw("copyb(r5, 8[a + 0]) -> r5"); raw("sub(r5, 2) -> r6")
raw("copyb(0, BLS_MP_I) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")
w("lmp_ep_loop:")
# result = mul(result, x)
raw("copyb(0, BLS_MP_XP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r11")
raw("copyb(0, BLS_MP_POLYR) -> r10"); raw("copyb(0, BLS_MP_POLYR) -> r12"); raw(f"call zisklib_mul_fp{S}")
# result = add(result, coeffs[idx])
raw("copyb(0, BLS_MP_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("copyb(0, BLS_MP_CO) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("sll(r13, 5) -> r7"); raw("sll(r13, 4) -> r5"); raw("add(r7, r5) -> r7"); raw("add(r10, r7) -> r11")
raw("copyb(0, BLS_MP_POLYR) -> r10"); raw("copyb(0, BLS_MP_POLYR) -> r12"); raw(f"call zisklib_add_fp{S}")
raw("copyb(0, BLS_MP_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("eq(r13, 0), j(lmp_ep_done)")
raw("sub(r13, 1) -> r13"); raw("copyb(0, BLS_MP_I) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]"); raw("jump(lmp_ep_loop)")
w("lmp_ep_done:")
raw("pop r1"); raw("ret"); w("")

# ---------- compute_y2_iso_g1(x r10) -> BLS_MP_GX ----------
w("; y2 = x^3 + A'x + B'. r10=x -> BLS_MP_GX")
w("zisklib_compute_y2_iso_g1_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_MP_XP) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_MP_XP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_MP_T) -> r11"); raw(f"call zisklib_square_fp{S}")   # x^2
raw("copyb(0, BLS_MP_T) -> r10"); raw("copyb(0, BLS_MP_XP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_MP_T) -> r12"); raw(f"call zisklib_mul_fp{S}")  # x^3
raw("copyb(0, BLS_ISO_A_G1) -> r10"); raw("copyb(0, BLS_MP_XP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_MP_T2) -> r12"); raw(f"call zisklib_mul_fp{S}")  # A'x
add("BLS_MP_T","BLS_MP_T","BLS_MP_T2")
add("BLS_MP_GX","BLS_MP_T","BLS_ISO_B_G1")
raw("pop r1"); raw("ret"); w("")

# ---------- isogeny_map_g1(p r10) -> BLS_MP_ISOP[12] ----------
w("; 11-isogeny E'->E. r10=p(12) -> BLS_MP_ISOP(12)")
w("zisklib_isogeny_map_g1_bls12_381:")
raw("push r1")
# save x=p[0..6] -> MP_PX, y=p[6..12] -> MP_PY
raw("copyb(0, BLS_MP_RP) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_MP_RP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_MP_PX) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_MP_RP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 48) -> r10"); raw("copyb(0, BLS_MP_PY) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
def evalpoly(tbl,n,dst):
    raw(f"copyb(0, {tbl}) -> r10"); raw(f"copyb(0, {n}) -> r11"); raw("copyb(0, BLS_MP_PX) -> r12"); raw(f"call zisklib_eval_poly_fp{S}")
    cp6("BLS_MP_POLYR",dst)
evalpoly("BLS_ISO_X_NUM_G1",NX_NUM,"BLS_MP_XNUM")
evalpoly("BLS_ISO_X_DEN_G1",NX_DEN,"BLS_MP_XDEN")
evalpoly("BLS_ISO_Y_NUM_G1",NY_NUM,"BLS_MP_YNUM")
evalpoly("BLS_ISO_Y_DEN_G1",NY_DEN,"BLS_MP_YDEN")
# x_out = xnum/xden ; store into ISOP[0..6]
inv("BLS_MP_T","BLS_MP_XDEN"); mul("BLS_MP_T","BLS_MP_XNUM","BLS_MP_T")
raw("copyb(0, BLS_MP_T) -> r10"); raw("copyb(0, BLS_MP_ISOP) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
# y_out = y * (ynum/yden) ; store into ISOP[6..12]
inv("BLS_MP_T","BLS_MP_YDEN"); mul("BLS_MP_T","BLS_MP_YNUM","BLS_MP_T"); mul("BLS_MP_T","BLS_MP_PY","BLS_MP_T")
raw("copyb(0, BLS_MP_T) -> r10"); raw("copyb(0, BLS_MP_ISOP) -> r11"); raw("add(r11, 48) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ---------- cofactor_mul_g1(p r10) -> BLS_MP_CR[12]  (double-and-add by 1-x) ----------
w("; scalar-mul by COFACTOR_G1=(1-x)=0xD201000000010001, complete add/dbl. r10=p -> BLS_MP_CR")
w("zisklib_cofactor_mul_g1_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_MP_CO) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")   # reuse MP_CO to hold p ptr
# r = p (MSB is bit 0 of BE array = 1)
raw("copyb(0, BLS_MP_CO) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_MP_CR) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, 1) -> r13")
w("lmp_cof_loop:")
raw("eq(r13, 64), j(lmp_cof_done)")
raw("copyb(0, BLS_MP_CR) -> r10"); raw("copyb(0, BLS_MP_CR2) -> r11"); raw(f"call zisklib_dbl_complete{S}")
cp12b=lambda a,b:(raw(f"copyb(0, {a}) -> r10"),raw(f"copyb(0, {b}) -> r11"),raw("copyb(0, 12) -> r12"),raw("call bn254_memcpy"))
cp12b("BLS_MP_CR2","BLS_MP_CR")
raw("copyb(0, BLS_COFACTOR_G1_BE) -> r5"); raw("sll(r13, 3) -> r6"); raw("add(r5, r6) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("eq(r6, 0), j(lmp_cof_next)")
raw("copyb(0, BLS_MP_CR) -> r10"); raw("copyb(0, BLS_MP_CO) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_MP_CR2) -> r12"); raw(f"call zisklib_add_complete{S}")
cp12b("BLS_MP_CR2","BLS_MP_CR")
w("lmp_cof_next:")
raw("add(r13, 1) -> r13"); raw("jump(lmp_cof_loop)")
w("lmp_cof_done:")
raw("pop r1"); raw("ret"); w("")

# ---------- simple_swu_g1(u r10) -> BLS_MP_PP[12] ----------
w("; SSWU: u(6) -> point on E' (BLS_MP_PP[12]). r10=u")
w("zisklib_simple_swu_g1_bls12_381:")
raw("push r1")
# u is expected in BLS_MP_U (caller copies it there before calling)
# tv1 = inv0(Z^2 u^4 + Z u^2)
sq("BLS_MP_T","BLS_MP_U")                 # u2
mul("BLS_MP_ZU2","BLS_SWU_Z_G1","BLS_MP_T")   # z_u2 = Z*u2
sq("BLS_MP_T2","BLS_MP_T")                # u4
mul("BLS_MP_T2","BLS_SWU_Z2_G1","BLS_MP_T2")  # z2_u4
add("BLS_MP_T","BLS_MP_T2","BLS_MP_ZU2")  # denom
# tv1 = inv(denom) if denom!=0 else 0 (inv0). Our inv halts on 0; guard: if denom==0 -> tv1=0 path (x1 special)
raw("copyb(0, BLS_MP_T) -> r10"); raw("copyb(0, BLS_ZERO) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_eqn")
raw("ltu(0, r10), j(lmp_swu_tv0)")
inv("BLS_MP_T","BLS_MP_T")                # tv1
# x1 = (-B/A)(1+tv1)
neg("BLS_MP_T2","BLS_ISO_B_G1")           # -B
inv("BLS_MP_Y","BLS_ISO_A_G1")            # 1/A (reuse MP_Y)
mul("BLS_MP_T2","BLS_MP_T2","BLS_MP_Y")   # -B/A
add("BLS_MP_Y","BLS_FP_ONE_6","BLS_MP_T") # 1+tv1
mul("BLS_MP_X1","BLS_MP_T2","BLS_MP_Y")
raw("jump(lmp_swu_havex1)")
w("lmp_swu_tv0:")
# x1 = B/(Z*A)
mul("BLS_MP_T","BLS_SWU_Z_G1","BLS_ISO_A_G1")
inv("BLS_MP_T","BLS_MP_T")
mul("BLS_MP_X1","BLS_ISO_B_G1","BLS_MP_T")
w("lmp_swu_havex1:")
# gx1 = compute_y2(x1)
raw("copyb(0, BLS_MP_X1) -> r10"); raw(f"call zisklib_compute_y2_iso_g1{S}")   # -> MP_GX
# (y1, is_qr) = sqrt(gx1)
raw("copyb(0, BLS_MP_GX) -> r10"); raw("copyb(0, BLS_MP_Y) -> r11"); raw(f"call zisklib_sqrt_fp{S}")
raw("eq(r10, 0), j(lmp_swu_nqr)")
# qr: x=x1 (already), y=y1 (in MP_Y). set PX=x1
cp6("BLS_MP_X1","BLS_MP_PX")
raw("jump(lmp_swu_sgn)")
w("lmp_swu_nqr:")
# x2 = z_u2 * x1 ; gx2 = compute_y2(x2) ; (y2,_)=sqrt(gx2)
mul("BLS_MP_X2","BLS_MP_ZU2","BLS_MP_X1")
raw("copyb(0, BLS_MP_X2) -> r10"); raw(f"call zisklib_compute_y2_iso_g1{S}")
raw("copyb(0, BLS_MP_GX) -> r10"); raw("copyb(0, BLS_MP_Y) -> r11"); raw(f"call zisklib_sqrt_fp{S}")
cp6("BLS_MP_X2","BLS_MP_PX")
w("lmp_swu_sgn:")
# if sgn0(u) != sgn0(y): y=-y
raw("copyb(0, BLS_MP_U) -> r10"); raw(f"call zisklib_sgn0_fp{S}"); raw("copyb(0, BLS_MP_SGNU) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_MP_Y) -> r10"); raw(f"call zisklib_sgn0_fp{S}")   # r10 = sgn0(y)
raw("copyb(0, BLS_MP_SGNU) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("eq(r10, r6), j(lmp_swu_noflip)")
neg("BLS_MP_Y","BLS_MP_Y")
w("lmp_swu_noflip:")
# PP = (PX, Y)
raw("copyb(0, BLS_MP_PX) -> r10"); raw("copyb(0, BLS_MP_PP) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_MP_Y) -> r10"); raw("copyb(0, BLS_MP_PP) -> r11"); raw("add(r11, 48) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

# ---------- map_to_curve_g1(u r10, result r11) -> r10 status ----------
w("; u64 zisklib_map_to_curve_g1_bls12_381(const u64* u, u64* result)  [r10=u(6), r11=result(12)] -> r10 status")
w("zisklib_map_to_curve_g1_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_MP_RESP) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")   # result ptr (dedicated; isogeny clobbers MP_RP)
# save u ptr in RAM (bn254_ltn clobbers r14)
raw("copyb(0, BLS_MP_CO) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
# field check u<P
raw(f"copyb(0, BLS_P) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_ltn")
raw("eq(r10, 0), j(lmp_mtc_err)")
# copy u into BLS_MP_U
raw("copyb(0, BLS_MP_CO) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_MP_U) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
# p' = swu(u)
raw("copyb(0, BLS_MP_U) -> r10"); raw(f"call zisklib_simple_swu_g1{S}")   # -> MP_PP
# p = isogeny(p')
raw("copyb(0, BLS_MP_PP) -> r10"); raw(f"call zisklib_isogeny_map_g1{S}") # -> MP_ISOP
# result = cofactor_mul(p)
raw("copyb(0, BLS_MP_ISOP) -> r10"); raw(f"call zisklib_cofactor_mul_g1{S}")  # -> MP_CR
raw("copyb(0, BLS_MP_CR) -> r10"); raw("copyb(0, BLS_MP_RESP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, 0) -> r10")
raw("pop r1"); raw("ret")
w("lmp_mtc_err:")
raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret"); w("")

open(_OUT+"/bls12_381/map.zisk","w").write("\n".join(out)+"\n")
print("wrote map.zisk", len(out), "lines; NX_NUM",NX_NUM,"NX_DEN",NX_DEN,"NY_NUM",NY_NUM,"NY_DEN",NY_DEN)
