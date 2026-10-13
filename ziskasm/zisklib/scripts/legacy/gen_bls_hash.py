#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bls12_381/hash.zisk : expand_message_xmd_sha256,
# os2ip_64_be_mod_p, hash_to_field_fp2_count2, hash_to_curve_g2. Mirrors
# hash_to_curve.rs (SHA-256 XMD, RFC 9380). Prefix lh_.
out=[]
def w(s=""): out.append(s)
def raw(s): w("\t"+s)
S="_bls12_381"

w("; ============================================================================")
w("; bls12_381/hash.zisk - hash_to_curve for G2 (RFC9380 BLS12381G2_XMD:SHA-256_")
w("; SSWU_RO_). GENERATED (scripts/legacy/gen_bls_hash.py). Prefix lh_.")
w("; ============================================================================")
w("")
w("const u64 BLS_R256_FP[6] = 0, 0, 0, 0, 1, 0")   # 2^256 mod p
# byte buffers (u64-backed)
w("u64 BLS_H_DSTP[40] = "+", ".join(["0"]*40))     # dst_prime up to 320B
w("u64 BLS_H_MSGP[160] = "+", ".join(["0"]*160))   # msg_prime up to 1280B
w("u64 BLS_H_BUF[48] = "+", ".join(["0"]*48))      # buf up to 384B
w("u64 BLS_H_UNIF[32] = "+", ".join(["0"]*32))     # uniform 256B
w("u64 BLS_H_B0[4] = 0, 0, 0, 0")
for nm in ["H_EJ","H_DHI","H_DLO"]:
    w(f"u64 BLS_{nm}[6] = 0, 0, 0, 0, 0, 0")
w("u64 BLS_H_U[24] = "+", ".join(["0"]*24))
for nm in ["H_Q0","H_Q1","H_RPT"]:
    w(f"u64 BLS_{nm}[24] = "+", ".join(["0"]*24))
for nm in ["H_MSG","H_MSGLEN","H_DST","H_DSTLEN","H_DSTPLEN","H_BUFLEN","H_RESP","H_I","H_TMP","H_FB_B","H_O2P"]:
    w(f"u64 BLS_{nm}[1] = 0")
w("")

# ---------- bytecopy(src r10, dst r11, count r12) ----------
w("; void zisklib_bytecopy(const u8* src, u8* dst, u64 count)  [r10=src,r11=dst,r12=count]")
w("zisklib_bytecopy:")
raw("eq(r12, 0), j(lh_bc_done)")
raw("copyb(r10, 1[a + 0]) -> r14")
raw("copyb(r11, r14) -> 1[a + 0]")
raw("add(r10, 1) -> r10"); raw("add(r11, 1) -> r11"); raw("sub(r12, 1) -> r12")
raw("jump(zisklib_bytecopy)")
w("lh_bc_done:")
raw("ret"); w("")

# ---------- be64(src r10) -> r10 : big-endian u64 ----------
w("; u64 zisklib_be64(const u8* src)  [r10=src] -> r10 = big-endian 8-byte int")
w("zisklib_be64:")
raw("copyb(0, 0) -> r6"); raw("copyb(0, 0) -> r7")
w("lh_be_loop:")
raw("eq(r7, 8), j(lh_be_done)")
raw("sll(r6, 8) -> r6")
raw("copyb(r10, 1[a + 0]) -> r14"); raw("or(r6, r14) -> r6")
raw("add(r10, 1) -> r10"); raw("add(r7, 1) -> r7")
raw("jump(lh_be_loop)")
w("lh_be_done:")
raw("copyb(0, r6) -> r10"); raw("ret"); w("")

# ---------- fill_be(bytes r10, dest6 r11): dest[3-i]=be64(bytes+i*8), top2=0 ----------
w("; void zisklib_fill_be(const u8* bytes32, u64* dest6)  fill 4 BE limbs (limb3..0) + zero top2")
w("zisklib_fill_be_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_H_TMP) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")   # save dest ptr
raw("copyb(0, BLS_H_FB_B) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")  # save bytes ptr (private slot)
raw("copyb(0, BLS_H_BUFLEN) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")  # i=0 (reuse BUFLEN as loop)
w("lh_fb_loop:")
raw("copyb(0, BLS_H_BUFLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("eq(r13, 4), j(lh_fb_done)")
raw("copyb(0, BLS_H_FB_B) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("sll(r13, 3) -> r6"); raw("add(r10, r6) -> r10")
raw("call zisklib_be64")     # r10 = be64(bytes + i*8)
raw("copyb(0, BLS_H_BUFLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("copyb(0, 3) -> r6"); raw("sub(r6, r13) -> r6"); raw("sll(r6, 3) -> r6")    # (3-i)*8
raw("copyb(0, BLS_H_TMP) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("add(r7, r6) -> r7")
raw("copyb(r7, r10) -> 8[a + 0]")
raw("add(r13, 1) -> r13"); raw("copyb(0, BLS_H_BUFLEN) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]")
raw("jump(lh_fb_loop)")
w("lh_fb_done:")
raw("copyb(0, BLS_H_TMP) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7")
raw("copyb(r7, 0) -> 8[a + 32]"); raw("copyb(r7, 0) -> 8[a + 40]")   # top 2 limbs = 0
raw("pop r1"); raw("ret"); w("")

# ---------- os2ip_be_mod_p(bytes64 r10) -> BLS_H_EJ ----------
w("; void zisklib_os2ip_be_mod_p(const u8* bytes64)  [r10] -> BLS_H_EJ[6] = int mod p")
w("zisklib_os2ip_be_mod_p_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_H_O2P) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")   # save bytes ptr (private slot)
raw("copyb(0, BLS_H_DHI) -> r11"); raw("call zisklib_fill_be_bls12_381")   # d_hi = bytes[0..32]
raw("copyb(0, BLS_H_O2P) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("add(r10, 32) -> r10"); raw("copyb(0, BLS_H_DLO) -> r11"); raw("call zisklib_fill_be_bls12_381")  # d_lo = bytes[32..64]
# ej = d_hi*2^256 + d_lo
raw("copyb(0, BLS_H_DHI) -> r10"); raw("copyb(0, BLS_R256_FP) -> r11"); raw("copyb(0, BLS_H_EJ) -> r12"); raw(f"call zisklib_mul_fp{S}")
raw("copyb(0, BLS_H_EJ) -> r10"); raw("copyb(0, BLS_H_DLO) -> r11"); raw("copyb(0, BLS_H_EJ) -> r12"); raw(f"call zisklib_add_fp{S}")
raw("pop r1"); raw("ret"); w("")

# ---------- expand_message_xmd (msg r10, msglen r11, dst r12, dstlen r13) -> BLS_H_UNIF (256B) ----------
w("; void zisklib_expand_message_xmd(msg,msglen,dst,dstlen) -> BLS_H_UNIF[256B] (len=256,ell=8)")
w("zisklib_expand_message_xmd_bls12_381:")
raw("push r1")
# Bound check: msg_prime = 64 || msg || 3 || dst_prime(dstlen+1) must fit BLS_H_MSGP (1280B).
# Emitted as .zisk comments so the guard is documented in the generated file too.
raw("; Bound check: msg_prime = 64 || msg || 3 || dst_prime(dstlen+1) must fit BLS_H_MSGP (1280B).")
raw("; Bytes needed = 68 + msglen + dstlen <= 1280  ->  msglen + dstlen <= 1212. dst_prime must also")
raw("; fit BLS_H_DSTP (320B); the reference asserts dst.len() <= 255. An over-long msg/dst would")
raw("; overflow the fixed buffers, so halt (matches the reference assert! panic on out-of-spec input).")
raw("ltu(255, r13), j(lh_em_toolong)")   # dstlen > 255 -> reject (reference assert; protects BLS_H_DSTP)
raw("ltu(1212, r11), j(lh_em_toolong)")  # msglen alone > 1212 -> reject (also prevents u64 add wrap)
raw("add(r11, r13) -> r6"); raw("ltu(1212, r6), j(lh_em_toolong)")  # 68 + msglen + dstlen > 1280
raw("copyb(0, BLS_H_MSG) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BLS_H_MSGLEN) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
raw("copyb(0, BLS_H_DST) -> r5"); raw("copyb(r5, r12) -> 8[a + 0]")
raw("copyb(0, BLS_H_DSTLEN) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]")
# dst_prime = dst || dstlen ; dstplen = dstlen+1
raw("copyb(0, BLS_H_DST) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_H_DSTP) -> r11"); raw("copyb(0, BLS_H_DSTLEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("call zisklib_bytecopy")
raw("copyb(0, BLS_H_DSTLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")   # dstlen
raw("copyb(0, BLS_H_DSTP) -> r7"); raw("add(r7, r6) -> r7"); raw("copyb(r7, r6) -> 1[a + 0]")   # dst_prime[dstlen]=dstlen
raw("add(r6, 1) -> r6"); raw("copyb(0, BLS_H_DSTPLEN) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")  # dstplen
# msg_prime: 64 zero bytes
raw("copyb(0, BLS_H_MSGP) -> r6"); raw("copyb(0, 8) -> r7")
w("lh_em_z:")
raw("eq(r7, 0), j(lh_em_zd)"); raw("copyb(r6, 0) -> 8[a + 0]"); raw("add(r6, 8) -> r6"); raw("sub(r7, 1) -> r7"); raw("jump(lh_em_z)")
w("lh_em_zd:")
# msg at +64
raw("copyb(0, BLS_H_MSG) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BLS_H_MSGP) -> r11"); raw("add(r11, 64) -> r11"); raw("copyb(0, BLS_H_MSGLEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("call zisklib_bytecopy")
# off = 64 + msglen
raw("copyb(0, BLS_H_MSGLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 64) -> r6")   # off in r6
raw("copyb(0, BLS_H_MSGP) -> r7"); raw("add(r7, r6) -> r7")
raw("copyb(r7, 1) -> 1[a + 0]"); raw("copyb(r7, 0) -> 1[a + 1]"); raw("copyb(r7, 0) -> 1[a + 2]")  # len=256 -> 01 00, then 00
raw("add(r6, 3) -> r6")   # off += 3
# dst_prime at off ; then msgplen = off + dstplen
raw("copyb(0, BLS_H_TMP) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")   # save off
raw("copyb(0, BLS_H_DSTP) -> r10"); raw("copyb(0, BLS_H_MSGP) -> r11"); raw("add(r11, r6) -> r11"); raw("copyb(0, BLS_H_DSTPLEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("call zisklib_bytecopy")
raw("copyb(0, BLS_H_TMP) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BLS_H_DSTPLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("add(r6, r7) -> r6")  # msgplen
# b_0 = sha256(msgp, msgplen) -> H_B0
raw("copyb(0, BLS_H_MSGP) -> r10"); raw("copyb(0, r6) -> r11"); raw("copyb(0, BLS_H_B0) -> r12"); raw("call ziskasm_zkvm_sha256")
# buf = b0 || 1 || dst_prime ; buflen = 33+dstplen
raw("copyb(0, BLS_H_B0) -> r10"); raw("copyb(0, BLS_H_BUF) -> r11"); raw("copyb(0, 32) -> r12"); raw("call zisklib_bytecopy")
raw("copyb(0, BLS_H_BUF) -> r7"); raw("add(r7, 32) -> r7"); raw("copyb(r7, 1) -> 1[a + 0]")
raw("copyb(0, BLS_H_DSTP) -> r10"); raw("copyb(0, BLS_H_BUF) -> r11"); raw("add(r11, 33) -> r11"); raw("copyb(0, BLS_H_DSTPLEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("call zisklib_bytecopy")
raw("copyb(0, BLS_H_DSTPLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 33) -> r6"); raw("copyb(0, BLS_H_BUFLEN) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")
# b_1 = sha256(buf) -> UNIF[0..32]
raw("copyb(0, BLS_H_BUF) -> r10"); raw("copyb(0, BLS_H_BUFLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BLS_H_UNIF) -> r12"); raw("call ziskasm_zkvm_sha256")
# for i=2..8
raw("copyb(0, BLS_H_I) -> r5"); raw("copyb(r5, 2) -> 8[a + 0]")
w("lh_em_loop:")
raw("copyb(0, BLS_H_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("eq(r13, 9), j(lh_em_done)")
# xored (4 u64) = B0 ^ UNIF[(i-2)*32] -> BUF[0..32]
raw("sub(r13, 2) -> r6"); raw("sll(r6, 5) -> r6"); raw("copyb(0, BLS_H_UNIF) -> r7"); raw("add(r7, r6) -> r7")   # prev ptr
raw("copyb(0, 0) -> r14")
w("lh_em_xor:")
raw("eq(r14, 4), j(lh_em_xd)")
raw("sll(r14, 3) -> r15")
raw("copyb(0, BLS_H_B0) -> r10"); raw("add(r10, r15) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")   # b0[w]
raw("add(r7, r15) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6")   # prev[w]
raw("xor(r10, r6) -> r10")
raw("copyb(0, BLS_H_BUF) -> r6"); raw("add(r6, r15) -> r6"); raw("copyb(r6, r10) -> 8[a + 0]")
raw("add(r14, 1) -> r14"); raw("jump(lh_em_xor)")
w("lh_em_xd:")
# BUF[32] = i
raw("copyb(0, BLS_H_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("copyb(0, BLS_H_BUF) -> r7"); raw("add(r7, 32) -> r7"); raw("copyb(r7, r13) -> 1[a + 0]")
# dst_prime at BUF+33
raw("copyb(0, BLS_H_DSTP) -> r10"); raw("copyb(0, BLS_H_BUF) -> r11"); raw("add(r11, 33) -> r11"); raw("copyb(0, BLS_H_DSTPLEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("call zisklib_bytecopy")
# b_i = sha256(buf) -> UNIF[(i-1)*32]
raw("copyb(0, BLS_H_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13"); raw("sub(r13, 1) -> r6"); raw("sll(r6, 5) -> r6")
raw("copyb(0, BLS_H_UNIF) -> r12"); raw("add(r12, r6) -> r12")
raw("copyb(0, BLS_H_TMP) -> r5"); raw("copyb(r5, r12) -> 8[a + 0]")    # save out ptr
raw("copyb(0, BLS_H_BUF) -> r10"); raw("copyb(0, BLS_H_BUFLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11")
raw("copyb(0, BLS_H_TMP) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("call ziskasm_zkvm_sha256")
raw("copyb(0, BLS_H_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13"); raw("add(r13, 1) -> r13"); raw("copyb(r5, r13) -> 8[a + 0]")
raw("jump(lh_em_loop)")
w("lh_em_done:")
raw("pop r1"); raw("ret"); w("")
w("lh_em_toolong:")
raw("copyb(0, 0) -> r10, end")   # halt: out-of-spec msg/dst length would overflow BLS_H_MSGP/DSTP
w("")

# ---------- hash_to_field_g2 (msg,msglen,dst,dstlen) -> BLS_H_U[24] ----------
w("; void zisklib_hash_to_field_g2(msg,msglen,dst,dstlen) -> BLS_H_U[24] (2 Fp2)")
w("zisklib_hash_to_field_g2_bls12_381:")
raw("push r1")
raw("call zisklib_expand_message_xmd_bls12_381")   # -> BLS_H_UNIF
# 4 chunks of 64B: chunk k -> os2ip -> place. u[i][j*6..], k=j+i*2, off=64*k
raw("copyb(0, BLS_H_I) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")   # k=0
w("lh_hf_loop:")
raw("copyb(0, BLS_H_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13")
raw("eq(r13, 4), j(lh_hf_done)")
raw("sll(r13, 6) -> r6"); raw("copyb(0, BLS_H_UNIF) -> r10"); raw("add(r10, r6) -> r10")   # chunk ptr
raw("call zisklib_os2ip_be_mod_p_bls12_381")   # -> BLS_H_EJ
# dest = BLS_H_U + k*48  (k maps to u[k/2][ (k%2)*6 ]; u element = 24 words? no: u[i]=12 words=Fp2; j*6 within; k=j+i*2 -> byte off = k*48)
raw("copyb(0, BLS_H_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13"); raw("sll(r13, 5) -> r6"); raw("sll(r13, 4) -> r7"); raw("add(r6, r7) -> r6")  # k*48
raw("copyb(0, BLS_H_EJ) -> r10"); raw("copyb(0, BLS_H_U) -> r11"); raw("add(r11, r6) -> r11"); raw("copyb(0, 6) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BLS_H_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13"); raw("add(r13, 1) -> r13"); raw("copyb(r5, r13) -> 8[a + 0]")
raw("jump(lh_hf_loop)")
w("lh_hf_done:")
raw("pop r1"); raw("ret"); w("")

# ---------- hash_to_curve_g2 (msg,msglen,dst,dstlen,result) ----------
w("; void zisklib_hash_to_curve_g2(msg r10,msglen r11,dst r12,dstlen r13,result r14) [result=24 u64]")
w("zisklib_hash_to_curve_g2_bls12_381:")
raw("push r1")
raw("copyb(0, BLS_H_RESP) -> r5"); raw("copyb(r5, r14) -> 8[a + 0]")
raw("call zisklib_hash_to_field_g2_bls12_381")   # -> BLS_H_U (u0=U[0..12], u1=U[12..24])
# q0 = map_no_cofactor(u0): copy u0->M2_U, swu, isogeny -> Q0
raw("copyb(0, BLS_H_U) -> r10"); raw("copyb(0, BLS_M2_U) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw(f"call zisklib_simple_swu_g2{S}")
raw("copyb(0, BLS_M2_PP) -> r10"); raw(f"call zisklib_isogeny_map_g2{S}")
raw("copyb(0, BLS_M2_ISOP) -> r10"); raw("copyb(0, BLS_H_Q0) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
# q1 = map_no_cofactor(u1)
raw("copyb(0, BLS_H_U) -> r10"); raw("add(r10, 96) -> r10"); raw("copyb(0, BLS_M2_U) -> r11"); raw("copyb(0, 12) -> r12"); raw("call bn254_memcpy")
raw(f"call zisklib_simple_swu_g2{S}")
raw("copyb(0, BLS_M2_PP) -> r10"); raw(f"call zisklib_isogeny_map_g2{S}")
raw("copyb(0, BLS_M2_ISOP) -> r10"); raw("copyb(0, BLS_H_Q1) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
# r = q0 + q1
raw("copyb(0, BLS_H_Q0) -> r10"); raw("copyb(0, BLS_H_Q1) -> r11"); raw("copyb(0, BLS_H_RPT) -> r12"); raw(f"call zisklib_add_complete_twist{S}")
# clear_cofactor(r) -> M2_A ; result = M2_A
raw("copyb(0, BLS_H_RPT) -> r10"); raw(f"call zisklib_clear_cofactor_twist{S}")
raw("copyb(0, BLS_M2_A) -> r10"); raw("copyb(0, BLS_H_RESP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 24) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

open(_OUT+"/bls12_381/hash.zisk","w").write("\n".join(out)+"\n")
print("wrote hash.zisk", len(out), "lines")
