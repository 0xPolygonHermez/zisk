"""Montgomery path of zkvm/modexp.zisk (odd moduli of 49..KMAX * 32 significant bytes),
imported by modexp_gen.py.

Returns the .zisk text. Digits are 256-bit (4 LE limbs, 32 bytes). Every k-digit
number is stored right-aligned in a KMAX-digit buffer: digit j at position
idx0 + j, idx0 = KMAX - k. The CIOS row passes are unrolled over all KMAX
positions with preset precompile parameter blocks, and entered at position idx0
with a computed jump, so a row costs 2 steps per digit whatever k is.
"""

KMAX = 33
KS = 16     # the dedicated squaring covers k <= KS (its pair tables grow as KS^2)
PB = 2 * (KMAX - KS)   # the lowest square position a k <= KS operand reaches


def gen():
    o = []
    w = o.append
    D = 32  # digit bytes

    def z(n):
        return f"u64 {n}"

    # ---------------- data ----------------
    w(f"""; ---------------------------------------------------------------------------
; Montgomery path: odd moduli of 49..{KMAX * 32} significant bytes (k = 2..{KMAX} digits
; of 256 bits, R = 2^(256k)). X = X * Y * R^-1 mod M is CIOS: per digit y_i of Y,
; t += X * y_i, then t = (t + m * M) / 2^256 with m = t_0 * (-M^-1) mod 2^256.
; Each digit step is one arith256 (lo, hi = x_j * y_i + t_j) and one add256
; (t_j = lo + hi_(j-1) + carry), whose carry is stored straight into the next
; add256's cin. The blocks for every position are preset; buffers are
; right-aligned (digit j at position KMAX - k + j), and each chain is entered at
; that first position with a computed jump. A final t - M (add256 chain with ~M)
; picks t or t - M. Setup, once per call: -M^-1 (Newton), base mod M and
; R^2 mod M (zisklib_rem_long, hint-verified; from 32 digits on, 2^k * R mod M
; squared 8 times), and one multiplication moves the base to Montgomery form. The
; exponent is walked as in the direct paths; a final multiplication by 1 leaves
; Montgomery form.
; ---------------------------------------------------------------------------
u64 ZMM_X[{KMAX * 4}] = 0             ; the accumulator
u64 ZMM_M[{KMAX * 4}] = 0             ; the modulus
u64 ZMM_NM[{KMAX * 4}] = 0            ; ~M (the digits in use)
u64 ZMM_S[{KMAX * 4}] = 0             ; t - M
u64 ZMM_BR[{KMAX * 4}] = 0            ; base mod M
u64 ZMM_R2[{KMAX * 4}] = 0            ; R^2 mod M, later 1
u64 ZMM_TT[{15 * KMAX * 4}] = 0      ; T[v] = B^v (Montgomery) at ZMM_TT + (v - 1) * {KMAX * D}
u64 ZMM_T[{(KMAX + 3) * 4}] = 0       ; t, positions -1..{KMAX + 1} (slot = position + 1)
u64 ZMM_H1[{(KMAX + 1) * 4}] = 0      ; row 1 high halves, positions -1..{KMAX - 1}
u64 ZMM_H2[{(KMAX + 1) * 4}] = 0      ; row 2 high halves
u64 ZMM_BI[4] = 0                     ; the current digit y_i
u64 ZMM_L[4] = 0                      ; a low half
u64 ZMM_JUNK[4] = 0                   ; a discarded high half
u64 ZMM_MM[4] = 0                     ; m
u64 ZMM_NP[4] = 0                     ; -M^-1 mod 2^256
u64 ZMM_M0[4] = 0                     ; M's lowest digit
u64 ZMM_INV[4] = 0
u64 ZMM_Y[4] = 0
u64 ZMM_NY[4] = 0
u64 ZMM_Z[4] = 0
const u64 ZMM_THREE[4] = 3, 0, 0, 0
u64 ZMM_BUFL[{(KMAX + 1) * 4}] = 0    ; a big-endian operand, right-aligned
u64 ZMM_SCARRY[1] = 0
u64 ZMM_RA2[{(2 * KMAX + 1) * 4}] = 0  ; R^2 (or 2^k * R), up to {2 * KMAX + 1} digits
; per-call values (the hot ones live in r32..r39 instead: r32 = &t[idx0 - 1],
; r33 = (k + 3) * 32, r34 = k, r35/r36/r37 = the row 1 / row 2 / t - M entry pcs,
; r38 = &X[idx0], r39 = k * 32; no routine this path calls uses r32..r39)
u64 ZMM_I32[1] = 0                    ; idx0 * 32
u64 ZMM_TP[1] = 0                     ; &t[idx0]
u64 ZMM_SP[1] = 0                     ; &S[idx0]
u64 ZMM_TVM[1] = 0                    ; &T[v][idx0] - v * {KMAX * D}
u64 ZMM_A_BASE[1] = 0
u64 ZMM_A_BLEN[1] = 0
u64 ZMM_A_EXP[1] = 0
u64 ZMM_A_ELEN[1] = 0
u64 ZMM_A_MLEN[1] = 0
u64 ZMM_A_OUT[1] = 0
""")

    def dig(buf, p):  # address of digit position p in a KMAX buffer
        return f"{buf} + {p * D}"

    def ts(p):  # t position p (-1..KMAX+1)
        return f"ZMM_T + {(p + 1) * D}"

    def h(buf, p):  # H position p (-1..KMAX-1)
        return f"{buf} + {(p + 1) * D}"

    for p in range(KMAX):
        w(f"u64 ZMM_A1_{p}[5] = {dig('ZMM_X', p)}, ZMM_BI, {ts(p)}, ZMM_L, {h('ZMM_H1', p)}")
    for p in range(KMAX):
        w(f"u64 ZMM_A2_{p}[5] = ZMM_MM, {dig('ZMM_M', p)}, {ts(p)}, ZMM_L, {h('ZMM_H2', p)}")
    # D blocks as one contiguous array each (32 bytes per block), so position
    # idx0's cin is at a computed address.
    d1 = []
    for p in range(KMAX):
        d1 += ["ZMM_L", h('ZMM_H1', p - 1), "0", ts(p)]
    d1 += [ts(KMAX), h('ZMM_H1', KMAX - 1), "0", ts(KMAX)]
    w(f"u64 ZMM_D1[{len(d1)}] = " + ", ".join(d1))
    d2 = []
    for p in range(KMAX):
        d2 += ["ZMM_L", h('ZMM_H2', p - 1), "0", ts(p - 1)]
    d2 += [ts(KMAX), h('ZMM_H2', KMAX - 1), "0", ts(KMAX - 1)]
    w(f"u64 ZMM_D2[{len(d2)}] = " + ", ".join(d2))
    w(f"u64 ZMM_D3[4] = {ts(KMAX + 1)}, ZME_ZERO, 0, {ts(KMAX)}")
    ds = []
    for p in range(KMAX):
        ds += [ts(p), dig('ZMM_NM', p), "0", dig('ZMM_S', p)]
    w(f"u64 ZMM_DS[{len(ds)}] = " + ", ".join(ds))
    w("u64 ZMM_HM[5]  = ZMM_T, ZMM_NP, ZME_ZERO, ZMM_MM, ZMM_JUNK   ; a = &t[idx0], set per call")
    w("u64 ZMM_HN1[5] = ZMM_M0, ZMM_INV, ZME_ZERO, ZMM_Y, ZMM_JUNK  ; y = M0 * inv")
    w("u64 ZMM_HN2[4] = ZMM_NY, ZMM_THREE, 0, ZMM_Z               ; z = ~y + 3 = 2 - y")
    w("u64 ZMM_HN3[5] = ZMM_INV, ZMM_Z, ZME_ZERO, ZMM_INV, ZMM_JUNK ; inv = inv * z")
    w("u64 ZMM_HN4[4] = ZMM_NY, ZME_ZERO, 1, ZMM_NP                ; np = ~inv + 1")
    w("")

    # ---------------- helpers ----------------
    w(f"""; zmm_ldl: [r5 = src, r6 = nbytes, r7 = dst limbs, r28 = digits] the nbytes-byte
;   big-endian number at src as 4 * r28 little-endian limbs at dst.
;   Clobbers r7, r29, r30, r31.
zmm_ldl:
	sll(r28, 5) -> r29
	dma_xmemset(ZMM_BUFL, r29) -> r0, j(0, 4)
	add(r29, ZMM_BUFL) -> r30
	sub(r30, r6) -> r30
	copyb(0, r6) -> [ZME_DMA_COUNT]
	dma_memcpy(r30, r5) -> r0
	add(r29, ZMM_BUFL - 8) -> r30      ; the last big-endian word
	srl(r29, 3) -> r31
zmm_ldl_w:
	rev8(r30, 8[a + 0])
	copyb(r7, c) -> 8[a + 0]
	sub(r30, 8) -> r30
	add(r7, 8) -> r7
	sub(r31, 1) -> r31
	ltu(0, r31), j(zmm_ldl_w)
	ret

; zmm_mul: [r10 = &Y[idx0]] X = X * Y * R^-1 mod M (Y may be X).
;   Clobbers r10, r11. Uses the per-call registers r32..r39 (see zmm).
; zmm_sqr: [r16 = 1 for 3 <= k <= KS] X = X^2 / R mod M: zms_go, or zmm_mul
;   with Y = X (falls through).
zmm_sqr:
	ltu(0, r16), j(zms_go)
	copyb(0, r38) -> r10
zmm_mul:
	dma_xmemset(r32, r33) -> r0, j(0, 4)
	copyb(0, r34) -> r11
zmm_row:
	dma_xmemcpy(ZMM_BI, r10) -> r0, j(32, 4)
	copyb(0, r35), setpc(0)
zmm_p1:""")
    for p in range(KMAX):
        w(f"\tarith256(0, ZMM_A1_{p})")
        w(f"\tadd256(0, ZMM_D1 + {p * 32}) -> [ZMM_D1 + {(p + 1) * 32 + 16}]")
    w(f"\tadd256(0, ZMM_D1 + {KMAX * 32}) -> [{ts(KMAX + 1)}]")
    w("\tarith256(0, ZMM_HM)                 ; m = t_0 * -M^-1")
    w("\tcopyb(0, r36), setpc(0)")
    w("zmm_p2:")
    for p in range(KMAX):
        w(f"\tarith256(0, ZMM_A2_{p})")
        w(f"\tadd256(0, ZMM_D2 + {p * 32}) -> [ZMM_D2 + {(p + 1) * 32 + 16}]")
    w(f"\tadd256(0, ZMM_D2 + {KMAX * 32}) -> [ZMM_D3 + 16]")
    w("\tadd256(0, ZMM_D3)")
    w(f"""	add(r10, 32) -> r10
	sub(r11, 1) -> r11
	ltu(0, r11), j(zmm_row)
	; --- t (< 2M) or t - M ---
zmm_sub:
	copyb(0, r37), setpc(0)
zmm_ps:""")
    for p in range(KMAX):
        nxt = f"[ZMM_DS + {(p + 1) * 32 + 16}]" if p < KMAX - 1 else "[ZMM_SCARRY]"
        w(f"\tadd256(0, ZMM_DS + {p * 32}) -> {nxt}")
    w(f"""	copyb(0, r39) -> [ZME_DMA_COUNT]
	copyb(0, [{ts(KMAX)}])            ; t's top digit (0 or 1)
	or(c, [ZMM_SCARRY])
	eq(c, 0), j(zmm_keep)
	dma_memcpy(r38, [ZMM_SP]) -> r0    ; t >= M: X = t - M
	ret
zmm_keep:
	dma_memcpy(r38, [ZMM_TP]) -> r0    ; X = t
	ret
""")

    # ---------------- squaring ----------------
    o.extend(gen_sqr())

    # ---------------- entry ----------------
    w(f"""; --- the Montgomery path: r17 = the modulus' first nonzero byte, r28 = its length ---
zmm:
	add(r14, r15) -> r5
	copyb(r5, 1[a - 1]) -> r5          ; the modulus' last byte
	and(r5, 1) -> r5
	eq(r5, 0), j(zme_legacy)            ; even modulus: the general path
	ltu({KMAX * 32}, r28), j(zme_legacy)       ; over {KMAX} digits: the general path
	copyb(0, r10) -> [ZMM_A_BASE]
	copyb(0, r11) -> [ZMM_A_BLEN]
	copyb(0, r12) -> [ZMM_A_EXP]
	copyb(0, r13) -> [ZMM_A_ELEN]
	copyb(0, r15) -> [ZMM_A_MLEN]
	copyb(0, r16) -> [ZMM_A_OUT]
	; --- per-call values: k, idx0 = {KMAX} - k, the addresses and entries of position idx0 ---
	add(r28, 31) -> r5
	srl(r5, 5) -> r5
	copyb(0, r5) -> r34
	sll(r5, 5) -> r39
	add(r5, 3) -> r6
	sll(r6, 5) -> r33
	sub({KMAX}, r5) -> r6                ; idx0
	sll(r6, 3) -> r7
	add(r7, zmm_p1) -> r35         ; 2 instructions (8 bytes) per position
	add(r7, zmm_p2) -> r36
	sll(r6, 2) -> r7
	add(r7, zmm_ps) -> r37         ; 1 instruction per position
	sll(r6, 5) -> r6                    ; idx0 * 32
	copyb(0, r6) -> [ZMM_I32]
	add(r6, ZMM_X) -> r38
	add(r6, ZMM_T + 32) -> [ZMM_TP]
	add(r6, ZMM_T + 32) -> [ZMM_HM]
	add(r6, ZMM_T) -> r32
	add(r6, ZMM_S) -> [ZMM_SP]
	add(r6, ZMM_TT - {KMAX * D}) -> [ZMM_TVM]
	; the chains start at idx0: no carry in (1 for t - M), no high half below
	add(r6, ZMM_D1) -> r7
	copyb(r7, 0) -> 8[a + 16]
	add(r6, ZMM_D2) -> r7
	copyb(r7, 0) -> 8[a + 16]
	add(r6, ZMM_DS) -> r7
	copyb(r7, 1) -> 8[a + 16]
	add(r6, ZMM_H1) -> r7
	dma_xmemset(r7, 32) -> r0, j(0, 4)
	add(r6, ZMM_H2) -> r7
	dma_xmemset(r7, 32) -> r0, j(0, 4)
	; --- the squaring's per-call values (k <= {KS}); r28 = the modulus length, still needed ---
	ltu({KS}, r34), j(zms_noset)
	add(r6, r6) -> r7                   ; 2 * idx0 * 32
	add(r7, ZMS_P - {PB * D}) -> [ZMS_PLO]
	add(r7, ZMS_DB - {PB * D}) -> r29
	copyb(r29, 0) -> 8[a + 16]
	add(r7, ZMS_DA - {PB * D}) -> r29
	copyb(r29, 0) -> 8[a + 16]
	srl(r6, 2) -> r29                   ; 2 * idx0 * 4
	add(r29, zms_db - {PB * 4}) -> [ZMS_EDB]
	add(r29, zms_da - {PB * 4}) -> [ZMS_EDA]
	add(r29, ZMS_ROWS - {(KMAX - KS) * 8}) -> r29
	copyb(r29, 8[a + 0]) -> [ZMS_ER]
	srl(r6, 3) -> r29                   ; idx0 * 4
	add(r29, zms_dg - {(KMAX - KS) * 4}) -> [ZMS_EDG]
	add(r6, ZMS_ADDS - {(KMAX - KS) * D}) -> [ZMS_Q0]
zms_noset:
	; --- M, ~M, M's lowest digit ---
	copyb(0, r17) -> r5
	copyb(0, r28) -> r6
	copyb(0, [ZMM_I32])
	add(c, ZMM_M) -> r7
	copyb(0, r34) -> r28
	call zmm_ldl
	copyb(0, [ZMM_I32]) -> r6
	add(r6, ZMM_M) -> r5
	dma_xmemcpy(ZMM_M0, r5) -> r0, j(32, 4)
	add(r6, ZMM_NM) -> r7
	copyb(0, r34) -> r29
	sll(r29, 2) -> r29                  ; limbs
zmm_nm:
	copyb(r5, 8[a + 0])
	xor(c, 0xffffffffffffffff)
	copyb(r7, c) -> 8[a + 0]
	add(r5, 8) -> r5
	add(r7, 8) -> r7
	sub(r29, 1) -> r29
	ltu(0, r29), j(zmm_nm)
	; --- np = -M^-1 mod 2^256: Newton from inv = M0 (right mod 8), 7 doublings ---
	dma_xmemcpy(ZMM_INV, ZMM_M0) -> r0, j(32, 4)""")
    for _ in range(7):
        w("\tarith256(0, ZMM_HN1)")
        for i in range(4):
            w(f"\txor([ZMM_Y + {8 * i}], 0xffffffffffffffff) -> [ZMM_NY + {8 * i}]")
        w("\tadd256(0, ZMM_HN2)")
        w("\tarith256(0, ZMM_HN3)")
    for i in range(4):
        w(f"\txor([ZMM_INV + {8 * i}], 0xffffffffffffffff) -> [ZMM_NY + {8 * i}]")
    w("\tadd256(0, ZMM_HN4)")
    w(f"""	; --- BR = base mod M (base without leading zero bytes, >= 1 digit) ---
	copyb(0, [ZMM_A_BASE]) -> r5
	copyb(0, [ZMM_A_BLEN]) -> r6
zmm_bz:
	eq(r6, 0), j(zmm_bzd)
	copyb(r5, 1[a + 0])
	ltu(0, c), j(zmm_bzd)
	add(r5, 1) -> r5
	sub(r6, 1) -> r6
	jump(zmm_bz)
zmm_bzd:
	add(r6, 31) -> r28
	srl(r28, 5) -> r28
	ltu(0, r28), j(zmm_bnz)
	copyb(0, 1) -> r28
zmm_bnz:
	copyb(0, r28) -> [ZKVM_ME_BU]       ; the base's digits (the general path's spill slot)
	copyb(0, ZKVM_ME_BASE_LIMBS) -> r7
	call zmm_ldl
	copyb(0, [ZMM_I32]) -> r6
	add(r6, ZMM_BR) -> r7
	copyb(0, r39) -> r29
	dma_xmemset(r7, r29) -> r0, j(0, 4)
	copyb(0, ZKVM_ME_BASE_LIMBS) -> r10
	copyb(0, [ZKVM_ME_BU]) -> r11
	add(r6, ZMM_M) -> r12
	copyb(0, r34) -> r13
	copyb(0, r7) -> r14
	call zisklib_rem_long
	; --- X = R^2 mod M. Up to 31 digits it is one rem_long of R^2 (a 1 in digit
	; 2k); beyond, R^2 and M overflow the division hint's 386 parameter words,
	; so it is 2^k * R mod M squared 8 times (2^k * R -> 2^(256k) * R = R^2) ---
	copyb(0, r34) -> r5
	ltu(31, r5), j(zmm_r2big)
	sll(r5, 1) -> r11
	add(r11, 1) -> r11                  ; 2k + 1 digits
	sll(r11, 5) -> r6
	dma_xmemset(ZMM_RA2, r6) -> r0, j(0, 4)
	sll(r5, 6) -> r6
	add(r6, ZMM_RA2) -> r7
	copyb(r7, 1) -> 8[a + 0]
	copyb(0, r38) -> r14
	copyb(0, r39) -> r29
	dma_xmemset(r14, r29) -> r0, j(0, 4)
	copyb(0, ZMM_RA2) -> r10
	copyb(0, [ZMM_I32]) -> r6
	add(r6, ZMM_M) -> r12
	copyb(0, r5) -> r13
	call zisklib_rem_long
	jump(zmm_r2done)
zmm_r2big:
	add(r5, 1) -> r6
	sll(r6, 5) -> r6
	copyb(0, ZMM_RA2) -> r7
	dma_xmemset(r7, r6) -> r0, j(0, 4)
	sll(r5, 5) -> r6
	add(r6, ZMM_RA2) -> r7
	sll(1, r5) -> r6
	copyb(r7, r6) -> 8[a + 0]
	copyb(0, r38) -> r14
	copyb(0, r39) -> r29
	dma_xmemset(r14, r29) -> r0, j(0, 4)
	copyb(0, ZMM_RA2) -> r10
	add(r5, 1) -> r11
	copyb(0, [ZMM_I32]) -> r6
	add(r6, ZMM_M) -> r12
	copyb(0, r5) -> r13
	call zisklib_rem_long
	; --- 8 squarings: 2^k * R -> 2^(256k) * R = R^2 (mod M) ---
	copyb(0, 8) -> r12
zmm_r2:
	copyb(0, r38) -> r10
	call zmm_mul
	sub(r12, 1) -> r12
	ltu(0, r12), j(zmm_r2)
zmm_r2done:
	; --- R2 = X; X = BR; X = BR * R2 / R = B * R (mod M); T[1] = X ---
	copyb(0, r39) -> [ZME_DMA_COUNT]
	copyb(0, [ZMM_I32]) -> r6
	add(r6, ZMM_R2) -> r5
	dma_memcpy(r5, r38) -> r0
	add(r6, ZMM_BR) -> r7
	dma_memcpy(r38, r7) -> r0
	copyb(0, r5) -> r10
	call zmm_mul
	copyb(0, r39) -> [ZME_DMA_COUNT]
	copyb(0, [ZMM_I32]) -> r6
	add(r6, ZMM_TT) -> r5
	dma_memcpy(r5, r38) -> r0
	; --- the exponent without its leading zero bytes: r12 = next byte, r13 = count;
	; r14 = &T[1][idx0], r17 = &T[v][idx0] - v * {KMAX * D}, r16 = zmm_sqr's choice
	; (zmm_mul and zmm_sqr keep all three) ---
	copyb(0, [ZMM_I32]) -> r6
	add(r6, ZMM_TT) -> r14
	copyb(0, [ZMM_TVM]) -> r17
	sub(r34, 3) -> r16
	ltu(r16, {KS - 2}) -> r16            ; r16 = 1 for 3 <= k <= {KS}: zmm_sqr squares with zms_go
	copyb(0, [ZMM_A_EXP]) -> r12
	copyb(0, [ZMM_A_ELEN]) -> r13
zmm_exp:
	eq(r13, 0), j(zmm_e0)
	copyb(r12, 1[a + 0]) -> r6
	add(r12, 1) -> r12
	sub(r13, 1) -> r13
	ltu(0, r6), j(zmm_e1)
	jump(zmm_exp)
zmm_e0:
	; exp == 0: the result is 1 (M > 1), as mod_len bytes
	copyb(0, [ZMM_A_OUT]) -> r16
	copyb(0, [ZMM_A_MLEN]) -> r15
	dma_xmemset(r16, r15) -> r0, j(0, 4)
	add(r16, r15) -> r5
	copyb(r5, 1) -> 1[a - 1]
	jump(zme_done)
zmm_e1:
	ltu(6, r13), j(zmm_w)               ; 8+ significant exponent bytes: 4-bit window
	; --- bit at a time: X = B is the leading bit; enter the byte body below it ---""")
    for k in range(7, 0, -1):
        w(f"\tltu(r6, {1 << k}), j(zmm_t{k})\n\tjump(zmm_b{k - 1})\nzmm_t{k}:")
    w("\tjump(zmm_next)")
    for k in range(7, -1, -1):
        w(f"zmm_b{k}:\n\tcall zmm_sqr\n\tand(r6, {1 << k})")
        w(f"\teq(c, 0), j(zmm_b{k - 1})" if k else "\teq(c, 0), j(zmm_next)")
        w("\tcopyb(0, r14) -> r10\n\tcall zmm_mul")
    w(f"""zmm_next:
	eq(r13, 0), j(zmm_out)
	copyb(r12, 1[a + 0]) -> r6
	add(r12, 1) -> r12
	sub(r13, 1) -> r13
	jump(zmm_b7)
	; --- 4-bit fixed window: T[v] = T[v - 1] * B, v = 2..15 ---
zmm_w:
	copyb(0, 2) -> r7
zmm_wt:
	copyb(0, r14) -> r10
	call zmm_mul
	copyb(0, r39) -> [ZME_DMA_COUNT]
	sll(r7, 10) -> r5
	sll(r7, 5)
	add(c, r5)
	add(c, r17) -> r5
	dma_memcpy(r5, r38) -> r0
	add(r7, 1) -> r7
	ltu(r7, 16), j(zmm_wt)
	copyb(0, r39) -> [ZME_DMA_COUNT]
	srl(r6, 4) -> r7                    ; the first byte's top nibble
	eq(r7, 0), j(zmm_wl0)
	sll(r7, 10) -> r5
	sll(r7, 5)
	add(c, r5)
	add(c, r17) -> r5
	dma_memcpy(r38, r5) -> r0      ; X = T[top nibble]
	jump(zmm_wlo)
zmm_wl0:
	and(r6, 15) -> r7
	sll(r7, 10) -> r5
	sll(r7, 5)
	add(c, r5)
	add(c, r17) -> r5
	dma_memcpy(r38, r5) -> r0      ; X = T[low nibble]
zmm_wnext:
	eq(r13, 0), j(zmm_out)
	copyb(r12, 1[a + 0]) -> r6
	add(r12, 1) -> r12
	sub(r13, 1) -> r13""")
    for lab, ext, nxt in (("", "srl(r6, 4) -> r7", "zmm_wlo"), ("zmm_wlo:", "and(r6, 15) -> r7", "zmm_wnext")):
        if lab:
            w(lab)
        for _ in range(4):
            w("\tcall zmm_sqr")
        w(f"\t{ext}\n\teq(r7, 0), j({nxt})\n\tsll(r7, 10) -> r5\n\tsll(r7, 5)\n\tadd(c, r5)\n\tadd(c, r17) -> r10\n\tcall zmm_mul")
    w(f"""	jump(zmm_wnext)
	; --- out of Montgomery form: X = X * 1 / R ---
zmm_out:
	copyb(0, [ZMM_I32]) -> r6
	add(r6, ZMM_R2) -> r10
	copyb(0, r39) -> r29
	dma_xmemset(r10, r29) -> r0, j(0, 4)
	copyb(r10, 1) -> 8[a + 0]
	call zmm_mul
	; --- X (k digits, < M) as mod_len big-endian bytes ---
	copyb(0, r39) -> r29
	add(r29, ZMM_BUFL - 8) -> r30      ; the last big-endian word
	srl(r29, 3) -> r31
	copyb(0, r38) -> r7
zmm_st:
	rev8(r7, 8[a + 0])
	copyb(r30, c) -> 8[a + 0]
	add(r7, 8) -> r7
	sub(r30, 8) -> r30
	sub(r31, 1) -> r31
	ltu(0, r31), j(zmm_st)
	copyb(0, [ZMM_A_OUT]) -> r16
	copyb(0, [ZMM_A_MLEN]) -> r15
	ltu(r15, r29), j(zmm_narrow)
	sub(r15, r29) -> r5
	dma_xmemset(r16, r5) -> r0, j(0, 4)  ; the zero bytes above the k-digit result
	add(r16, r5) -> r5
	copyb(0, r29) -> [ZME_DMA_COUNT]
	dma_memcpy(r5, ZMM_BUFL) -> r0
	jump(zme_done)
zmm_narrow:
	add(r29, ZMM_BUFL) -> r5
	sub(r5, r15) -> r5
	copyb(0, r15) -> [ZME_DMA_COUNT]
	dma_memcpy(r16, r5) -> r0
	jump(zme_done)
""")
    return "\n".join(o)


def gen_sqr():
    """X = X^2 / R mod M for k <= KS (X at r38): the square (cross products,
    doubled, plus the diagonal), then the CIOS reduction rows over it."""
    o = []
    w = o.append
    D = 32
    X0 = KMAX - KS                                     # lowest X position used
    P = lambda q: f"ZMS_P + {(q - PB) * D}"            # square digit at position p + q
    G = lambda q: f"ZMS_G + {(q - PB) * D}"            # diagonal squares
    X = lambda p: f"ZMM_X + {p * D}"
    H = lambda q: f"ZMM_H1 + {(q + 1) * D}"            # row 1's high halves, shared
    T = lambda p: f"ZMM_T + {(p + 1) * D}"
    NP_ = 2 * KMAX - PB
    w(f"""; ---------------------------------------------------------------------------
; zmm_sqr: X = X^2 / R mod M (X at r38). For 3 <= k <= {KS} digits: the 2k-digit square
; as cross products x_p * x_q (p < q; row p runs to the top digit, so one
; computed jump into row idx0 runs every row left), doubled by an add256 chain,
; plus the diagonal squares x_p^2; then the CIOS reduction rows of zmm_mul over
; it, bringing in one square digit per row, and zmm_mul's t - M tail. About 21%
; fewer arith256 than zmm_mul at k = 8 (-12.5% modexp cost). k = 2 and k > {KS}
; square through zmm_mul (at k = 2 the fixed overhead outweighs the saving).
;   Clobbers r10, r11, r28..r31.
; ---------------------------------------------------------------------------
u64 ZMS_P[{NP_ * 4}] = 0              ; the square, positions {PB}..{2 * KMAX - 1}
u64 ZMS_G[{NP_ * 4}] = 0              ; the diagonal squares
u64 ZMS_PLO[1] = 0                    ; per call: &P[2 * idx0]
u64 ZMS_ER[1] = 0                     ; per call: entry pcs
u64 ZMS_EDB[1] = 0
u64 ZMS_EDG[1] = 0
u64 ZMS_EDA[1] = 0
u64 ZMS_Q0[1] = 0                     ; per call: the first reduction row's add block""")
    rows = [p for p in range(X0, KMAX - 1)]
    for p in rows:
        for q in range(p + 1, KMAX):
            w(f"const u64 ZMS_A_{p}_{q}[5] = {X(p)}, {X(q)}, {P(p + q)}, ZMM_L, {H(q)}")
            b = "ZME_ZERO" if q == p + 1 else H(q - 1)
            w(f"u64 ZMS_D_{p}_{q}[4] = ZMM_L, {b}, 0, {P(p + q)}")
        w(f"u64 ZMS_RE_{p}[4] = {H(KMAX - 1)}, ZME_ZERO, 0, {P(p + KMAX)}")
    w(f"const u64 ZMS_ROWS[{len(rows)}] = " + ", ".join(f"zms_r{p}" for p in rows))
    db = []; da = []
    for q in range(PB, 2 * KMAX):
        db += [P(q), P(q), "0", P(q)]
        da += [P(q), G(q), "0", P(q)]
    w(f"u64 ZMS_DB[{len(db)}] = " + ", ".join(db))
    w(f"u64 ZMS_DA[{len(da)}] = " + ", ".join(da))
    for p in range(X0, KMAX):
        w(f"const u64 ZMS_DG_{p}[5] = {X(p)}, {X(p)}, ZME_ZERO, {G(2 * p)}, {G(2 * p + 1)}")
    adds = []
    for q in range(2 * KMAX - KS, 2 * KMAX):
        adds += [T(KMAX), P(q), "0", T(KMAX)]
    w(f"const u64 ZMS_ADDS[{len(adds)}] = " + ", ".join(adds))
    w(f"""
zms_go:
	add(r39, r39) -> r28
	copyb(0, [ZMS_PLO]) -> r29
	dma_xmemset(r29, r28) -> r0, j(0, 4)  ; the 2k square digits
	copyb(0, [ZMS_ER]), setpc(0)""")
    for p in rows:
        w(f"zms_r{p}:")
        for q in range(p + 1, KMAX):
            nxt = f"[ZMS_D_{p}_{q + 1} + 16]" if q < KMAX - 1 else f"[ZMS_RE_{p} + 16]"
            w(f"\tarith256(0, ZMS_A_{p}_{q})")
            w(f"\tadd256(0, ZMS_D_{p}_{q}) -> {nxt}")
        w(f"\tadd256(0, ZMS_RE_{p})")
    w("\t; --- double the cross products ---\n\tcopyb(0, [ZMS_EDB]), setpc(0)\nzms_db:")
    for i, q in enumerate(range(PB, 2 * KMAX)):
        st = f" -> [ZMS_DB + {(i + 1) * 32 + 16}]" if q < 2 * KMAX - 1 else ""
        w(f"\tadd256(0, ZMS_DB + {i * 32}){st}")
    w("\t; --- the diagonal squares, added in ---\n\tcopyb(0, [ZMS_EDG]), setpc(0)\nzms_dg:")
    for p in range(X0, KMAX):
        w(f"\tarith256(0, ZMS_DG_{p})")
    w("\tcopyb(0, [ZMS_EDA]), setpc(0)\nzms_da:")
    for i, q in enumerate(range(PB, 2 * KMAX)):
        st = f" -> [ZMS_DA + {(i + 1) * 32 + 16}]" if q < 2 * KMAX - 1 else ""
        w(f"\tadd256(0, ZMS_DA + {i * 32}){st}")
    w(f"""	; --- reduce: t = the low k square digits, then per row add the next one ---
	copyb(0, r39) -> [ZME_DMA_COUNT]
	dma_memcpy([ZMM_TP], r29) -> r0
	dma_xmemset({T(KMAX)}, 64) -> r0, j(0, 4)
	copyb(0, [ZMS_Q0]) -> r30
	copyb(0, r34) -> r11
	sub(r36, zmm_p2) -> r31
	add(r31, zms_p2) -> r31             ; zms_p2's entry for idx0
zms_row:
	add256(0, r30) -> [{T(KMAX + 1)}]
	arith256(0, ZMM_HM)                 ; m = t_0 * -M^-1
	copyb(0, r31), setpc(0)
zms_p2:""")
    for p in range(KMAX):
        w(f"\tarith256(0, ZMM_A2_{p})")
        w(f"\tadd256(0, ZMM_D2 + {p * 32}) -> [ZMM_D2 + {(p + 1) * 32 + 16}]")
    w(f"""	add256(0, ZMM_D2 + {KMAX * 32}) -> [ZMM_D3 + 16]
	add256(0, ZMM_D3)
	add(r30, 32) -> r30
	sub(r11, 1) -> r11
	ltu(0, r11), j(zms_row)
	jump(zmm_sub)
""")
    return o
