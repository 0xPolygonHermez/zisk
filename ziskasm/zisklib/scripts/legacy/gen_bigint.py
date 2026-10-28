#!/usr/bin/env python3
import os as _os, sys as _sys
# Legacy first-version generator (see ../README.md). It writes into OUT_DIR, never
# into zisklib: the library files have had fixes and optimizations since.
if len(_sys.argv) < 2: _sys.exit(f"usage: {_sys.argv[0]} OUT_DIR")
_OUT = _os.path.abspath(_sys.argv[1])
_REPO = _os.path.abspath(_os.path.join(_os.path.dirname(__file__), *[".."] * 4))
for _d in ("bn254", "bls12_381", "bigint"): _os.makedirs(_os.path.join(_OUT, _d), exist_ok=True)
# Generate ziskasm/zisklib/bigint/*.zisk : arbitrary-precision arithmetic +
# modexp (EIP-198). Radix 2^256 (each "digit" = U256 = 4 u64). Hint-then-verify
# division. One file per ziskos bigint/*.rs source file. Prefix bi_.
FILES={}
CUR=["common"]
# one-line description per file (mirrors the matching bigint/*.rs)
DESC={
  "common":   "shared U256 constants, scratch buffers, comparisons (bi_lt/bi_eq/bi_iszero) and the bigint_div hint wrapper. Mirrors common.rs.",
  "add_short":"add a large number and a short (single-U256) number. Mirrors add_short.rs.",
  "add_agtb": "add two large numbers with a >= b. Mirrors add_agtb.rs.",
  "mul_short":"multiply a large number by a short number, and (a*b) mod m. Mirrors mul_short.rs.",
  "mul_long": "schoolbook multiply of two large numbers + mul-and-reduce. Mirrors mul_long.rs.",
  "rem_short":"remainder of a large number modulo a short number. Mirrors rem_short.rs.",
  "rem_long": "remainder of a large number modulo a large number. Mirrors rem_long.rs.",
  "modexp":   "modular exponentiation (EIP-198): bit-decomp, short/long cores, dispatcher. Mirrors modexp.rs.",
}
def setf(name): CUR[0]=name; FILES.setdefault(name,[])
def w(s=""): FILES.setdefault(CUR[0],[]).append(s)
def raw(s): w("\t"+s)

MAXW=128         # max u64 limbs for base/modulus operands (32 U256 = 8192-bit)
MAXE=128         # max u64 limbs for the exponent (8192-bit)
MAXBITS=MAXE*64  # max exponent bits

setf("common")
w("const u64 BI_ZERO[4] = 0, 0, 0, 0")
w("const u64 BI_ONE[4] = 1, 0, 0, 0")
w("")
# arith headers + scratch
w("u64 BI_HDR[5] = 0, 0, 0, 0, 0")
w("u64 BI_DH[4] = 0, 0, 0, 0")
w("u64 BI_CIN[4] = 0, 0, 0, 0")
w("u64 BI_CARRY[4] = 0, 0, 0, 0")
# loop-state RAM (single-word pointers/counters)
for nm in ["BI_APTR","BI_BPTR","BI_OPTR","BI_LEN","BI_I","BI_CARRYNZ"]:
    w(f"u64 {nm}[1] = 0")
w("")

setf("mul_short")
# ---------- mulmod_short(a r10, b r11, modulus r12, result r13) ----------
w("; void zisklib_mulmod_short(const u64* a, const u64* b, const u64* modulus, u64* result)")
w(";   result = (a*b) mod modulus  (all single U256). arith256_mod precompile.")
w("zisklib_mulmod_short:")
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(r5, r11) -> 8[a + 8]")
raw("copyb(0, BI_ZERO) -> r6"); raw("copyb(r5, r6) -> 8[a + 16]")   # &c = 0
raw("copyb(r5, r12) -> 8[a + 24]")                                   # &module
raw("copyb(r5, r13) -> 8[a + 32]")                                   # &d
raw("arith256_mod(0, r5) -> r14")
raw("ret"); w("")

# ---------- mul_short(a r10, len_a r11, b r12, out r13) -> r10 = out_len ----------
# out = a * b  (a = len_a U256 digits, b = 1 U256). schoolbook, radix 2^256.
w("; u64 zisklib_mul_short(const u64* a, u64 len_a, const u64* b, u64* out) -> r10 = out_len")
w(";   out = a*b (a has len_a U256 digits, b is one U256). out needs len_a+1 digits.")
w("zisklib_mul_short:")
raw("push r1")
raw("copyb(0, BI_APTR) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
raw("copyb(0, BI_BPTR) -> r5"); raw("copyb(r5, r12) -> 8[a + 0]")
raw("copyb(0, BI_OPTR) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]")
# carry = 0
raw("copyb(0, BI_CARRY) -> r5")
raw("copyb(r5, 0) -> 8[a + 0]"); raw("copyb(r5, 0) -> 8[a + 8]"); raw("copyb(r5, 0) -> 8[a + 16]"); raw("copyb(r5, 0) -> 8[a + 24]")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("bi_ms_loop:")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")     # i
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7")   # len
raw("eq(r6, r7), j(bi_ms_done)")
# cin = carry (copy 4 words) to avoid c/dh alias — do this FIRST (memcpy clobbers regs)
raw("copyb(0, BI_CARRY) -> r10"); raw("copyb(0, BI_CIN) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
# reload i and compute i*32 AFTER the memcpy
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("sll(r6, 5) -> r16")
# header [&a[i], &b, &cin, &out[i], &carry]
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(0, BI_APTR) -> r17"); raw("copyb(r17, 8[a + 0]) -> r17"); raw("add(r17, r16) -> r17"); raw("copyb(r5, r17) -> 8[a + 0]")   # &a[i]
raw("copyb(0, BI_BPTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("copyb(r5, r6) -> 8[a + 8]")                              # &b
raw("copyb(0, BI_CIN) -> r6"); raw("copyb(r5, r6) -> 8[a + 16]")                                                                # &c = cin
raw("copyb(0, BI_OPTR) -> r14"); raw("copyb(r14, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14"); raw("copyb(r5, r14) -> 8[a + 24]")  # &dl = out[i]
raw("copyb(0, BI_CARRY) -> r6"); raw("copyb(r5, r6) -> 8[a + 32]")                                                              # &dh = carry
raw("arith256(0, r5) -> r13")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("jump(bi_ms_loop)")
w("bi_ms_done:")
# if carry != 0: out[len] = carry ; return len+1 else len
raw("copyb(0, BI_CARRY) -> r6")
raw("copyb(r6, 8[a + 0]) -> r7"); raw("copyb(r6, 8[a + 8]) -> r16"); raw("or(r7, r16) -> r7")
raw("copyb(r6, 8[a + 16]) -> r16"); raw("or(r7, r16) -> r7"); raw("copyb(r6, 8[a + 24]) -> r16"); raw("or(r7, r16) -> r7")
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10")     # default return = len
raw("eq(r7, 0), j(bi_ms_ret)")
# out[len] = carry
raw("sll(r10, 5) -> r16")
raw("copyb(0, BI_OPTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")
raw("copyb(0, BI_CARRY) -> r10"); raw("copyb(0, r14) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("add(r10, 1) -> r10")
w("bi_ms_ret:")
raw("pop r1"); raw("ret"); w("")

setf("add_short")
# ---------- add_short(a r10, len_a r11, b r12, out r13) -> r10 = out_len ----------
w("; u64 zisklib_add_short(const u64* a, u64 len_a, const u64* b, u64* out) -> r10 = out_len")
w(";   out = a + b (a has len_a U256 digits, b is one U256). add256 precompile.")
w("zisklib_add_short:")
raw("push r1")
raw("copyb(0, BI_APTR) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
raw("copyb(0, BI_BPTR) -> r5"); raw("copyb(r5, r12) -> 8[a + 0]")
raw("copyb(0, BI_OPTR) -> r5"); raw("copyb(r5, r13) -> 8[a + 0]")
# out[0] = a[0] + b ; carry
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(r5, r10) -> 8[a + 0]")                                  # &a[0]
raw("copyb(r5, r12) -> 8[a + 8]")                                  # &b
raw("copyb(r5, 0) -> 8[a + 16]")                                   # cin = 0 (value)
raw("copyb(r5, r13) -> 8[a + 24]")                                 # &c = out[0]
raw("add256(0, r5) -> r15")                                        # carry -> r15
# i = 1
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 1) -> 8[a + 0]")
w("bi_as_loop:")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")      # i
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7")    # len
raw("eq(r6, r7), j(bi_as_done)")
raw("sll(r6, 5) -> r16")                                            # i*32
raw("copyb(0, BI_APTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r17"); raw("add(r17, r16) -> r17")     # &a[i]
raw("copyb(0, BI_OPTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")  # &out[i]
raw("eq(r15, 0), j(bi_as_copy)")
# out[i] = a[i] + 0 + carry
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(r5, r17) -> 8[a + 0]")
raw("copyb(0, BI_ZERO) -> r6"); raw("copyb(r5, r6) -> 8[a + 8]")
raw("copyb(r5, 1) -> 8[a + 16]")                                   # cin = 1
raw("copyb(r5, r14) -> 8[a + 24]")
raw("add256(0, r5) -> r15")
raw("jump(bi_as_next)")
w("bi_as_copy:")
# out[i] = a[i]  (memcpy 4). carry is 0 here and stays 0; restore r15 after the call.
raw("copyb(0, r17) -> r10"); raw("copyb(0, r14) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, 0) -> r15")
w("bi_as_next:")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("jump(bi_as_loop)")
w("bi_as_done:")
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10")
raw("eq(r15, 0), j(bi_as_ret)")
# out[len] = 1 ; return len+1
raw("sll(r10, 5) -> r16")
raw("copyb(0, BI_OPTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")
raw("copyb(r14, 1) -> 8[a + 0]"); raw("copyb(r14, 0) -> 8[a + 8]"); raw("copyb(r14, 0) -> 8[a + 16]"); raw("copyb(r14, 0) -> 8[a + 24]")
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("add(r10, 1) -> r10")
w("bi_as_ret:")
raw("pop r1"); raw("ret"); w("")

setf("common")
# ================= additional data (fcall wrappers, rem_short, bin_decomp, modexp) =====
for nm in ["BI_QUOP","BI_REMP","BI_LENB","BI_LENQ","BI_LENR","BI_J","BI_EXPP","BI_MODP",
           "BI_RESP","BI_BASEP","BI_LENBASE","BI_NBITS",
           "BI_RS_A","BI_RS_LEN","BI_RS_B","BI_RS_RES","BI_RS_QLEN","BI_RS_QBLEN",
           "BI_ME_BASE","BI_ME_LENB","BI_ME_EXP","BI_ME_LENE","BI_ME_MOD","BI_ME_RES","BI_ME_J"]:
    w(f"u64 {nm}[1] = 0")
P=2*MAXW+8   # product/quotient buffer size (u64): up to 2*len_m U256
w(f"u64 BI_QUO[{P}] = "+", ".join(["0"]*P))
w(f"u64 BI_REM[{MAXW+4}] = "+", ".join(["0"]*(MAXW+4)))
w(f"u64 BI_QB[{P}] = "+", ".join(["0"]*P))
w(f"u64 BI_QBR[{P}] = "+", ".join(["0"]*P))
w(f"u64 BI_MUL[{P}] = "+", ".join(["0"]*P))
w(f"u64 BI_BITS[{MAXE}] = "+", ".join(["0"]*MAXE))   # packed: 1 bit per bit (MAXE u64 = MAXBITS bits)
w(f"u64 BI_RECEXP[{MAXE}] = "+", ".join(["0"]*MAXE))
w(f"u64 BI_OUT[{MAXW+4}] = "+", ".join(["0"]*(MAXW+4)))
w(f"u64 BI_BASE[{MAXW+4}] = "+", ".join(["0"]*(MAXW+4)))
w(f"u64 BI_TMP[{MAXW+4}] = "+", ".join(["0"]*(MAXW+4)))
w("")

def save(slot,reg): raw(f"copyb(0, {slot}) -> r5"); raw(f"copyb(r5, {reg}) -> 8[a + 0]")
def load(reg,slot): raw(f"copyb(0, {slot}) -> {reg}"); raw(f"copyb({reg}, 8[a + 0]) -> {reg}")

# ---------- bigint_div(a r10, lena_u64 r11, b r12, lenb_u64 r13, quo r14, rem r15) ----------
#   -> r10 = len_quo_u64, r11 = len_rem_u64. Appends params then reads (q,r) hint.
w("; (usize,usize) zisklib_bigint_div(const u64* a, u64 lena, const u64* b, u64 lenb, u64* quo, u64* rem)")
w(";   hint quotient+remainder via FCALL_BIGINT_DIV. r10=len_quo, r11=len_rem (u64 counts).")
w("zisklib_bigint_div:")
raw("push r1")
save("BI_APTR","r10"); save("BI_LEN","r11"); save("BI_BPTR","r12"); save("BI_LENB","r13"); save("BI_QUOP","r14"); save("BI_REMP","r15")
raw("fcall_param(1, r11) -> r14")                                  # len_a
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")       # j=0
w("bi_bd_ap:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7")
raw("eq(r6, r7), j(bi_bd_apd)")
raw("sll(r6, 3) -> r16"); raw("copyb(0, BI_APTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r17"); raw("add(r17, r16) -> r17"); raw("copyb(r17, 8[a + 0]) -> r17")
raw("fcall_param(1, r17) -> r14")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("jump(bi_bd_ap)")
w("bi_bd_apd:")
raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("fcall_param(1, r6) -> r14")   # len_b
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("bi_bd_bp:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7")
raw("eq(r6, r7), j(bi_bd_bpd)")
raw("sll(r6, 3) -> r16"); raw("copyb(0, BI_BPTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r17"); raw("add(r17, r16) -> r17"); raw("copyb(r17, 8[a + 0]) -> r17")
raw("fcall_param(1, r17) -> r14")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("jump(bi_bd_bp)")
w("bi_bd_bpd:")
raw("fcall(FCALL_BIGINT_DIV, 0) -> r14")
# len_quo = result[0] (already in [FREE_INPUT])
raw("copyb(0, [FREE_INPUT]) -> r6"); raw("copyb(0, BI_LENQ) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("bi_bd_rq:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("copyb(0, BI_LENQ) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7")
raw("eq(r6, r7), j(bi_bd_rqd)")
raw("fcall_get(0, 0) -> r14"); raw("copyb(0, [FREE_INPUT]) -> r17")
raw("sll(r6, 3) -> r16"); raw("copyb(0, BI_QUOP) -> r5"); raw("copyb(r5, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14"); raw("copyb(r14, r17) -> 8[a + 0]")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("jump(bi_bd_rq)")
w("bi_bd_rqd:")
# len_rem = next result
raw("fcall_get(0, 0) -> r14"); raw("copyb(0, [FREE_INPUT]) -> r6"); raw("copyb(0, BI_LENR) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("bi_bd_rr:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6")
raw("copyb(0, BI_LENR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7")
raw("eq(r6, r7), j(bi_bd_rrd)")
raw("fcall_get(0, 0) -> r14"); raw("copyb(0, [FREE_INPUT]) -> r17")
raw("sll(r6, 3) -> r16"); raw("copyb(0, BI_REMP) -> r5"); raw("copyb(r5, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14"); raw("copyb(r14, r17) -> 8[a + 0]")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("jump(bi_bd_rr)")
w("bi_bd_rrd:")
raw("copyb(0, BI_LENQ) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10")
raw("copyb(0, BI_LENR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r11")
raw("pop r1"); raw("ret"); w("")

setf("rem_short")
# ---------- rem_short_init(a r10, lena_u256 r11, b r12, result r13) ----------
w("; void zisklib_rem_short_init(const u64* a, u64 lena_u256, const u64* b, u64* result)")
w(";   result = a mod b (b is one U256). hint (q,r) then verify a == q*b + r, r < b.")
w("zisklib_rem_short_init:")
raw("push r1")
# private slots (sub-calls clobber BI_APTR/BI_LEN/BI_LENB/... )
save("BI_RS_A","r10"); save("BI_RS_LEN","r11"); save("BI_RS_B","r12"); save("BI_RS_RES","r13")
# edge: len_a == 1
raw("copyb(0, 1) -> r5"); raw("copyb(0, BI_RS_LEN) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("eq(r6, r5), j(bi_rs_len1)")
raw("jump(bi_rs_general)")
w("bi_rs_len1:")
raw("copyb(0, BI_RS_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_RS_B) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_ltn")
raw("eq(r10, 0), j(bi_rs_ge)")
# a1 < b -> result = a1
raw("copyb(0, BI_RS_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_RS_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret")
w("bi_rs_ge:")
raw("copyb(0, BI_RS_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_RS_B) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_eqn")
raw("eq(r10, 0), j(bi_rs_general)")
# a1 == b -> result = 0
raw("copyb(0, BI_ZERO) -> r10"); raw("copyb(0, BI_RS_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret")
w("bi_rs_general:")
# bigint_div(a, lena*4, b, 4, BI_QUO, BI_REM)
raw("copyb(0, BI_RS_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BI_RS_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r11"); raw("sll(r11, 2) -> r11")   # lena*4 u64
raw("copyb(0, BI_RS_B) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12")
raw("copyb(0, 4) -> r13")
raw("copyb(0, BI_QUO) -> r14"); raw("copyb(0, BI_REM) -> r15")
raw("call zisklib_bigint_div")   # r10 = len_quo_u64, r11 = len_rem_u64
raw("srl(r10, 2) -> r10"); raw("copyb(0, BI_RS_QLEN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")   # len_quo_u256
# q_b = mul_short(BI_QUO, len_quo_u256, b, BI_QB)
raw("copyb(0, BI_QUO) -> r10"); raw("copyb(0, BI_RS_QLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_RS_B) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_QB) -> r13")
raw("call zisklib_mul_short")   # r10 = q_b_len (U256)
raw("copyb(0, BI_RS_QBLEN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
# rem == 0 ?
raw("copyb(0, BI_REM) -> r10"); raw("copyb(0, BI_ZERO) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_eqn")
raw("ltu(0, r10), j(bi_rs_remzero)")
# rem != 0: assert rem < b ; q_b_r = add_short(q_b, q_b_len, rem, BI_QBR) ; assert a == q_b_r, len == lena
raw("copyb(0, BI_REM) -> r10"); raw("copyb(0, BI_RS_B) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_ltn"); raw("eq(r10, 0), j(bi_rs_bad)")
raw("copyb(0, BI_QB) -> r10"); raw("copyb(0, BI_RS_QBLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_REM) -> r12"); raw("copyb(0, BI_QBR) -> r13"); raw("call zisklib_add_short")   # r10 = len
raw("copyb(0, BI_RS_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("eq(r10, r6), j(bi_rs_cmp_qbr)"); raw("jump(bi_rs_bad)")
w("bi_rs_cmp_qbr:")
raw("sll(r10, 2) -> r12"); raw("copyb(0, BI_RS_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_QBR) -> r11"); raw("call bn254_eqn"); raw("eq(r10, 0), j(bi_rs_bad)")
raw("jump(bi_rs_setres)")
w("bi_rs_remzero:")
raw("copyb(0, BI_RS_QBLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("copyb(0, BI_RS_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("eq(r10, r6), j(bi_rs_cmp_qb)"); raw("jump(bi_rs_bad)")
w("bi_rs_cmp_qb:")
raw("copyb(0, BI_RS_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r12"); raw("sll(r12, 2) -> r12"); raw("copyb(0, BI_RS_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_QB) -> r11"); raw("call bn254_eqn"); raw("eq(r10, 0), j(bi_rs_bad)")
w("bi_rs_setres:")
raw("copyb(0, BI_REM) -> r10"); raw("copyb(0, BI_RS_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret")
w("bi_rs_bad:")
raw("copyb(0, 0) -> r10, end"); w("")

setf("modexp")
# ---------- bin_decomp(exp r10, len_e_u64 r11) -> r10 = len_bits ; bits in BI_BITS ----------
w("; u64 zisklib_bin_decomp(const u64* exp, u64 len_e) -> r10 = len_bits ; MSB-first bits in BI_BITS")
w(";   hints the bit decomposition, then recomposes and asserts it equals exp.")
w("zisklib_bin_decomp:")
raw("push r1")
save("BI_EXPP","r10"); save("BI_LENB","r11")   # BI_LENB = len_e
# zero BI_BITS (packed) — bin_decomp ORs bits in and is called once per modexp
raw("copyb(0, BI_BITS) -> r6"); raw("copyb(0, 0) -> r7")
w("bi_bc_zbits:")
raw(f"eq(r7, {MAXE}), j(bi_bc_zbitsd)"); raw("copyb(r6, 0) -> 8[a + 0]"); raw("add(r6, 8) -> r6"); raw("add(r7, 1) -> r7"); raw("jump(bi_bc_zbits)")
w("bi_bc_zbitsd:")
raw("fcall_param(1, r11) -> r14")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("bi_bc_ap:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("eq(r6, r7), j(bi_bc_apd)")
raw("sll(r6, 3) -> r16"); raw("copyb(0, BI_EXPP) -> r5"); raw("copyb(r5, 8[a + 0]) -> r17"); raw("add(r17, r16) -> r17"); raw("copyb(r17, 8[a + 0]) -> r17")
raw("fcall_param(1, r17) -> r14")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_bc_ap)")
w("bi_bc_apd:")
raw("fcall(FCALL_BIN_DECOMP, 0) -> r14")
raw("copyb(0, [FREE_INPUT]) -> r6"); raw("copyb(0, BI_NBITS) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")   # len_bits
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("bi_bc_rb:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_NBITS) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("eq(r6, r7), j(bi_bc_rbd)")
raw("fcall_get(0, 0) -> r14"); raw("copyb(0, [FREE_INPUT]) -> r17")
raw("srl(r6, 6) -> r16"); raw("sll(r16, 3) -> r16"); raw("copyb(0, BI_BITS) -> r5"); raw("add(r5, r16) -> r5"); raw("and(r6, 63) -> r7"); raw("sll(r17, r7) -> r17"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("or(r7, r17) -> r7"); raw("copyb(r5, r7) -> 8[a + 0]")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_bc_rb)")
w("bi_bc_rbd:")
# assert bits[0] == 1
raw("copyb(0, BI_BITS) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("and(r6, 1) -> r6"); raw("copyb(0, 1) -> r7"); raw("eq(r6, r7), j(bi_bc_recinit)"); raw("jump(bi_bc_bad)")
w("bi_bc_recinit:")
# rec_exp = 0 (len_e words)
raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7")   # len_e
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("bi_bc_zero:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("eq(r6, r7), j(bi_bc_zerod)")
raw("sll(r6, 3) -> r16"); raw("copyb(0, BI_RECEXP) -> r5"); raw("add(r5, r16) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_bc_zero)")
w("bi_bc_zerod:")
# for bit_idx 0..len_bits: if bits[bit_idx]==1: pos=len_bits-1-bit_idx; recexp[pos>>6] |= 1<<(pos&63)
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("bi_bc_rec:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_NBITS) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("eq(r6, r7), j(bi_bc_recd)")
# bit = BITS[bit_idx]
raw("srl(r6, 6) -> r16"); raw("sll(r16, 3) -> r16"); raw("copyb(0, BI_BITS) -> r5"); raw("add(r5, r16) -> r5"); raw("copyb(r5, 8[a + 0]) -> r5"); raw("and(r6, 63) -> r16"); raw("srl(r5, r16) -> r5"); raw("and(r5, 1) -> r17")   # packed bit
raw("eq(r17, 0), j(bi_bc_recnext)")
# pos = nbits - 1 - bit_idx
raw("copyb(0, BI_NBITS) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("sub(r7, 1) -> r7"); raw("sub(r7, r6) -> r7")   # pos
raw("srl(r7, 6) -> r16"); raw("and(r7, 63) -> r7")   # word idx, bit
raw("copyb(0, 1) -> r14"); raw("sll(r14, r7) -> r14")   # mask = 1<<bit
raw("sll(r16, 3) -> r16"); raw("copyb(0, BI_RECEXP) -> r5"); raw("add(r5, r16) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("or(r6, r14) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")
w("bi_bc_recnext:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_bc_rec)")
w("bi_bc_recd:")
# assert rec_exp == exp (len_e words)
raw("copyb(0, BI_RECEXP) -> r10"); raw("copyb(0, BI_EXPP) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_LENB) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("call bn254_eqn"); raw("eq(r10, 0), j(bi_bc_bad)")
raw("copyb(0, BI_NBITS) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10")
raw("pop r1"); raw("ret")
w("bi_bc_bad:")
raw("copyb(0, 0) -> r10, end"); w("")

# ---------- modexp_short(base r10, lenb_u256 r11, exp r12, lene_u64 r13, mod r14, result r15) ----------
w("; void zisklib_modexp_short(const u64* base, u64 lenb_u256, const u64* exp, u64 lene, const u64* mod, u64* result)")
w(";   result = base^exp mod mod  (mod is one U256, result is one U256).")
w("zisklib_modexp_short:")
raw("push r1")
save("BI_ME_BASE","r10"); save("BI_ME_LENB","r11"); save("BI_ME_EXP","r12"); save("BI_ME_LENE","r13"); save("BI_ME_MOD","r14"); save("BI_ME_RES","r15")
# base_r = rem_short_init(base, lenb, mod) -> BI_BASE
raw("copyb(0, BI_ME_BASE) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_ME_LENB) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_ME_MOD) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_BASE) -> r13"); raw("call zisklib_rem_short_init")
# bits = bin_decomp(exp, lene) -> len_bits (sets BI_NBITS)
raw("copyb(0, BI_ME_EXP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_ME_LENE) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("call zisklib_bin_decomp")
# out = base_r
raw("copyb(0, BI_BASE) -> r10"); raw("copyb(0, BI_OUT) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
# for bit_idx 1..len_bits: if out==0 break; out=mulmod(out,out,mod); if bit: out=mulmod(out,base,mod)
raw("copyb(0, BI_ME_J) -> r5"); raw("copyb(r5, 1) -> 8[a + 0]")
w("bi_me_loop:")
raw("copyb(0, BI_ME_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_NBITS) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("eq(r6, r7), j(bi_me_done)")
# if out == 0 break
raw("copyb(0, BI_OUT) -> r10"); raw("copyb(0, BI_ZERO) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_eqn"); raw("ltu(0, r10), j(bi_me_done)")
# out = out^2 mod mod
raw("copyb(0, BI_OUT) -> r10"); raw("copyb(0, BI_OUT) -> r11"); raw("copyb(0, BI_ME_MOD) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_TMP) -> r13"); raw("call zisklib_mulmod_short")
raw("copyb(0, BI_TMP) -> r10"); raw("copyb(0, BI_OUT) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
# bit = BITS[bit_idx]
raw("copyb(0, BI_ME_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("srl(r6, 6) -> r16"); raw("sll(r16, 3) -> r16"); raw("copyb(0, BI_BITS) -> r5"); raw("add(r5, r16) -> r5"); raw("copyb(r5, 8[a + 0]) -> r5"); raw("and(r6, 63) -> r16"); raw("srl(r5, r16) -> r5"); raw("and(r5, 1) -> r17")
raw("eq(r17, 0), j(bi_me_next)")
# out = out * base mod mod
raw("copyb(0, BI_OUT) -> r10"); raw("copyb(0, BI_BASE) -> r11"); raw("copyb(0, BI_ME_MOD) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_TMP) -> r13"); raw("call zisklib_mulmod_short")
raw("copyb(0, BI_TMP) -> r10"); raw("copyb(0, BI_OUT) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
w("bi_me_next:")
raw("copyb(0, BI_ME_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_me_loop)")
w("bi_me_done:")
raw("copyb(0, BI_OUT) -> r10"); raw("copyb(0, BI_ME_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_memcpy")
raw("pop r1"); raw("ret"); w("")

setf("common")
# ================= LONG PATH =================
for nm in ["BI_RL_A","BI_RL_LENA","BI_RL_B","BI_RL_LENB","BI_RL_RES","BI_RL_QLEN","BI_RL_QBLEN","BI_RL_RLEN",
           "BI_ML2_BASE","BI_ML2_LENB","BI_ML2_EXP","BI_ML2_LENE","BI_ML2_MOD","BI_ML2_LENM","BI_ML2_RES",
           "BI_ML2_OUTLEN","BI_ML2_BASELEN","BI_ML2_J","BI_MR_MOD","BI_MR_LENM","BI_MR_OUT","BI_K"]:
    w(f"u64 {nm}[1] = 0")
w(f"u64 BI_OUT2[{MAXW+4}] = "+", ".join(["0"]*(MAXW+4)))
w("")

setf("add_agtb")
# ---------- add_agtb(a r10, lena r11, b r12, lenb r13, out r14) -> r10 out_len ----------
# a >= b, both multi-U256. out = a + b. Uses add256 only (no sub-calls).
w("; u64 zisklib_add_agtb(const u64* a, u64 lena, const u64* b, u64 lenb, u64* out) -> r10 out_len")
w("zisklib_add_agtb:")
save("BI_APTR","r10"); save("BI_LEN","r11"); save("BI_BPTR","r12"); save("BI_LENB","r13"); save("BI_OPTR","r14")
# out[0] = a[0] + b[0], cin=0
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(r5, r10) -> 8[a + 0]"); raw("copyb(r5, r12) -> 8[a + 8]"); raw("copyb(r5, 0) -> 8[a + 16]"); raw("copyb(r5, r14) -> 8[a + 24]")
raw("add256(0, r5) -> r15")
raw("copyb(0, BI_CARRYNZ) -> r5"); raw("copyb(r5, r15) -> 8[a + 0]")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 1) -> 8[a + 0]")
w("bi_ag_loop:")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("eq(r6, r7), j(bi_ag_done)")
raw("sll(r6, 5) -> r16")
raw("copyb(0, BI_APTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("add(r10, r16) -> r10")   # &a[i]
raw("copyb(0, BI_OPTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")   # &out[i]
# &b[i] if i<len_b else &ZERO
raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("ltu(r6, r7), j(bi_ag_bi)")
raw("copyb(0, BI_ZERO) -> r12"); raw("jump(bi_ag_hdr)")
w("bi_ag_bi:")
raw("copyb(0, BI_BPTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r12"); raw("add(r12, r16) -> r12")
w("bi_ag_hdr:")
raw("copyb(0, BI_CARRYNZ) -> r5"); raw("copyb(r5, 8[a + 0]) -> r17")   # carry
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(r5, r10) -> 8[a + 0]"); raw("copyb(r5, r12) -> 8[a + 8]"); raw("copyb(r5, r17) -> 8[a + 16]"); raw("copyb(r5, r14) -> 8[a + 24]")
raw("add256(0, r5) -> r15")
raw("copyb(0, BI_CARRYNZ) -> r5"); raw("copyb(r5, r15) -> 8[a + 0]")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_ag_loop)")
w("bi_ag_done:")
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10")
raw("copyb(0, BI_CARRYNZ) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("eq(r6, 0), j(bi_ag_ret)")
raw("sll(r10, 5) -> r16"); raw("copyb(0, BI_OPTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")
raw("copyb(r14, 1) -> 8[a + 0]"); raw("copyb(r14, 0) -> 8[a + 8]"); raw("copyb(r14, 0) -> 8[a + 16]"); raw("copyb(r14, 0) -> 8[a + 24]")
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("add(r10, 1) -> r10")
w("bi_ag_ret:")
raw("ret"); w("")

setf("mul_long")
# ---------- mul_long(a r10, lena r11, b r12, lenb r13, out r14) -> r10 out_len ----------
w("; u64 zisklib_mul_long(const u64* a, u64 lena, const u64* b, u64 lenb, u64* out) -> r10 out_len")
w(";   schoolbook radix-2^256. out needs lena+lenb U256 (zeroed here). Uses arith256+add256.")
w("zisklib_mul_long:")
save("BI_APTR","r10"); save("BI_LEN","r11"); save("BI_BPTR","r12"); save("BI_LENB","r13"); save("BI_OPTR","r14")
# zero out[0..(lena+lenb)] U256
# zero out[0..(lena+lenb)] U256 via the dma_xmemset precompile (one op)
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("add(r6, r7) -> r6"); raw("sll(r6, 5) -> r6")   # (lena+lenb)*32 bytes
raw("copyb(0, BI_OPTR) -> r14"); raw("copyb(r14, 8[a + 0]) -> r14")
raw("dma_xmemset(r14, r6) -> r5, j(0, 4)")
# out[0],out[1] = a[0]*b[0] + 0
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(0, BI_APTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")   # &a[0]
raw("copyb(0, BI_BPTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("copyb(r5, r6) -> 8[a + 8]")   # &b[0]
raw("copyb(0, BI_ZERO) -> r6"); raw("copyb(r5, r6) -> 8[a + 16]")
raw("copyb(0, BI_OPTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r14"); raw("copyb(r5, r14) -> 8[a + 24]")   # &out[0]
raw("add(r14, 32) -> r14"); raw("copyb(r5, r14) -> 8[a + 32]")   # &out[1]
raw("arith256(0, r5) -> r13")
# for j in 1..lenb: arith(a[0], b[j], out[j]) -> out[j]=dl, out[j+1]=dh
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 1) -> 8[a + 0]")
w("bi_mll_j0:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("eq(r6, r7), j(bi_mll_j0d)")
raw("sll(r6, 5) -> r16")
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(0, BI_APTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")   # &a[0]
raw("copyb(0, BI_BPTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("add(r6, r16) -> r6"); raw("copyb(r5, r6) -> 8[a + 8]")   # &b[j]
raw("copyb(0, BI_OPTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")   # &out[j]
raw("copyb(r5, r14) -> 8[a + 16]")   # &c = out[j]
raw("copyb(r5, r14) -> 8[a + 24]")   # &dl = out[j]
raw("add(r14, 32) -> r14"); raw("copyb(r5, r14) -> 8[a + 32]")   # &dh = out[j+1]
raw("arith256(0, r5) -> r13")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_mll_j0)")
w("bi_mll_j0d:")
# for i in 1..lena
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 1) -> 8[a + 0]")
w("bi_mll_i:")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("eq(r6, r7), j(bi_mll_id)")
raw("copyb(0, BI_CARRYNZ) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")   # carry=0
# for j in 0..lenb-1
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 0) -> 8[a + 0]")
w("bi_mll_ij:")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("sub(r7, 1) -> r7"); raw("eq(r6, r7), j(bi_mll_ijlast)")
# k = i+j ; arith(a[i], b[j], out[k]) -> out[k]=dl, dh=BI_DH ; then out[k+1],carry = add256(out[k+1], BI_DH, carry)
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r17"); raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r17, r6) -> r17"); raw("sll(r17, 5) -> r16")   # k*32
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(0, BI_APTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("copyb(0, BI_I) -> r7"); raw("copyb(r7, 8[a + 0]) -> r7"); raw("sll(r7, 5) -> r7"); raw("add(r6, r7) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")   # &a[i]
raw("copyb(0, BI_BPTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("copyb(0, BI_J) -> r7"); raw("copyb(r7, 8[a + 0]) -> r7"); raw("sll(r7, 5) -> r7"); raw("add(r6, r7) -> r6"); raw("copyb(r5, r6) -> 8[a + 8]")   # &b[j]
raw("copyb(0, BI_OPTR) -> r14"); raw("copyb(r14, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")   # &out[k]
raw("copyb(r5, r14) -> 8[a + 16]"); raw("copyb(r5, r14) -> 8[a + 24]")   # &c=&dl=out[k]
raw("copyb(0, BI_DH) -> r6"); raw("copyb(r5, r6) -> 8[a + 32]")   # &dh = BI_DH
raw("arith256(0, r5) -> r13")
# out[k+1], carry = add256(out[k+1], BI_DH, cin=carry)
raw("copyb(0, BI_CARRYNZ) -> r5"); raw("copyb(r5, 8[a + 0]) -> r17")   # carry
raw("copyb(0, BI_OPTR) -> r14"); raw("copyb(r14, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14"); raw("add(r14, 32) -> r14")   # &out[k+1]
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(r5, r14) -> 8[a + 0]"); raw("copyb(0, BI_DH) -> r6"); raw("copyb(r5, r6) -> 8[a + 8]"); raw("copyb(r5, r17) -> 8[a + 16]"); raw("copyb(r5, r14) -> 8[a + 24]")
raw("add256(0, r5) -> r15")
raw("copyb(0, BI_CARRYNZ) -> r5"); raw("copyb(r5, r15) -> 8[a + 0]")
raw("copyb(0, BI_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_mll_ij)")
w("bi_mll_ijlast:")
# j = lenb-1 ; k = i+lenb-1 ; arith(a[i], b[lenb-1], out[k]) -> out[k]=dl, dh=BI_DH
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r17"); raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("sub(r6, 1) -> r6"); raw("add(r17, r6) -> r17"); raw("sll(r17, 5) -> r16")   # k*32
raw("copyb(0, BI_HDR) -> r5")
raw("copyb(0, BI_APTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("copyb(0, BI_I) -> r7"); raw("copyb(r7, 8[a + 0]) -> r7"); raw("sll(r7, 5) -> r7"); raw("add(r6, r7) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")   # &a[i]
raw("copyb(0, BI_BPTR) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("copyb(0, BI_LENB) -> r7"); raw("copyb(r7, 8[a + 0]) -> r7"); raw("sub(r7, 1) -> r7"); raw("sll(r7, 5) -> r7"); raw("add(r6, r7) -> r6"); raw("copyb(r5, r6) -> 8[a + 8]")   # &b[lenb-1]
raw("copyb(0, BI_OPTR) -> r14"); raw("copyb(r14, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")   # &out[k]
raw("copyb(r5, r14) -> 8[a + 16]"); raw("copyb(r5, r14) -> 8[a + 24]")
raw("copyb(0, BI_DH) -> r6"); raw("copyb(r5, r6) -> 8[a + 32]")
raw("arith256(0, r5) -> r13")
# if carry: BI_DH = BI_DH + 1
raw("copyb(0, BI_CARRYNZ) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("eq(r6, 0), j(bi_mll_nocarry)")
raw("copyb(0, BI_HDR) -> r5"); raw("copyb(0, BI_DH) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("copyb(0, BI_ZERO) -> r7"); raw("copyb(r5, r7) -> 8[a + 8]"); raw("copyb(r5, 1) -> 8[a + 16]"); raw("copyb(0, BI_DH) -> r6"); raw("copyb(r5, r6) -> 8[a + 24]")
raw("add256(0, r5) -> r15")
w("bi_mll_nocarry:")
# out[i+lenb] = BI_DH
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r17"); raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r17, r6) -> r17"); raw("sll(r17, 5) -> r16")
raw("copyb(0, BI_OPTR) -> r14"); raw("copyb(r14, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")
raw("copyb(0, BI_DH) -> r10"); raw("copyb(0, r14) -> r11"); raw("copyb(0, 4) -> r12")
# inline 4-word copy (avoid call, keep regs)
raw("copyb(r10, 8[a + 0]) -> r5"); raw("copyb(r11, r5) -> 8[a + 0]")
raw("copyb(r10, 8[a + 8]) -> r5"); raw("copyb(r11, r5) -> 8[a + 8]")
raw("copyb(r10, 8[a + 16]) -> r5"); raw("copyb(r11, r5) -> 8[a + 16]")
raw("copyb(r10, 8[a + 24]) -> r5"); raw("copyb(r11, r5) -> 8[a + 24]")
raw("copyb(0, BI_I) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_mll_i)")
w("bi_mll_id:")
# len = (out[lena+lenb-1]==0) ? lena+lenb-1 : lena+lenb
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("add(r6, r7) -> r6")   # lena+lenb
raw("sub(r6, 1) -> r7"); raw("sll(r7, 5) -> r16"); raw("copyb(0, BI_OPTR) -> r5"); raw("copyb(r5, 8[a + 0]) -> r14"); raw("add(r14, r16) -> r14")
# check out[last] (4 words) == 0
raw("copyb(r14, 8[a + 0]) -> r5"); raw("copyb(r14, 8[a + 8]) -> r7"); raw("or(r5, r7) -> r5"); raw("copyb(r14, 8[a + 16]) -> r7"); raw("or(r5, r7) -> r5"); raw("copyb(r14, 8[a + 24]) -> r7"); raw("or(r5, r7) -> r5")
raw("copyb(0, r6) -> r10")   # default lena+lenb
raw("ltu(0, r5), j(bi_mll_ret)")   # nonzero top -> full length
raw("sub(r6, 1) -> r10")            # zero top -> lena+lenb-1
w("bi_mll_ret:")
raw("ret"); w("")

setf("common")
# ---------- bi_lt / bi_eq / bi_iszero ----------
w("; u64 zisklib_bi_lt(const u64* a, u64 lena, const u64* b, u64 lenb) -> r10 (1 if a<b)")
w("zisklib_bi_lt:")
raw("push r1")
raw("ltu(r11, r13), j(bi_lt_less)")
raw("ltu(r13, r11), j(bi_lt_notless)")
raw("copyb(0, r12) -> r14"); raw("sll(r11, 2) -> r12"); raw("copyb(0, r14) -> r11"); raw("call bn254_ltn")
raw("pop r1"); raw("ret")
w("bi_lt_less:")
raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret")
w("bi_lt_notless:")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret"); w("")

w("; u64 zisklib_bi_eq(const u64* a, u64 lena, const u64* b, u64 lenb) -> r10 (1 if a==b)")
w("zisklib_bi_eq:")
raw("push r1")
raw("eq(r11, r13), j(bi_eq_same)")
raw("copyb(0, 0) -> r10"); raw("pop r1"); raw("ret")
w("bi_eq_same:")
raw("copyb(0, r12) -> r14"); raw("sll(r11, 2) -> r12"); raw("copyb(0, r14) -> r11"); raw("call bn254_eqn")
raw("pop r1"); raw("ret"); w("")

w("; u64 zisklib_bi_iszero(const u64* a, u64 nwords) -> r10 (1 if all zero)")
w("zisklib_bi_iszero:")
raw("copyb(0, 0) -> r6")   # acc
raw("copyb(0, 0) -> r7")   # i
w("bi_iz_loop:")
raw("eq(r7, r11), j(bi_iz_done)"); raw("copyb(r10, 8[a + 0]) -> r5"); raw("or(r6, r5) -> r6"); raw("add(r10, 8) -> r10"); raw("add(r7, 1) -> r7"); raw("jump(bi_iz_loop)")
w("bi_iz_done:")
raw("copyb(0, 1) -> r10"); raw("eq(r6, 0), j(bi_iz_ret)"); raw("copyb(0, 0) -> r10")
w("bi_iz_ret:")
raw("ret"); w("")

setf("rem_long")
# ---------- rem_long(a r10, lena r11, b r12, lenb r13, res r14) -> r10 = reslen ----------
w("; u64 zisklib_rem_long(const u64* a, u64 lena, const u64* b, u64 lenb, u64* res) -> r10 reslen")
w(";   res = a mod b (multi-U256 divisor). hint (q,r) then verify a == q*b + r, r < b.")
w("zisklib_rem_long:")
raw("push r1")
save("BI_RL_A","r10"); save("BI_RL_LENA","r11"); save("BI_RL_B","r12"); save("BI_RL_LENB","r13"); save("BI_RL_RES","r14")
# if a < b: res = a (lena), return lena
raw("call zisklib_bi_lt")
raw("eq(r10, 0), j(bi_rl_notlt)")
raw("copyb(0, BI_RL_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_RL_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_RL_LENA) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("sll(r12, 2) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BI_RL_LENA) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("pop r1"); raw("ret")
w("bi_rl_notlt:")
# if a == b: res = 0, return 1
raw("copyb(0, BI_RL_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_RL_LENA) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_RL_B) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_RL_LENB) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("call zisklib_bi_eq")
raw("eq(r10, 0), j(bi_rl_general)")
raw("copyb(0, BI_RL_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(r11, 0) -> 8[a + 0]"); raw("copyb(r11, 0) -> 8[a + 8]"); raw("copyb(r11, 0) -> 8[a + 16]"); raw("copyb(r11, 0) -> 8[a + 24]")
raw("copyb(0, 1) -> r10"); raw("pop r1"); raw("ret")
w("bi_rl_general:")
# bigint_div(a, lena*4, b, lenb*4, BI_QUO, BI_REM)
raw("copyb(0, BI_RL_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10")
raw("copyb(0, BI_RL_LENA) -> r5"); raw("copyb(r5, 8[a + 0]) -> r11"); raw("sll(r11, 2) -> r11")
raw("copyb(0, BI_RL_B) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12")
raw("copyb(0, BI_RL_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r13"); raw("sll(r13, 2) -> r13")
raw("copyb(0, BI_QUO) -> r14"); raw("copyb(0, BI_REM) -> r15")
raw("call zisklib_bigint_div")   # r10 lenquo_u64, r11 lenrem_u64
raw("srl(r10, 2) -> r10"); raw("copyb(0, BI_RL_QLEN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("srl(r11, 2) -> r11"); raw("copyb(0, BI_RL_RLEN) -> r5"); raw("copyb(r5, r11) -> 8[a + 0]")
# q_b = mul_long(BI_QUO, qlen, b, lenb, BI_QB)
raw("copyb(0, BI_QUO) -> r10"); raw("copyb(0, BI_RL_QLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_RL_B) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_RL_LENB) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("copyb(0, BI_QB) -> r14"); raw("call zisklib_mul_long")
raw("copyb(0, BI_RL_QBLEN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
# rem == 0 ?
raw("copyb(0, BI_REM) -> r10"); raw("copyb(0, BI_RL_RLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("sll(r11, 2) -> r11"); raw("call zisklib_bi_iszero")
raw("ltu(0, r10), j(bi_rl_remzero)")
# rem != 0: assert rem < b ; q_b_r = add_agtb(q_b, qblen, rem, rlen, BI_QBR) ; assert a == q_b_r
raw("copyb(0, BI_REM) -> r10"); raw("copyb(0, BI_RL_RLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_RL_B) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_RL_LENB) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("call zisklib_bi_lt"); raw("eq(r10, 0), j(bi_rl_bad)")
raw("copyb(0, BI_QB) -> r10"); raw("copyb(0, BI_RL_QBLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_REM) -> r12"); raw("copyb(0, BI_RL_RLEN) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("copyb(0, BI_QBR) -> r14"); raw("call zisklib_add_agtb")
# assert bi_eq(a, lena, BI_QBR, r10)
raw("copyb(0, BI_QBR) -> r12"); raw("copyb(0, r10) -> r13"); raw("copyb(0, BI_RL_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_RL_LENA) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("call zisklib_bi_eq"); raw("eq(r10, 0), j(bi_rl_bad)")
raw("jump(bi_rl_setres)")
w("bi_rl_remzero:")
# assert bi_eq(a, lena, BI_QB, qblen)
raw("copyb(0, BI_QB) -> r12"); raw("copyb(0, BI_RL_QBLEN) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("copyb(0, BI_RL_A) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_RL_LENA) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("call zisklib_bi_eq"); raw("eq(r10, 0), j(bi_rl_bad)")
w("bi_rl_setres:")
raw("copyb(0, BI_REM) -> r10"); raw("copyb(0, BI_RL_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_RL_RLEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("sll(r12, 2) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BI_RL_RLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10"); raw("pop r1"); raw("ret")
w("bi_rl_bad:")
raw("copyb(0, 0) -> r10, end"); w("")

setf("mul_long")
# ---------- mul_and_reduce_long(a r10, lena r11, b r12, lenb r13, mod r14, lenm r15, out r16) -> r10 outlen ----------
w("; u64 zisklib_mul_and_reduce_long(a, lena, b, lenb, mod, lenm, out) -> r10 outlen  (out = a*b mod mod)")
w("zisklib_mul_and_reduce_long:")
raw("push r1")
save("BI_MR_MOD","r14"); save("BI_MR_LENM","r15"); save("BI_MR_OUT","r16")
# mul_long(a, lena, b, lenb, BI_MUL) -> mullen
raw("copyb(0, BI_MUL) -> r14"); raw("call zisklib_mul_long")   # r10 = mullen
# rem_long(BI_MUL, mullen, mod, lenm, out)
raw("copyb(0, r10) -> r11"); raw("copyb(0, BI_MUL) -> r10")
raw("copyb(0, BI_MR_MOD) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_MR_LENM) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13")
raw("copyb(0, BI_MR_OUT) -> r14"); raw("copyb(r14, 8[a + 0]) -> r14"); raw("call zisklib_rem_long")
raw("pop r1"); raw("ret"); w("")

setf("modexp")
# ---------- modexp_long(base r10, lenb r11, exp r12, lene r13, mod r14, lenm r15, res r16) -> r10 reslen ----------
w("; u64 zisklib_modexp_long(base, lenb, exp, lene, mod, lenm, res) -> r10 reslen")
w("zisklib_modexp_long:")
raw("push r1")
save("BI_ML2_BASE","r10"); save("BI_ML2_LENB","r11"); save("BI_ML2_EXP","r12"); save("BI_ML2_LENE","r13"); save("BI_ML2_MOD","r14"); save("BI_ML2_LENM","r15"); save("BI_ML2_RES","r16")
# base_r = rem_long(base, lenb, mod, lenm, BI_BASE) -> baselen
raw("copyb(0, BI_ML2_BASE) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_ML2_LENB) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_ML2_MOD) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_ML2_LENM) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("copyb(0, BI_BASE) -> r14"); raw("call zisklib_rem_long")
raw("copyb(0, BI_ML2_BASELEN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
# bin_decomp(exp, lene)
raw("copyb(0, BI_ML2_EXP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_ML2_LENE) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("call zisklib_bin_decomp")
# out = base_r ; outlen = baselen
raw("copyb(0, BI_BASE) -> r10"); raw("copyb(0, BI_OUT) -> r11"); raw("copyb(0, BI_ML2_BASELEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("sll(r12, 2) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BI_ML2_OUTLEN) -> r5"); raw("copyb(0, BI_ML2_BASELEN) -> r6"); raw("copyb(r6, 8[a + 0]) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]")
# loop bit_idx 1..nbits
raw("copyb(0, BI_ML2_J) -> r5"); raw("copyb(r5, 1) -> 8[a + 0]")
w("bi_mel_loop:")
raw("copyb(0, BI_ML2_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, BI_NBITS) -> r5"); raw("copyb(r5, 8[a + 0]) -> r7"); raw("eq(r6, r7), j(bi_mel_done)")
# if out==0 (outlen==1 && out[0..4]==0) break
raw("copyb(0, BI_ML2_OUTLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, 1) -> r7"); raw("eq(r6, r7), j(bi_mel_chkz)"); raw("jump(bi_mel_sq)")
w("bi_mel_chkz:")
raw("copyb(0, BI_OUT) -> r10"); raw("copyb(0, BI_ZERO) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_eqn"); raw("ltu(0, r10), j(bi_mel_done)")
w("bi_mel_sq:")
# out = out^2 mod mod : mul_and_reduce(out, outlen, out, outlen, mod, lenm, BI_OUT2)
raw("copyb(0, BI_OUT) -> r10"); raw("copyb(0, BI_ML2_OUTLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_OUT) -> r12"); raw("copyb(0, BI_ML2_OUTLEN) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("copyb(0, BI_ML2_MOD) -> r14"); raw("copyb(r14, 8[a + 0]) -> r14"); raw("copyb(0, BI_ML2_LENM) -> r15"); raw("copyb(r15, 8[a + 0]) -> r15"); raw("copyb(0, BI_OUT2) -> r16"); raw("call zisklib_mul_and_reduce_long")
# outlen = r10 ; out = BI_OUT2
raw("copyb(0, BI_ML2_OUTLEN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BI_OUT2) -> r10"); raw("copyb(0, BI_OUT) -> r11"); raw("sll(r6, 0) -> r6") if False else None
raw("copyb(0, BI_ML2_OUTLEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("sll(r12, 2) -> r12"); raw("call bn254_memcpy")
# bit = BITS[bit_idx]
raw("copyb(0, BI_ML2_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("srl(r6, 6) -> r16"); raw("sll(r16, 3) -> r16"); raw("copyb(0, BI_BITS) -> r5"); raw("add(r5, r16) -> r5"); raw("copyb(r5, 8[a + 0]) -> r5"); raw("and(r6, 63) -> r16"); raw("srl(r5, r16) -> r5"); raw("and(r5, 1) -> r17")
raw("eq(r17, 0), j(bi_mel_next)")
# out = out * base_r mod mod
raw("copyb(0, BI_OUT) -> r10"); raw("copyb(0, BI_ML2_OUTLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_BASE) -> r12"); raw("copyb(0, BI_ML2_BASELEN) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("copyb(0, BI_ML2_MOD) -> r14"); raw("copyb(r14, 8[a + 0]) -> r14"); raw("copyb(0, BI_ML2_LENM) -> r15"); raw("copyb(r15, 8[a + 0]) -> r15"); raw("copyb(0, BI_OUT2) -> r16"); raw("call zisklib_mul_and_reduce_long")
raw("copyb(0, BI_ML2_OUTLEN) -> r5"); raw("copyb(r5, r10) -> 8[a + 0]")
raw("copyb(0, BI_OUT2) -> r10"); raw("copyb(0, BI_OUT) -> r11"); raw("copyb(0, BI_ML2_OUTLEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("sll(r12, 2) -> r12"); raw("call bn254_memcpy")
w("bi_mel_next:")
raw("copyb(0, BI_ML2_J) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 1) -> r6"); raw("copyb(r5, r6) -> 8[a + 0]"); raw("jump(bi_mel_loop)")
w("bi_mel_done:")
raw("copyb(0, BI_OUT) -> r10"); raw("copyb(0, BI_ML2_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_ML2_OUTLEN) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("sll(r12, 2) -> r12"); raw("call bn254_memcpy")
raw("copyb(0, BI_ML2_OUTLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r10")
raw("pop r1"); raw("ret"); w("")

setf("common")
# ================= DISPATCHER (modexp_u64_c) =================
for nm in ["BI_DP_EXP","BI_DP_ELEN","BI_DP_RES","BI_DP_BLEN","BI_DP_MLEN"]:
    w(f"u64 {nm}[1] = 0")
w(f"u64 BI_BASEPAD[{MAXW+4}] = "+", ".join(["0"]*(MAXW+4)))
w(f"u64 BI_MODPAD[{MAXW+4}] = "+", ".join(["0"]*(MAXW+4)))
w("")

def zeropad(src_ptr_slot, len_u64_slot, dst, dst_len_u256_slot):
    # dst[0..dst_len_u256*4] = 0 ; dst[0..len_u64] = src[0..len_u64]
    raw(f"copyb(0, {dst_len_u256_slot}) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("sll(r6, 5) -> r6")   # dst bytes
    raw(f"copyb(0, {dst}) -> r7")
    raw("dma_xmemset(r7, r6) -> r5, j(0, 4)")   # zero the whole dst in one op
    raw(f"copyb(0, {src_ptr_slot}) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw(f"copyb(0, {dst}) -> r11"); raw(f"copyb(0, {len_u64_slot}) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("call bn254_memcpy")

setf("modexp")
w("; u64 zisklib_modexp_u64_c(base, blen, exp, elen, mod, mlen, result) -> r10 = result_len (u64)")
w(";   base/mod lengths in u64; padded to U256. result written LE u64; returns count.")
w("zisklib_modexp_u64_c:")
raw("push r1")
# save raw args
save("BI_APTR","r10"); save("BI_LEN","r11"); save("BI_DP_EXP","r12"); save("BI_DP_ELEN","r13"); save("BI_BPTR","r14"); save("BI_LENB","r15"); save("BI_DP_RES","r16")
# blen_u256 = (blen+3)>>2 ; mlen_u256 = (mlen+3)>>2
raw("copyb(0, BI_LEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 3) -> r6"); raw("srl(r6, 2) -> r6"); raw("copyb(0, BI_DP_BLEN) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")
raw("copyb(0, BI_LENB) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("add(r6, 3) -> r6"); raw("srl(r6, 2) -> r6"); raw("copyb(0, BI_DP_MLEN) -> r5"); raw("copyb(r5, r6) -> 8[a + 0]")
# pad base -> BI_BASEPAD (blen_u256*4) ; mod -> BI_MODPAD
zeropad("BI_APTR","BI_LEN","BI_BASEPAD","BI_DP_BLEN")
zeropad("BI_BPTR","BI_LENB","BI_MODPAD","BI_DP_MLEN")
# ---- edge cases ----
# mod == 0 or mod == 1 (mlen_u256==1)
raw("copyb(0, BI_DP_MLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, 1) -> r7"); raw("eq(r6, r7), j(bi_dp_m1)"); raw("jump(bi_dp_expchk)")
w("bi_dp_m1:")
raw("copyb(0, BI_MODPAD) -> r10"); raw("copyb(0, BI_ZERO) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_eqn"); raw("ltu(0, r10), j(bi_dp_ret0)")   # mod==0 -> [0]
raw("copyb(0, BI_MODPAD) -> r10"); raw("copyb(0, BI_ONE) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_eqn"); raw("ltu(0, r10), j(bi_dp_ret0)")   # mod==1 -> [0]
w("bi_dp_expchk:")
# exp == 0 ? (iszero over elen u64s)
raw("copyb(0, BI_DP_EXP) -> r10"); raw("copyb(r10, 8[a + 0]) -> r10"); raw("copyb(0, BI_DP_ELEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("call zisklib_bi_iszero"); raw("ltu(0, r10), j(bi_dp_ret1)")   # exp==0 -> [1]
# base == 0 or 1 (blen_u256==1)
raw("copyb(0, BI_DP_BLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, 1) -> r7"); raw("eq(r6, r7), j(bi_dp_b1)"); raw("jump(bi_dp_dispatch)")
w("bi_dp_b1:")
raw("copyb(0, BI_BASEPAD) -> r10"); raw("copyb(0, BI_ZERO) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_eqn"); raw("ltu(0, r10), j(bi_dp_ret0)")   # base==0 -> [0]
raw("copyb(0, BI_BASEPAD) -> r10"); raw("copyb(0, BI_ONE) -> r11"); raw("copyb(0, 4) -> r12"); raw("call bn254_eqn"); raw("ltu(0, r10), j(bi_dp_ret1)")   # base==1 -> [1]
w("bi_dp_dispatch:")
# mlen_u256 == 1 ? short : long
raw("copyb(0, BI_DP_MLEN) -> r5"); raw("copyb(r5, 8[a + 0]) -> r6"); raw("copyb(0, 1) -> r7"); raw("eq(r6, r7), j(bi_dp_short)")
# long: modexp_long(BASEPAD, blen, exp, elen, MODPAD, mlen, result) -> reslen
raw("copyb(0, BI_BASEPAD) -> r10"); raw("copyb(0, BI_DP_BLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_DP_EXP) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_DP_ELEN) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("copyb(0, BI_MODPAD) -> r14"); raw("copyb(0, BI_DP_MLEN) -> r15"); raw("copyb(r15, 8[a + 0]) -> r15"); raw("copyb(0, BI_DP_RES) -> r16"); raw("copyb(r16, 8[a + 0]) -> r16"); raw("call zisklib_modexp_long")
raw("sll(r10, 2) -> r10"); raw("pop r1"); raw("ret")
w("bi_dp_short:")
# short: modexp_short(BASEPAD, blen, exp, elen, MODPAD, result) -> result is 1 U256
raw("copyb(0, BI_BASEPAD) -> r10"); raw("copyb(0, BI_DP_BLEN) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(0, BI_DP_EXP) -> r12"); raw("copyb(r12, 8[a + 0]) -> r12"); raw("copyb(0, BI_DP_ELEN) -> r13"); raw("copyb(r13, 8[a + 0]) -> r13"); raw("copyb(0, BI_MODPAD) -> r14"); raw("copyb(0, BI_DP_RES) -> r15"); raw("copyb(r15, 8[a + 0]) -> r15"); raw("call zisklib_modexp_short")
raw("copyb(0, 4) -> r10"); raw("pop r1"); raw("ret")
w("bi_dp_ret0:")
raw("copyb(0, BI_DP_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(r11, 0) -> 8[a + 0]"); raw("copyb(r11, 0) -> 8[a + 8]"); raw("copyb(r11, 0) -> 8[a + 16]"); raw("copyb(r11, 0) -> 8[a + 24]")
raw("copyb(0, 4) -> r10"); raw("pop r1"); raw("ret")
w("bi_dp_ret1:")
raw("copyb(0, BI_DP_RES) -> r11"); raw("copyb(r11, 8[a + 0]) -> r11"); raw("copyb(r11, 1) -> 8[a + 0]"); raw("copyb(r11, 0) -> 8[a + 8]"); raw("copyb(r11, 0) -> 8[a + 16]"); raw("copyb(r11, 0) -> 8[a + 24]")
raw("copyb(0, 4) -> r10"); raw("pop r1"); raw("ret"); w("")

import os
BASE=_OUT+"/bigint"
os.makedirs(BASE, exist_ok=True)
ORDER=["common","add_short","add_agtb","mul_short","mul_long","rem_short","rem_long","modexp"]
for name in ORDER:
    body=FILES.get(name, [])
    hdr=[
      "; ============================================================================",
      f"; bigint/{name}.zisk - {DESC[name]}",
      "; GENERATED by scripts/legacy/gen_bigint.py. Radix 2^256 (digit = U256 = 4 u64);",
      "; arbitrary-precision arithmetic for modexp (EIP-198). Prefix bi_.",
      "; ============================================================================",
      "",
    ]
    open(os.path.join(BASE, name+".zisk"),"w").write("\n".join(hdr+body)+"\n")
    print(f"wrote bigint/{name}.zisk ({len(body)} lines)")
# remove the old single-file version if present
old=_OUT+"/bigint.zisk"
if os.path.exists(old): os.remove(old); print("removed old bigint.zisk")
