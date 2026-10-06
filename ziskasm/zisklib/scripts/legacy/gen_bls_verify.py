#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/verify.zisk : point decompression (G1/G2),
# single pairing (miller+final_exp), bls_verify. Mirrors bls.rs. Prefix lv_.
out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)
S="_bls12_381"

DST=b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_"
padded=list(DST)+[0]*((8-len(DST)%8)%8)
dst_words=[int.from_bytes(bytes(padded[i:i+8]),'little') for i in range(0,len(padded),8)]

w("; ============================================================================")
w("; bls12_381/verify.zisk - BLS signature verify + point decompression. GENERATED")
w("; (scripts/legacy/gen_bls_verify.py). Mirrors bls.rs. Prefix lv_.")
w("; ============================================================================")
w("")
w(f"const u64 BLS_DST_STR[{len(dst_words)}] = "+", ".join("0x%016x"%x for x in dst_words))
w(f"define BLS_DST_LEN {len(DST)}")
w("u64 BLS_V_TMP96[12] = "+", ".join(["0"]*12))
for nm in ["V_X","V_Y","V_YN","V_XR","V_XI","V_YR","V_YI","V_YNR","V_YNI"]:
    w(f"u64 BLS_{nm}[6] = 0, 0, 0, 0, 0, 0")
w("u64 BLS_V_XF[12] = "+", ".join(["0"]*12))
w("u64 BLS_V_YF[12] = "+", ".join(["0"]*12))
w("u64 BLS_V_PK[12] = "+", ".join(["0"]*12))
for nm in ["V_SIG","V_Q","V_YSQ2","V_MILL2"]:
    pass
w("u64 BLS_V_SIG[24] = "+", ".join(["0"]*24))
w("u64 BLS_V_Q[24] = "+", ".join(["0"]*24))
w("u64 BLS_V_MILL[72] = "+", ".join(["0"]*72))
w("u64 BLS_V_C1[72] = "+", ".join(["0"]*72))
w("u64 BLS_V_C2[72] = "+", ".join(["0"]*72))
for nm in ["V_IN","V_FBIN","V_PKP","V_MSG","V_MSGLEN","V_SIGP","V_YSIGN","V_G1INF","V_G2INF","V_I","V_DEST"]:
    w(f"u64 BLS_{nm}[1] = 0")
w("")

def sq(d,a): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {d}) -> r11"); raw(f"call zisklib_square_fp{S}")
def mul(d,a,b): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {b}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_mul_fp{S}")
def add(d,a,b): raw(f"copyb(0, {a}) -> r10"); raw(f"copyb(0, {b}) -> r11"); raw(f"copyb(0, {d}) -> r12"); raw(f"call zisklib_add_fp{S}")
def cp(src,dst,n): raw(f"copyb(0, {src}) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"copyb(0, {n}) -> r12"); raw("call bn254_memcpy")

# ---------- fill_be6(bytes r10, dest r11): dest[5-i]=be64(bytes+i*8) ----------
w("; void zisklib_fill_be6(const u8* bytes48, u64* dest6)")
w("zisklib_fill_be6_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_V_FBIN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_V_DEST) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
raw("copyb(0, BLS_V_I) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("lv_fb6_loop:")
raw("copyb(0, BLS_V_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("eq(r13, 6), j(lv_fb6_done)")
raw("copyb(0, BLS_V_FBIN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("sll(r13, 3) -> r6"); raw("add(r10, r6) -> r10")
raw("call zisklib_be64")
raw("copyb(0, BLS_V_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("copyb(0, 5) -> r6"); raw("sub(r6, r13) -> r6"); raw("sll(r6, 3) -> r6")   # (5-i)*8
raw("copyb(0, BLS_V_DEST) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("add(r7, r6) -> r7"); raw("copyb(r7, r10) -> 8[a + 0]")
raw("add(r13, 1) -> r13"); raw("copyb(0, BLS_V_I) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]")
raw("jump(lv_fb6_loop)")
w("lv_fb6_done:")
raw("pop r1"); raw("ret"); w("")

# ---------- decompress_g1(input r10) -> BLS_V_PK, BLS_V_G1INF ; r10 status(0 ok,1 err) ----------
w("; u64 zisklib_decompress_g1(const u8* in48) [r10] -> r10 status; result BLS_V_PK + BLS_V_G1INF")
w("zisklib_decompress_g1_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_V_IN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(r10, 1[a + 0]) -> r6")            # flags
raw("copyb(0, BLS_V_YSIGN) -> r5")
raw("srl(r6, 5) -> r7"); raw("and(r7, 1) -> r7"); raw("copyb(r5, r7) -> 8[a + 0]")   # y_sign
raw("and(r6, 0x80) -> r7"); raw("eq(r7, 0), j(lv_dg1_err)")
raw("and(r6, 0x40) -> r7"); raw("eq(r7, 0), j(lv_dg1_notinf)")
# infinity
raw("copyb(0, BLS_V_PK) -> r10"); raw("copyb(0, BLS_G1_IDENTITY) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_V_G1INF) -> r5"); raw("copyb(r5, 1) -> 8[a + 0]"); raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret")
w("lv_dg1_notinf:")
raw("copyb(0, BLS_V_G1INF) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
# copy 48 bytes to V_TMP96, clear byte0 flags
raw("copyb(0, BLS_V_IN) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_V_TMP96) -> r11"); raw("copyb(0, 48) -> r12"); raw("call zisklib_bytecopy")
raw("copyb(0, BLS_V_TMP96) -> r7"); raw("copyb(r7, 1[a + 0]) -> r6"); raw("and(r6, 0x1f) -> r6"); raw("copyb(r7, r6) -> 1[a + 0]")
# parse x
raw("copyb(0, BLS_V_TMP96) -> r10"); raw("copyb(0, BLS_V_X) -> r11"); raw("call zisklib_fill_be6_bls12_381")
# x<P?
raw("copyb(0, BLS_V_X) -> r10"); raw("copyb(0, BLS_P) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_ltn"); raw("eq(r10, 0), j(lv_dg1_err)")
# y_sq = x^3 + E_B
sq("BLS_V_Y","BLS_V_X"); mul("BLS_V_Y","BLS_V_Y","BLS_V_X"); add("BLS_V_Y","BLS_V_Y","BLS_E_B")
# y = sqrt(y_sq); if !qr err   (reuse BLS_V_Y as y_sq input, output to BLS_V_YR temp then move)
raw("copyb(0, BLS_V_Y) -> r10"); raw("copyb(0, BLS_V_YR) -> r11"); raw(f"call zisklib_sqrt_fp{S}"); raw("eq(r10, 0), j(lv_dg1_err)")
cp("BLS_V_YR","BLS_V_Y",6)
# y_neg = -y ; y_is_larger = ltn(y_neg, y)
raw("copyb(0, BLS_V_Y) -> r10"); raw("copyb(0, BLS_V_YN) -> r11"); raw(f"call zisklib_neg_fp{S}")
raw("copyb(0, BLS_V_YN) -> r10"); raw("copyb(0, BLS_V_Y) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_ltn")   # r10 = y_is_larger
raw("copyb(0, BLS_V_YSIGN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("eq(r10, r6), j(lv_dg1_usey)")
cp("BLS_V_YN","BLS_V_YF",6); raw("jump(lv_dg1_setpk)")
w("lv_dg1_usey:")
cp("BLS_V_Y","BLS_V_YF",6)
w("lv_dg1_setpk:")
cp("BLS_V_X","BLS_V_PK",6)
raw("copyb(0, BLS_V_YF) -> r10"); raw("copyb(0, BLS_V_PK) -> r11"); raw("add(r11, 48) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret")
w("lv_dg1_err:")
raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret"); w("")

# ---------- decompress_g2(input r10) -> BLS_V_SIG, BLS_V_G2INF ; r10 status ----------
w("; u64 zisklib_decompress_g2(const u8* in96) [r10] -> r10 status; result BLS_V_SIG + BLS_V_G2INF")
w("zisklib_decompress_g2_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_V_IN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(r10, 1[a + 0]) -> r6")
raw("copyb(0, BLS_V_YSIGN) -> r5"); raw("srl(r6, 5) -> r7"); raw("and(r7, 1) -> r7"); raw("copyb(r5, r7) -> 8[a + 0]")
raw("and(r6, 0x80) -> r7"); raw("eq(r7, 0), j(lv_dg2_err)")
raw("and(r6, 0x40) -> r7"); raw("eq(r7, 0), j(lv_dg2_notinf)")
raw("copyb(0, BLS_V_SIG) -> r10"); raw("copyb(0, BLS_G2_IDENTITY) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_V_G2INF) -> r5"); raw("copyb(r5, 1) -> 8[a + 0]"); raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret")
w("lv_dg2_notinf:")
raw("copyb(0, BLS_V_G2INF) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
# copy first 48 bytes (x_i) to V_TMP96, clear flags; parse x_i
raw("copyb(0, BLS_V_IN) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_V_TMP96) -> r11"); raw("copyb(0, 48) -> r12"); raw("call zisklib_bytecopy")
raw("copyb(0, BLS_V_TMP96) -> r7"); raw("copyb(r7, 1[a + 0]) -> r6"); raw("and(r6, 0x1f) -> r6"); raw("copyb(r7, r6) -> 1[a + 0]")
raw("copyb(0, BLS_V_TMP96) -> r10"); raw("copyb(0, BLS_V_XI) -> r11"); raw("call zisklib_fill_be6_bls12_381")
# parse x_r from input[48..96]
raw("copyb(0, BLS_V_IN) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 48) -> r10"); raw("copyb(0, BLS_V_XR) -> r11"); raw("call zisklib_fill_be6_bls12_381")
# checks x_r<P, x_i<P
raw("copyb(0, BLS_V_XR) -> r10"); raw("copyb(0, BLS_P) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_ltn"); raw("eq(r10, 0), j(lv_dg2_err)")
raw("copyb(0, BLS_V_XI) -> r10"); raw("copyb(0, BLS_P) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_ltn"); raw("eq(r10, 0), j(lv_dg2_err)")
# x = (x_r, x_i) -> V_XF[12]
cp("BLS_V_XR","BLS_V_XF",6)
raw("copyb(0, BLS_V_XI) -> r10"); raw("copyb(0, BLS_V_XF) -> r11"); raw("add(r11, 48) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
# y_sq = x^3 + ETWISTED_B  (Fp2)
raw("copyb(0, BLS_V_XF) -> r10"); raw("copyb(0, BLS_V_YF) -> r11"); raw(f"call zisklib_square_fp2{S}")
raw("copyb(0, BLS_V_YF) -> r10"); raw("copyb(0, BLS_V_XF) -> r11"); raw("copyb(0, BLS_V_YF) -> r12"); raw(f"call zisklib_mul_fp2{S}")
raw("copyb(0, BLS_V_YF) -> r10"); raw("copyb(0, BLS_ETWISTED_B) -> r11"); raw("copyb(0, BLS_V_YF) -> r12"); raw(f"call zisklib_add_fp2{S}")
# y = sqrt_fp2 ; if !qr err  (y_sq in V_YF, out to V_SIG+12 area? use a temp: reuse V_MILL first 12)
raw("copyb(0, BLS_V_YF) -> r10"); raw("copyb(0, BLS_V_MILL) -> r11"); raw(f"call zisklib_sqrt_fp2{S}"); raw("eq(r10, 0), j(lv_dg2_err)")
cp("BLS_V_MILL","BLS_V_YF",12)     # y in V_YF
# y_neg = -y -> V_MILL (reuse)
raw("copyb(0, BLS_V_YF) -> r10"); raw("copyb(0, BLS_V_MILL) -> r11"); raw(f"call zisklib_neg_fp2{S}")   # y_neg in V_MILL[0..12]
# split y=(yr,yi) at V_YF, y_neg=(ynr,yni) at V_MILL
cp("BLS_V_YF","BLS_V_YR",6)
raw("copyb(0, BLS_V_YF) -> r10"); raw("add(r10, 48) -> r10"); raw("copyb(0, BLS_V_YI) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
cp("BLS_V_MILL","BLS_V_YNR",6)
raw("copyb(0, BLS_V_MILL) -> r10"); raw("add(r10, 48) -> r10"); raw("copyb(0, BLS_V_YNI) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
# y_is_larger: if yi != yni: ltn(yni, yi) else ltn(ynr, yr)
raw("copyb(0, BLS_V_YI) -> r10"); raw("copyb(0, BLS_V_YNI) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_eqn")   # r10=1 if equal
raw("ltu(0, r10), j(lv_dg2_cmpr)")
raw("copyb(0, BLS_V_YNI) -> r10"); raw("copyb(0, BLS_V_YI) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_ltn"); raw("jump(lv_dg2_havelarge)")
w("lv_dg2_cmpr:")
raw("copyb(0, BLS_V_YNR) -> r10"); raw("copyb(0, BLS_V_YR) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_ltn")
w("lv_dg2_havelarge:")
raw("copyb(0, BLS_V_YSIGN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("eq(r10, r6), j(lv_dg2_usey)")
# final = y_neg (V_MILL); else y (V_YF)
raw("copyb(0, BLS_V_XF) -> r10"); raw("copyb(0, BLS_V_SIG) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_V_MILL) -> r10"); raw("copyb(0, BLS_V_SIG) -> r11"); raw("add(r11, 96) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret")
w("lv_dg2_usey:")
raw("copyb(0, BLS_V_XF) -> r10"); raw("copyb(0, BLS_V_SIG) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_V_YF) -> r10"); raw("copyb(0, BLS_V_SIG) -> r11"); raw("add(r11, 96) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret")
w("lv_dg2_err:")
raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret"); w("")

# ---------- pairing(P r10, Q r11, dest r12) -> dest[72] ----------
w("; void zisklib_pairing_bls12_381(const u64* p12, const u64* q24, u64* dest72)")
w("zisklib_pairing_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_V_DEST) -> r5"); raw("copyb(r5, r12) -> 8[a + 0]")
raw("copyb(0, BLS_V_MILL) -> r12"); raw(f"call zisklib_miller_loop{S}")   # P,Q in r10,r11 already
raw("copyb(0, BLS_V_MILL) -> r10"); raw("copyb(0, BLS_V_DEST) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw(f"call zisklib_final_exp{S}")
raw("pop r1"); raw("ret"); w("")

# ---------- bls_verify(pk r10, msg r11, msglen r12, sig r13) -> r10 (1 valid / 0 invalid) ----------
w("; u64 zisklib_bls_verify(const u8* pk48, const u8* msg, u64 msglen, const u8* sig96) -> r10")
w("zisklib_bls_verify_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_V_PKP) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_V_MSG) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
raw("copyb(0, BLS_V_MSGLEN) -> r5"); raw("copyb(r5, r12) -> 8[a + 0]")
raw("copyb(0, BLS_V_SIGP) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]")
# decompress sig (G2)
raw("copyb(0, BLS_V_SIGP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_decompress_g2_bls12_381"); raw("ltu(0, r10), j(lv_bv_bad)")
# subgroup (if not inf)
raw("copyb(0, BLS_V_G2INF) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("ltu(0, r6), j(lv_bv_sigok)")
raw("copyb(0, BLS_V_SIG) -> r10"); raw("call zisklib_is_on_subgroup_twist_bls12_381"); raw("eq(r10, 0), j(lv_bv_bad)")
w("lv_bv_sigok:")
# decompress pk (G1)
raw("copyb(0, BLS_V_PKP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("call zisklib_decompress_g1_bls12_381"); raw("ltu(0, r10), j(lv_bv_bad)")
raw("copyb(0, BLS_V_G1INF) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("ltu(0, r6), j(lv_bv_bad)")   # pk infinity -> invalid
raw("copyb(0, BLS_V_PK) -> r10"); raw("call zisklib_is_on_subgroup_bls12_381"); raw("eq(r10, 0), j(lv_bv_bad)")
# q = hash_to_curve_g2(msg, DST)
raw("copyb(0, BLS_V_MSG) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BLS_V_MSGLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("copyb(0, BLS_DST_STR) -> r12"); raw("copyb(0, BLS_DST_LEN) -> r13"); raw("copyb(0, BLS_V_Q) -> r14")
raw("call zisklib_hash_to_curve_g2_bls12_381")
# c1 = pairing(pk, q)
raw("copyb(0, BLS_V_PK) -> r10"); raw("copyb(0, BLS_V_Q) -> r11"); raw("copyb(0, BLS_V_C1) -> r12"); raw("call zisklib_pairing_bls12_381")
# c2 = pairing(G1_GEN, sig)
raw("copyb(0, BLS_G1_GENERATOR) -> r10"); raw("copyb(0, BLS_V_SIG) -> r11"); raw("copyb(0, BLS_V_C2) -> r12"); raw("call zisklib_pairing_bls12_381")
# c1 == c2 ?
raw("copyb(0, BLS_V_C1) -> r10"); raw("copyb(0, BLS_V_C2) -> r11"); raw("copyb(0, 72) -> r12"); raw("call bn254_eqn")
raw("pop r1"); raw("ret")
w("lv_bv_bad:")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret"); w("")

open(_OUT+"/bls12_381/verify.zisk","w").write("\n".join(out)+"\n")
print("wrote verify.zisk", len(out), "lines; DST", len(DST), "bytes")
