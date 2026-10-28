#!/usr/bin/env python3
"""kern.py - bn254 / bls12_381 tower kernels (Fp6, Fp12, cyclotomic square) via tc.py.

Fp6 = Fp2[v]/(v³-ξ) as [x0,x1,x2]; Fp12 = Fp6[w]/(w²-v) as [a(3) ‖ b(3)];
compressed cyclotomic = [g2,g3,g4,g5]. ZisK cost model: every Fp2 add, sub
and mul is one precompile of equal cost, so schoolbook beats Karatsuba for Fp6.
usage: kern.py <curve> <out.zisk>
"""
import sys
from tc import Tower

curve, outp = sys.argv[1], sys.argv[2]
tw = Tower(curve)
bls = curve == 'bls12_381'


def fp6_mul(k, a, b):
    xi = lambda x: k.cmul(x, 'xi')
    c0 = k.add(xi(k.add(k.mul(a[1], b[2]), k.mul(a[2], b[1]))), k.mul(a[0], b[0]))
    c1 = k.add(k.add(xi(k.mul(a[2], b[2])), k.mul(a[0], b[1])), k.mul(a[1], b[0]))
    c2 = k.add(k.add(k.mul(a[0], b[2]), k.mul(a[1], b[1])), k.mul(a[2], b[0]))
    return [c0, c1, c2]


def fp6_sq(k, a):
    c0 = k.add(k.cmul(k.mul(a[1], a[2]), '2xi'), k.sq(a[0]))
    c1 = k.add(k.dbl(k.mul(a[0], a[1])), k.cmul(k.sq(a[2]), 'xi'))
    c2 = k.add(k.dbl(k.mul(a[0], a[2])), k.sq(a[1]))
    return [c0, c1, c2]


def fp6_add(k, a, b): return [k.add(x, y) for x, y in zip(a, b)]
def fp6_sub(k, a, b): return [k.sub(x, y) for x, y in zip(a, b)]
def fp6_dbl(k, a): return [k.dbl(x) for x in a]
def fp6_mulv(k, a): return [k.cmul(a[2], 'xi'), a[0], a[1]]


# sparse Fp6 products, exactly the reference formulas (b = the nonzero Fp2 coefficients)
def mula(k, a, b2):                  # b = b2·v
    return [k.cmul(k.mul(b2, a[2]), 'xi'), k.mul(b2, a[0]), k.mul(b2, a[1])]


def bn_mulb(k, a, b1, b2):           # b = b1 + b2·v
    return [k.add(k.mul(a[0], b1), k.mul(a[2], k.cmul(b2, 'xi'))),
            k.add(k.mul(a[0], b2), k.mul(a[1], b1)),
            k.add(k.mul(a[1], b2), k.mul(a[2], b1))]


def bv_v2(k, a, b2, b3):             # b = b2·v + b3·v²  (bn254 mulc / bls12_381 mulb)
    return [k.cmul(k.add(k.mul(a[1], b3), k.mul(a[2], b2)), 'xi'),
            k.add(k.cmul(k.mul(a[2], b3), 'xi'), k.mul(a[0], b2)),
            k.add(k.mul(a[0], b3), k.mul(a[1], b2))]


def bls_mulc(k, a, b1, b3):          # b = b1 + b3·v²
    return [k.add(k.cmul(k.mul(a[1], b3), 'xi'), k.mul(a[0], b1)),
            k.add(k.cmul(k.mul(a[2], b3), 'xi'), k.mul(a[1], b1)),
            k.add(k.mul(a[0], b3), k.mul(a[2], b1))]


mulb = (lambda k, a, b1, b2: bv_v2(k, a, b1, b2)) if bls else bn_mulb
mulc = bls_mulc if bls else bv_v2

S = curve
A3 = lambda k, n: [k.inp('a', i) for i in range(n)]
B3 = lambda k, n: [k.inp('b', i) for i in range(n)]
def outs(k, arg, vals):
    for i, v in enumerate(vals):
        k.out(arg, i, v)

# ---- Fp6 ----
k = tw.kernel('mul_fp6', [('a', 'r10', 3, 'in'), ('b', 'r11', 3, 'in'), ('r', 'r12', 3, 'out')],
              f'void zisklib_mul_fp6_{S}(const u64* a, const u64* b, u64* result)  (schoolbook)')
outs(k, 'r', fp6_mul(k, A3(k, 3), B3(k, 3)))
k = tw.kernel('square_fp6', [('a', 'r10', 3, 'in'), ('r', 'r11', 3, 'out')],
              f'void zisklib_square_fp6_{S}(const u64* a, u64* result)')
outs(k, 'r', fp6_sq(k, A3(k, 3)))
k = tw.kernel('sparse_mula_fp6', [('a', 'r10', 3, 'in'), ('b', 'r11', 1, 'in'), ('r', 'r12', 3, 'out')],
              f'void zisklib_sparse_mula_fp6_{S}(const u64* a, const u64* b2, u64* result)  b = b2·v')
outs(k, 'r', mula(k, A3(k, 3), k.inp('b', 0)))
k = tw.kernel('sparse_mulb_fp6', [('a', 'r10', 3, 'in'), ('b', 'r11', 2, 'in'), ('r', 'r12', 3, 'out')],
              f'void zisklib_sparse_mulb_fp6_{S}(const u64* a, const u64* b, u64* result)  ' +
              ('b = b2·v + b3·v²' if bls else 'b = b1 + b2·v'))
outs(k, 'r', mulb(k, A3(k, 3), k.inp('b', 0), k.inp('b', 1)))
k = tw.kernel('sparse_mulc_fp6', [('a', 'r10', 3, 'in'), ('b', 'r11', 2, 'in'), ('r', 'r12', 3, 'out')],
              f'void zisklib_sparse_mulc_fp6_{S}(const u64* a, const u64* b, u64* result)  ' +
              ('b = b1 + b3·v²' if bls else 'b = b2·v + b3·v²'))
outs(k, 'r', mulc(k, A3(k, 3), k.inp('b', 0), k.inp('b', 1)))

# ---- Fp12 ----
k = tw.kernel('mul_fp12', [('a', 'r10', 6, 'in'), ('b', 'r11', 6, 'in'), ('r', 'r12', 6, 'out')],
              f'void zisklib_mul_fp12_{S}(const u64* a, const u64* b, u64* result)  (Karatsuba over Fp6)')
a, b = A3(k, 6), B3(k, 6)
t1 = fp6_mul(k, a[:3], b[:3])
t2 = fp6_mul(k, a[3:], b[3:])
c1 = fp6_add(k, t1, fp6_mulv(k, t2))
c2 = fp6_sub(k, fp6_sub(k, fp6_mul(k, fp6_add(k, a[:3], a[3:]), fp6_add(k, b[:3], b[3:])), t1), t2)
outs(k, 'r', c1 + c2)

k = tw.kernel('square_fp12', [('a', 'r10', 6, 'in'), ('r', 'r11', 6, 'out')],
              f'void zisklib_square_fp12_{S}(const u64* a, u64* result)  (a0² + v·a1²) + 2·a0·a1·w')
a = A3(k, 6)
c1 = fp6_add(k, fp6_sq(k, a[:3]), fp6_mulv(k, fp6_sq(k, a[3:])))
c2 = fp6_dbl(k, fp6_mul(k, a[:3], a[3:]))
outs(k, 'r', c1 + c2)

k = tw.kernel('sparse_mul_fp12', [('a', 'r10', 6, 'in'), ('b', 'r11', 2, 'in'), ('r', 'r12', 6, 'out')],
              f'void zisklib_sparse_mul_fp12_{S}(const u64* a, const u64* b, u64* result)  (line)')
a = A3(k, 6)
b0, b1 = k.inp('b', 0), k.inp('b', 1)
if bls:   # c1 = mulc(a2, [b23·ξ, b22]) + a1 ; c2 = mulb(a1, [b22, b23]) + a2
    c1 = fp6_add(k, mulc(k, a[3:], k.cmul(b1, 'xi'), b0), a[:3])
    c2 = fp6_add(k, mulb(k, a[:3], b0, b1), a[3:])
else:     # c1 = mulc(a2, b) + a1 ; c2 = mulb(a1, b) + a2
    c1 = fp6_add(k, mulc(k, a[3:], b0, b1), a[:3])
    c2 = fp6_add(k, mulb(k, a[:3], b0, b1), a[3:])
outs(k, 'r', c1 + c2)

# ---- compressed cyclotomic square (Karabina) ----
k = tw.kernel('square_cyclo', [('a', 'r10', 4, 'in'), ('r', 'r11', 4, 'out')],
              f'void zisklib_square_cyclo_{S}(const u64* a, u64* result)  [g2,g3,g4,g5]')
g2, g3, g4, g5 = A3(k, 4)
h2 = k.dbl(k.add(k.cmul(k.mul(g4, g5), '3xi'), g2))                                   # 2(g2 + 3ξ·g4g5)
h3 = k.sub(k.sub(k.cmul(k.add(k.sq(g4), k.cmul(k.sq(g5), 'xi')), '3'), g3), g3)        # 3(g4² + ξg5²) - 2g3
h4 = k.sub(k.sub(k.cmul(k.add(k.sq(g2), k.cmul(k.sq(g3), 'xi')), '3'), g4), g4)        # 3(g2² + ξg3²) - 2g4
h5 = k.dbl(k.add(k.cmul(k.mul(g2, g3), '3'), g5))                                     # 2(g5 + 3·g2g3)
outs(k, 'r', [h2, h3, h4, h5])

d = 'bls12_381' if bls else 'bn254'
hdr = ['; ============================================================================',
       f'; {d}/kernels.zisk - straight-line Fp6 / Fp12 / cyclotomic kernels for {curve}.',
       '; GENERATED by scripts/pairing/kern.py + tc.py (tower compiler). Each Fp2 op is',
       '; one precompile updating its first operand in place: ops run in place on a',
       '; dying operand, else copy it with one DMA op; chains ending in an output are',
       '; computed directly through the result pointer. A result pointer equal to an',
       '; input pointer is handled by first copying that input to a private buffer.',
       '; Formulas follow the ZisK cost model (add = sub = mul = one precompile), e.g.',
       '; schoolbook Fp6 multiplication. All kernels clobber r5, r14 (and r10/r11 when',
       '; aliased). Prefix ' + tw.pfx + '_.',
       '; ============================================================================']
L = tw.emit(hdr)
open(outp, 'w').write('\n'.join(L) + '\n')
for k in tw.kernels:
    print(f'{curve} {k.name}: ops={k.stats[0]} copies={k.stats[1]}', file=sys.stderr)
