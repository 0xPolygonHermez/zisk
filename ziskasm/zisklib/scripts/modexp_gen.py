#!/usr/bin/env python3
"""Rewrites the entry of ziskasm/zisklib/zkvm/modexp.zisk: direct paths for moduli
of up to 32 bytes (arith256_mod) and 48 bytes (arith384_mod), and the Montgomery
path (modexp_mont.py) for odd moduli of 49..1056 bytes; other moduli go to the
unchanged general path, renamed zme_legacy.

Usage: modexp_gen.py FILE  (edits FILE in place). The input is the general-path
source, i.e. modexp.zisk before the rewrite:
    git show ca6bbb955:ziskasm/zisklib/zkvm/modexp.zisk > modexp.zisk
    python3 modexp_gen.py modexp.zisk
    mv modexp.zisk ../zkvm/modexp.zisk
"""
import sys
p = sys.argv[1]
s = open(p).read()
hdr_end = s.index('; entry arg spills')
a = s.index('ziskasm_zkvm_modexp:')
b = s.index('\t; --- spill the seven EF arguments ---')
legacy_start = s.index('\n', s.index('eq(r15, 0), j(zme_emptymod)', a)) + 1

def z(n): return ', '.join(['0'] * n)

o = []; w = o.append
w('''; ---------------------------------------------------------------------------
; Direct paths. The modulus is taken without its leading zero bytes; when that
; leaves at most 32 bytes (48 bytes) the whole exponentiation runs on one
; arith256_mod (arith384_mod) per modular multiplication, whose quotient witness
; is 512 (768) bits wide, so any operands below 2^256 (2^384) are accepted:
;   - the base is reduced with Horner over 32-byte chunks, most significant
;     first: X = X * 2^256 + chunk (mod M), with R = 2^256 mod M;
;   - the exponent bits are read straight from its big-endian bytes, left to
;     right (no bit-decomposition hint): per bit X = X^2, and X = X * B if set,
;     from an unrolled 8-bit byte body. The first byte's leading 1 bit sets
;     X = B and enters the body just below it.
; Each precompile call uses a preset parameter block, so it is a single step.
; ---------------------------------------------------------------------------
define ZME_DMA_COUNT 0xa0400f00         ; dma_memcpy reads its byte count here

u64 ZME_RA[1]   = 0
u64 ZME_BUF[6]  = 0, 0, 0, 0, 0, 0      ; a right-aligned big-endian operand
u64 ZME_OBUF[6] = 0, 0, 0, 0, 0, 0      ; the big-endian result
const u64 ZME_ZERO[6] = 0, 0, 0, 0, 0, 0
const u64 ZME_ONE[6]  = 1, 0, 0, 0, 0, 0
const u64 ZME_T128[6] = 0, 0, 1, 0, 0, 0 ; 2^128: R = 2^128 * 2^128 mod M
''')
for W, op in ((32, 'arith256_mod'), (48, 'arith384_mod')):
    N = W // 8; P = f'ZME{N}'
    w(f'''u64 {P}_M[{N}]  = {z(N)}
u64 {P}_X[{N}]  = {z(N)}
u64 {P}_T[120] = {z(120)}  ; T[v] = B^v at {P}_T + (v - 1) * 64, v = 1..15 (T[1] = B)
u64 {P}_R[{N}]  = {z(N)}
u64 {P}_CH[{N}] = {z(N)}                 ; a 32-byte base chunk (upper limbs stay 0)
u64 {P}_HRED[5] = {P}_CH, ZME_ONE, ZME_ZERO, {P}_M, {P}_X   ; X = chunk mod M
u64 {P}_HR[5]   = ZME_T128, ZME_T128, ZME_ZERO, {P}_M, {P}_R ; R = 2^256 mod M
u64 {P}_HHOR[5] = {P}_X, {P}_R, {P}_CH, {P}_M, {P}_X        ; X = X * R + chunk
u64 {P}_HSQ[5]  = {P}_X, {P}_X, ZME_ZERO, {P}_M, {P}_X      ; X = X^2
u64 {P}_HMUL[5] = {P}_X, {P}_T, ZME_ZERO, {P}_M, {P}_X      ; X = X * B
u64 {P}_HONE[5] = ZME_ONE, ZME_ONE, ZME_ZERO, {P}_M, {P}_X  ; X = 1 mod M
u64 {P}_HMW[5]  = {P}_X, {P}_T, ZME_ZERO, {P}_M, {P}_X      ; X = X * T[v] (b set per nibble)''')
    for k in range(2, 16):
        w(f'u64 {P}_HT{k}[5] = {P}_T + {(k - 2) * 64}, {P}_T, ZME_ZERO, {P}_M, {P}_T + {(k - 1) * 64}')
    w('')
w('''ziskasm_zkvm_modexp:
	; Length guards: base(r11)/exp(r13)/mod(r15) byte lengths must fit the 132-u64
	; (1056-byte) limb buffers of the general path; an over-long operand returns
	; ZKVM_EFAIL instead of overrunning them.
	ltu(1056, r11), j(zme_toolong0)
	ltu(1056, r13), j(zme_toolong0)
	ltu(1056, r15), j(zme_toolong0)
	; An empty modulus has an empty output (EIP-198), so `output` need not be valid.
	eq(r15, 0), j(zme_eok0)
	; --- r17 = the modulus without its leading zero bytes, r28 = its length ---
	copyb(0, r14) -> r17
	copyb(0, r15) -> r28
zme_mz:
	copyb(r17, 1[a + 0]) -> r5
	ltu(0, r5), j(zme_mzd)
	add(r17, 1) -> r17
	sub(r28, 1) -> r28
	eq(r28, 0), j(zme_mzero)
	jump(zme_mz)
zme_mzd:
	copyb(0, r1) -> [ZME_RA]
	ltu(48, r28), j(zmm)                ; above 384 bits: Montgomery or the general path
	ltu(32, r28), j(zme6)
	jump(zme4)

; modulus == 0: the result is 0, as mod_len zero bytes
zme_mzero:
	dma_xmemset(r16, r15) -> r0, j(0, 4)
zme_eok0:
	copyb(0, 0) -> r10                  ; ZKVM_EOK
	ret
zme_toolong0:
	copyb(0, 0xffffffffffffffff) -> r10 ; ZKVM_EFAIL
	ret

; zme_ld{32,48}: [r5 = src, r6 = count (0..W), r7 = dst limbs] the count-byte
;   big-endian number at src as W/8 little-endian limbs at dst. Clobbers r28.
''')
for W in (32, 48):
    N = W // 8
    w(f'zme_ld{W}:')
    w(f'\tdma_xmemset(ZME_BUF, {W}) -> r0, j(0, 4)')
    w(f'\tsub(ZME_BUF + {W}, r6) -> r28')
    w('\tcopyb(0, r6) -> [ZME_DMA_COUNT]')
    w('\tdma_memcpy(r28, r5) -> r0')
    for k in range(N):
        w(f'\trev8(r7, [ZME_BUF + {W - 8 - 8*k}]) -> 8[a + {8*k}]')
    w('\tret\n')

for W, op in ((32, 'arith256_mod'), (48, 'arith384_mod')):
    N = W // 8; P = f'ZME{N}'; L = f'zme{N}'
    w(f'''; --- the modulus fits {W * 8} bits: r17 = its first byte, r28 = its length ---
{L}:
	copyb(0, r17) -> r5
	copyb(0, r28) -> r6
	copyb(0, {P}_M) -> r7
	call zme_ld{W}
	; --- X = base mod M: the top (1..32 byte) chunk, then 32-byte chunks ---
	copyb(0, r11) -> r6
	ltu(32, r11), j({L}_long)
	copyb(0, r10) -> r5
	copyb(0, {P}_CH) -> r7
	call zme_ld32
	{op}(0, {P}_HRED)
	jump({L}_exp)
{L}_long:
	sub(r11, 1) -> r6
	and(r6, 31) -> r6
	add(r6, 1) -> r6                    ; the top chunk's length
	copyb(0, r10) -> r5
	add(r10, r6) -> r10                 ; r10 = the next chunk
	sub(r11, r6) -> r11                 ; r11 = bytes after the top chunk (a multiple of 32)
	copyb(0, {P}_CH) -> r7
	call zme_ld32
	{op}(0, {P}_HRED)
	{op}(0, {P}_HR)
{L}_chunk:
	copyb(0, r10) -> r5
	copyb(0, 32) -> r6
	copyb(0, {P}_CH) -> r7
	call zme_ld32
	{op}(0, {P}_HHOR)
	add(r10, 32) -> r10
	sub(r11, 32) -> r11
	eq(r11, 0), j({L}_exp)
	jump({L}_chunk)
	; --- the exponent without its leading zero bytes: r12 = next byte, r13 = count ---
{L}_exp:
	eq(r13, 0), j({L}_e0)
	copyb(r12, 1[a + 0]) -> r6
	add(r12, 1) -> r12
	sub(r13, 1) -> r13
	ltu(0, r6), j({L}_e1)
	jump({L}_exp)
{L}_e0:
	{op}(0, {P}_HONE)                ; exp == 0: X = 1 mod M
	jump({L}_out)
{L}_e1:
	dma_xmemcpy({P}_T, {P}_X) -> r0, j({W}, 4)  ; B = T[1] = base
	ltu(6, r13), j({L}_w)               ; 8+ significant exponent bytes: 4-bit window
	; --- bit at a time: X = B is the leading bit; enter the byte body below it ---''')
    for k in range(7, 0, -1):
        w(f'\tltu(r6, {1 << k}), j({L}_t{k})\n\tjump({L}_b{k - 1})\n{L}_t{k}:')
    w(f'\tjump({L}_next)')
    for k in range(7, -1, -1):
        w(f'{L}_b{k}:\n\t{op}(0, {P}_HSQ)\n\tand(r6, {1 << k})')
        w(f'\teq(c, 0), j({L}_b{k - 1})' if k else f'\teq(c, 0), j({L}_next)')
        w(f'\t{op}(0, {P}_HMUL)')
    w(f'''{L}_next:
	eq(r13, 0), j({L}_out)
	copyb(r12, 1[a + 0]) -> r6
	add(r12, 1) -> r12
	sub(r13, 1) -> r13
	jump({L}_b7)
	; --- 4-bit fixed window: T[2..15] = B^2..B^15, then X = X^16 * T[v] per nibble ---
{L}_w:''')
    for k in range(2, 16):
        w(f'\t{op}(0, {P}_HT{k})')
    w(f'''	srl(r6, 4) -> r7                    ; the first byte's top nibble
	eq(r7, 0), j({L}_wl0)
	sll(r7, 6)
	add(c, {P}_T - 64) -> r5
	dma_xmemcpy({P}_X, r5) -> r0, j({W}, 4)   ; X = T[top nibble]
	jump({L}_wlo)
{L}_wl0:
	and(r6, 15) -> r7
	sll(r7, 6)
	add(c, {P}_T - 64) -> r5
	dma_xmemcpy({P}_X, r5) -> r0, j({W}, 4)   ; X = T[low nibble]
{L}_wnext:
	eq(r13, 0), j({L}_out)
	copyb(r12, 1[a + 0]) -> r6
	add(r12, 1) -> r12
	sub(r13, 1) -> r13''')
    for lab, ext, nxt in (('', 'srl(r6, 4) -> r7', f'{L}_wlo'), (f'{L}_wlo:', 'and(r6, 15) -> r7', f'{L}_wnext')):
        if lab: w(lab)
        for _ in range(4): w(f'\t{op}(0, {P}_HSQ)')
        w(f'\t{ext}\n\teq(r7, 0), j({nxt})\n\tsll(r7, 6)\n\tadd(c, {P}_T - 64) -> [{P}_HMW + 8]\n\t{op}(0, {P}_HMW)')
    w(f'''	jump({L}_wnext)
	; --- output: X as mod_len big-endian bytes (X < M, so no bytes are lost) ---
{L}_out:''')
    for k in range(N):
        w(f'\trev8(0, [{P}_X + {8*k}]) -> [ZME_OBUF + {W - 8 - 8*k}]')
    w(f'''	ltu({W}, r15), j({L}_wide)
	sub(ZME_OBUF + {W}, r15) -> r5
	copyb(0, r15) -> [ZME_DMA_COUNT]
	dma_memcpy(r16, r5) -> r0
	jump(zme_done)
{L}_wide:
	sub(r15, {W}) -> r5
	dma_xmemset(r16, r5) -> r0, j(0, 4)  ; the zero bytes above the {W}-byte result
	add(r16, r5) -> r5
	dma_xmemcpy(r5, ZME_OBUF) -> r0, j({W}, 4)
	jump(zme_done)
''')
import importlib.util as _u
_spec = _u.spec_from_file_location('modexp_mont', __file__.rsplit('/', 1)[0] + '/modexp_mont.py')
_m = _u.module_from_spec(_spec); _spec.loader.exec_module(_m)
w(_m.gen())
w('''zme_done:
	copyb(0, [ZME_RA]) -> r1
	copyb(0, 0) -> r10                  ; ZKVM_EOK
	ret

; --- moduli above 48 bytes: the general path (r10..r16 are still the arguments) ---
zme_legacy:
	push r1
''')
new = s[:hdr_end] + s[hdr_end:a] + '\n'.join(o) + s[b:]
# the general path's own guards are unreachable now (checked at the entry)
x = new.index('; Operand longer than the 132-limb (1056-byte) capacity')
y = new.index('; ---------------------------------------------------------------------------\n; zme_dispatch:')
new = new[:x] + new[y:]
x = new.index('; The EF ABI passes arbitrary-length')
y = new.index('; ============================================================================\n', 100)
new = new[:x] + """; The EF ABI passes arbitrary-length big-endian byte arrays. The modulus is
; taken without its leading zero bytes, and its size picks the path:
;   - up to 48 bytes, a direct path: one arith256_mod / arith384_mod per modular
;     multiplication;
;   - odd, 49..1056 bytes, Montgomery multiplication over 256-bit digits (CIOS,
;     arith256 + add256 chains);
;   - even, over 48 bytes, the general path (zme_legacy): each byte array
;     marshalled to LE limbs (zkvm_be_bytes_to_limbs), the modexp dispatcher
;     LIFTED verbatim from zisklib_modexp_u64_c (bigint/modexp.zisk; hinted
;     division per multiplication), and the result limbs back to mod_len bytes.
;     NOTHING here calls zisklib_modexp_u64_c itself.
; The first two walk the exponent straight from its big-endian bytes, bit at a
; time for short exponents and with a 4-bit fixed window from 8 significant bytes.
; GENERATED (the direct and Montgomery paths) by scripts/modexp_gen.py +
; scripts/modexp_mont.py from the general-path source.
; Labels `zme_` / `zmm_`, data `ZKVM_ME_*` / `ZME*` / `ZMM_*` (BI_ZERO/BI_ONE are
; shared consts).
""" + new[y:]
open(p, 'w').write(new)
