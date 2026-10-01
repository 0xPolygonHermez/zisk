#!/usr/bin/env python3
"""curves_vec.py [SEED] [OUT.c] [--fast]: many bn254 / bls12-381 / kzg vectors through the EF zkvm ABI.

Each case records (status, output bytes) into a transcript; cases with a known
expected value (py_ecc) are checked in-guest. The guest emits
  [fails u32][first_fail u32][ncases u32][0 u32][keccak(transcript) 32 B]
so a run can be compared both against py_ecc and against another build.
"""
import random, sys
from py_ecc import optimized_bn128 as bn
from py_ecc import optimized_bls12_381 as bl
from py_ecc.bls.point_compression import compress_G1

random.seed(int(sys.argv[1]) if len(sys.argv) > 1 else 1)
OUT = sys.argv[2] if len(sys.argv) > 2 else 'vec_guest.c'
FAST = '--fast' in sys.argv

def be(x, n): return x.to_bytes(n, 'big')

# ---------------- bn254 ----------------
BP, BN = bn.field_modulus, bn.curve_order
def bn_g1(P):
    if bn.is_inf(P): return bytes(64)
    x, y = bn.normalize(P); return be(x.n, 32) + be(y.n, 32)
def bn_g2(Q):
    if bn.is_inf(Q): return bytes(128)
    x, y = bn.normalize(Q)
    return be(x.coeffs[1], 32) + be(x.coeffs[0], 32) + be(y.coeffs[1], 32) + be(y.coeffs[0], 32)
def bn_rg1(): return bn.multiply(bn.G1, random.randrange(1, BN))
def bn_rg2(): return bn.multiply(bn.G2, random.randrange(1, BN))

# ---------------- bls12-381 ----------------
LP, LN = bl.field_modulus, bl.curve_order
def bl_g1(P):
    if bl.is_inf(P): return bytes(96)
    x, y = bl.normalize(P); return be(x.n, 48) + be(y.n, 48)
def bl_g2(Q):
    if bl.is_inf(Q): return bytes(192)
    x, y = bl.normalize(Q)
    return be(x.coeffs[0], 48) + be(x.coeffs[1], 48) + be(y.coeffs[0], 48) + be(y.coeffs[1], 48)
def bl_rg1(): return bl.multiply(bl.G1, random.randrange(1, LN))
def bl_rg2(): return bl.multiply(bl.G2, random.randrange(1, LN))

cases = []   # (call-kind, [input bytes...], n, out_len, expected-bytes-or-None)
def case(kind, ins, outlen, exp=None, n=0):
    cases.append((kind, ins, n, outlen, exp))

def st(ok=True): return b'\x00' if ok else None

# ---- bn254 g1 add ----
for _ in range(6):
    P, Q = bn_rg1(), bn_rg1()
    case('bn_add', [bn_g1(P), bn_g1(Q)], 64, bn_g1(bn.add(P, Q)))
P = bn_rg1()
case('bn_add', [bn_g1(P), bn_g1(P)], 64, bn_g1(bn.double(P)))
case('bn_add', [bn_g1(P), bn_g1(bn.neg(P))], 64, bytes(64))
case('bn_add', [bytes(64), bn_g1(P)], 64, bn_g1(P))
case('bn_add', [bn_g1(P), bytes(64)], 64, bn_g1(P))
case('bn_add', [bytes(64), bytes(64)], 64, bytes(64))
case('bn_add', [bn_g1(P)[:32] + be((bn.normalize(P)[1].n + 1) % BP, 32), bn_g1(P)], 64)   # off curve
case('bn_add', [be(BP, 32) + bn_g1(P)[32:], bn_g1(P)], 64)                                # x >= p
# ---- bn254 g1 mul ----
for k in [random.randrange(BN) for _ in range(5)] + [0, 1, 2, BN - 1, BN, BN + 1, 2**256 - 1]:
    P = bn_rg1()
    case('bn_mul', [bn_g1(P), be(k, 32)], 64, bn_g1(bn.multiply(P, k % BN)))
case('bn_mul', [bytes(64), be(12345, 32)], 64, bytes(64))
case('bn_mul', [bytes(64), be(2, 32)], 64, bytes(64))        # doubling the identity
case('bn_mul', [be(1, 32) + be(3, 32), be(7, 32)], 64)        # off curve
# ---- bn254 pairing ----
def bn_pairs(ps): return [b''.join(bn_g1(P) + bn_g2(Q) for P, Q in ps)]
a, b = random.randrange(1, BN), random.randrange(1, BN)
P, Q = bn_rg1(), bn_rg2()
case('bn_pair', [b''], 2, b'\x00\x01', n=0)
case('bn_pair', bn_pairs([(P, Q), (bn.neg(P), Q)]), 2, b'\x00\x01', n=2)
case('bn_pair', bn_pairs([(bn.multiply(P, a), bn.multiply(Q, b)), (bn.neg(bn.multiply(P, a * b % BN)), Q)]), 2, b'\x00\x01', n=2)
case('bn_pair', bn_pairs([(P, Q), (bn_rg1(), bn_rg2())]), 2, b'\x00\x00', n=2)
case('bn_pair', bn_pairs([(P, Q)]), 2, b'\x00\x00', n=1)
case('bn_pair', bn_pairs([(bn.Z1, Q)]), 2, b'\x00\x01', n=1)
case('bn_pair', bn_pairs([(P, bn.Z2)]), 2, b'\x00\x01', n=1)
case('bn_pair', bn_pairs([(bn.Z1, bn.Z2)]), 2, b'\x00\x01', n=1)
c = random.randrange(1, BN)
case('bn_pair', bn_pairs([(bn.multiply(P, a), Q), (bn.multiply(P, c), Q), (bn.neg(bn.multiply(P, (a + c) % BN)), Q)]), 2, b'\x00\x01', n=3)
case('bn_pair', [bn_g1(P) + bn_g2(Q)[:127] + bytes([bn_g2(Q)[127] ^ 1])], 2, n=1)   # G2 off curve
case('bn_pair', [bn_g1(P)[:63] + bytes([bn_g1(P)[63] ^ 1]) + bn_g2(Q)], 2, n=1)     # G1 off curve

# ---- bls g1 / g2 add ----
for _ in range(4):
    P, Q = bl_rg1(), bl_rg1()
    case('bl_g1add', [bl_g1(P), bl_g1(Q)], 96, bl_g1(bl.add(P, Q)))
P = bl_rg1()
case('bl_g1add', [bl_g1(P), bl_g1(P)], 96, bl_g1(bl.double(P)))
case('bl_g1add', [bl_g1(P), bl_g1(bl.neg(P))], 96, bytes(96))
case('bl_g1add', [bytes(96), bl_g1(P)], 96, bl_g1(P))
case('bl_g1add', [bl_g1(P), bytes(96)], 96, bl_g1(P))
case('bl_g1add', [bl_g1(P)[:95] + bytes([bl_g1(P)[95] ^ 1]), bl_g1(P)], 96)   # off curve
for _ in range(3):
    P, Q = bl_rg2(), bl_rg2()
    case('bl_g2add', [bl_g2(P), bl_g2(Q)], 192, bl_g2(bl.add(P, Q)))
Q = bl_rg2()
case('bl_g2add', [bl_g2(Q), bl_g2(Q)], 192, bl_g2(bl.double(Q)))
case('bl_g2add', [bl_g2(Q), bl_g2(bl.neg(Q))], 192, bytes(192))
case('bl_g2add', [bytes(192), bl_g2(Q)], 192, bl_g2(Q))
case('bl_g2add', [bl_g2(Q)[:191] + bytes([bl_g2(Q)[191] ^ 1]), bl_g2(Q)], 192)
# ---- bls msm ----
def msm1(ps): return [b''.join(bl_g1(P) + be(k, 32) for P, k in ps)]
def msm2(ps): return [b''.join(bl_g2(P) + be(k, 32) for P, k in ps)]
def lin(ps, z):
    acc = z
    for P, k in ps: acc = bl.add(acc, bl.multiply(P, k % LN))
    return acc
for n in (1, 2, 3, 4):
    ps = [(bl_rg1(), random.randrange(2**256)) for _ in range(n)]
    if n == 3: ps[1] = (ps[1][0], 0)
    if n == 4: ps[2] = (bl.Z1, ps[2][1])
    case('bl_g1msm', msm1(ps), 96, bl_g1(lin(ps, bl.Z1)), n=n)
ps = [(bl_rg1(), LN)]
case('bl_g1msm', msm1(ps), 96, bytes(96), n=1)
# doubling the identity, and doublings that cancel (k = 2 is a fast path)
P = bl_rg1()
for ps in ([(bl.Z1, 2)], [(P, 2)], [(bl.Z1, 1), (bl.Z1, 2)], [(P, 2), (bl.neg(P), 2)]):
    case('bl_g1msm', msm1(ps), 96, bl_g1(lin(ps, bl.Z1)), n=len(ps))
for n in (1, 2, 3):
    ps = [(bl_rg2(), random.randrange(2**256)) for _ in range(n)]
    if n == 3: ps[0] = (ps[0][0], 1)
    case('bl_g2msm', msm2(ps), 192, bl_g2(lin(ps, bl.Z2)), n=n)
Q = bl_rg2()
for ps in ([(bl.Z2, 2)], [(Q, 2)], [(bl.Z2, 1), (bl.Z2, 2)], [(Q, 2), (bl.neg(Q), 2)]):
    case('bl_g2msm', msm2(ps), 192, bl_g2(lin(ps, bl.Z2)), n=len(ps))
# ---- bls pairing ----
def bl_pairs(ps): return [b''.join(bl_g1(P) + bl_g2(Q) for P, Q in ps)]
P, Q = bl_rg1(), bl_rg2()
a, b = random.randrange(1, LN), random.randrange(1, LN)
case('bl_pair', bl_pairs([(P, Q), (bl.neg(P), Q)]), 2, b'\x00\x01', n=2)
case('bl_pair', bl_pairs([(bl.multiply(P, a), bl.multiply(Q, b)), (bl.neg(bl.multiply(P, a * b % LN)), Q)]), 2, b'\x00\x01', n=2)
case('bl_pair', bl_pairs([(P, Q)]), 2, b'\x00\x00', n=1)
case('bl_pair', bl_pairs([(P, Q), (bl_rg1(), Q)]), 2, b'\x00\x00', n=2)
case('bl_pair', bl_pairs([(bl.Z1, Q)]), 2, b'\x00\x01', n=1)
case('bl_pair', bl_pairs([(P, bl.Z2)]), 2, b'\x00\x01', n=1)
case('bl_pair', bl_pairs([(bl.Z1, bl.Z2)]), 2, b'\x00\x01', n=1)
if not FAST:
    c = random.randrange(1, LN)
    case('bl_pair', bl_pairs([(bl.multiply(P, a), Q), (bl.multiply(P, c), Q), (bl.neg(bl.multiply(P, (a + c) % LN)), Q)]), 2, b'\x00\x01', n=3)
case('bl_pair', [bl_g1(P) + bl_g2(Q)[:191] + bytes([bl_g2(Q)[191] ^ 1])], 2, n=1)
# ---- bls map (differential only) ----
for _ in range(4):
    case('bl_map1', [be(random.randrange(LP), 48)], 96)
for _ in range(3):
    case('bl_map2', [be(random.randrange(LP), 48) + be(random.randrange(LP), 48)], 192)
case('bl_map1', [be(0, 48)], 96)
case('bl_map1', [be(LP, 48)], 96)         # non-canonical
case('bl_map2', [bytes(96)], 192)
# ---- kzg (differential; one known-valid vector + tampered / random) ----
KC = bytes.fromhex('8530c1bdc4cd6b1408be0933c4a41ac3513350eef36850b804708e1f338932ce01b655a163344a4500b281c8750c461f')
KZ, KY = be(0x309, 32), be(0x3039, 32)
KP = bytes([0xc0]) + bytes(47)
case('kzg', [KC, KZ, KY, KP], 2)
case('kzg', [KC, KZ, be(0x1869f, 32), KP], 2)
case('kzg', [KC, be(0x30a, 32), KY, KP], 2)
for _ in range(2):
    C, Pp = compress_G1(bl_rg1()), compress_G1(bl_rg1())
    case('kzg', [be(C, 48), be(random.randrange(LN), 32), be(random.randrange(LN), 32), be(Pp, 48)], 2)
case('kzg', [KC, be(LN, 32), KY, KP], 2)                 # z non-canonical
case('kzg', [bytes([0x00]) + KC[1:], KZ, KY, KP], 2)     # bad compression flag


# ---- more: KZG proofs that verify (built from the public [tau]G1), batch overflow ----
from py_ecc.bls.point_compression import decompress_G1
import glob, os
# The Ethereum KZG trusted setup, as shipped by the c-kzg crate (a zisk dependency).
_ts = os.environ.get('KZG_TRUSTED_SETUP') or (sorted(glob.glob(os.path.expanduser(
    '~/.cargo/registry/src/*/c-kzg-*/src/trusted_setup.txt'))) or [None])[-1]
if not _ts: sys.exit('trusted_setup.txt not found: set KZG_TRUSTED_SETUP')
TS = open(_ts).read().split()
TAU1 = decompress_G1(int(TS[2 + 4096 + 65 + 1], 16))
for _ in range(2 if FAST else 3):
    a_, z_, y_ = random.randrange(1, LN), random.randrange(LN), random.randrange(LN)
    Cp = bl.add(bl.multiply(bl.G1, y_), bl.multiply(bl.add(TAU1, bl.neg(bl.multiply(bl.G1, z_))), a_))
    Pp = bl.multiply(bl.G1, a_)
    C, Pr = be(compress_G1(Cp), 48), be(compress_G1(Pp), 48)
    case('kzg', [C, be(z_, 32), be(y_, 32), Pr], 2, b'\x00\x01')
    case('kzg', [C, be(z_, 32), be((y_ + 1) % LN, 32), Pr], 2, b'\x00\x00')
for n in (9, 17):
    P, Q = bn_rg1(), bn_rg2()
    ks = [random.randrange(1, BN) for _ in range(n - 1)]
    ps = [(bn.multiply(P, k), Q) for k in ks] + [(bn.neg(bn.multiply(P, sum(ks) % BN)), Q)]
    case('bn_pair', bn_pairs(ps), 2, b'\x00\x01', n=n)
    ps[3] = (bn.multiply(P, 5), Q)
    case('bn_pair', bn_pairs(ps), 2, b'\x00\x00', n=n)
for n in (9,):
    P, Q = bl_rg1(), bl_rg2()
    ks = [random.randrange(1, LN) for _ in range(n - 1)]
    ps = [(bl.multiply(P, k), Q) for k in ks] + [(bl.neg(bl.multiply(P, sum(ks) % LN)), Q)]
    case('bl_pair', bl_pairs(ps), 2, b'\x00\x01', n=n)
    ps[2] = (bl.multiply(P, 7), Q)
    case('bl_pair', bl_pairs(ps), 2, b'\x00\x00', n=n)

# ---- on-curve points outside the prime-order subgroup (must be rejected) ----
from py_ecc.bls import hash_to_curve as h2c
from py_ecc.fields import optimized_bls12_381_FQ as LFQ, optimized_bls12_381_FQ2 as LFQ2
def bl_nosub_g1():
    while True:
        x = random.randrange(LP); r = (x**3 + 4) % LP; y = pow(r, (LP + 1) // 4, LP)
        if y * y % LP == r:
            P = (bl.FQ(x), bl.FQ(y), bl.FQ(1))
            if not bl.is_inf(bl.multiply(P, LN)): return P
def bl_nosub_g2():
    u = LFQ2([random.randrange(LP), random.randrange(LP)])
    P = h2c.iso_map_G2(*h2c.map_to_curve_G2(u))
    assert not bl.is_inf(bl.multiply(P, LN))
    return P
def bn_nosub_g2():
    F2 = bn.FQ2
    while True:
        x = F2([random.randrange(BP), random.randrange(BP)])
        a = x**3 + bn.b2
        # sqrt in Fp2 for p = 3 mod 4
        a1 = a ** ((BP - 3) // 4); al = a1 * a1 * a; x0 = a1 * a
        if al == F2([BP - 1, 0]): y = F2([0, 1]) * x0
        else: y = (al + F2.one()) ** ((BP - 1) // 2) * x0
        if y * y == a:
            Q = (x, y, F2.one())
            if not bn.is_inf(bn.multiply(Q, BN)): return Q
P, Q = bl_rg1(), bl_rg2()
case('bl_g1msm', msm1([(bl_nosub_g1(), 5)]), 96, n=1)
case('bl_g1add', [bl_g1(bl_nosub_g1()), bl_g1(P)], 96)
case('bl_g2msm', msm2([(bl_nosub_g2(), 5)]), 192, n=1)
case('bl_pair', bl_pairs([(bl_nosub_g1(), Q)]), 2, b'\xff\x00', n=1)
case('bl_pair', bl_pairs([(P, bl_nosub_g2())]), 2, b'\xff\x00', n=1)
case('bn_pair', bn_pairs([(bn_rg1(), bn_nosub_g2())]), 2, b'\xff\x00', n=1)

# ---------------- emit C ----------------
def carr(name, bs):
    return f'static const uint8_t {name}[{max(len(bs),1)}]={{' + ','.join(f'0x{x:02x}' for x in bs or b'\0') + '};'

L = ['#include "zkvm_accelerators.h"', '#include <stdbool.h>', '#include <stddef.h>',
     'static uint8_t T[131072]; static unsigned tn; static unsigned fails, first_fail=0xffffffff;',
     'static uint8_t ob[256]; static bool okb;',
     'static void rec(unsigned i, zkvm_status s, const uint8_t* o, unsigned n, const uint8_t* e) {',
     '  T[tn++]=(uint8_t)s; for(unsigned k=0;k<n;k++) T[tn++]=o[k];',
     '  if(e){ int d=(s!=0); for(unsigned k=0;k<n;k++) d|=o[k]^e[k]; if(d){fails++; if(first_fail==0xffffffff) first_fail=i;} } }',
     'static void rec2(unsigned i, zkvm_status s, bool ok, const uint8_t* e) {',
     '  T[tn++]=(uint8_t)s; T[tn++]=(uint8_t)ok;',
     '  if(e){ if((uint8_t)s!=e[0] || (uint8_t)ok!=e[1]){fails++; if(first_fail==0xffffffff) first_fail=i;} } }']
body = []
for i, (kind, ins, n, outlen, exp) in enumerate(cases):
    for j, x in enumerate(ins):
        L.append(carr(f'i{i}_{j}', x))
    if exp is not None:
        L.append(carr(f'e{i}', exp))
    e = f'e{i}' if exp is not None else '0'
    I = [f'(const void*)i{i}_{j}' for j in range(len(ins))]
    call = {
        'bn_add': lambda: f'zkvm_bn254_g1_add({I[0]},{I[1]},(void*)ob)',
        'bn_mul': lambda: f'zkvm_bn254_g1_mul({I[0]},{I[1]},(void*)ob)',
        'bn_pair': lambda: f'zkvm_bn254_pairing({I[0]},{n},&okb)',
        'bl_g1add': lambda: f'zkvm_bls12_g1_add({I[0]},{I[1]},(void*)ob)',
        'bl_g2add': lambda: f'zkvm_bls12_g2_add({I[0]},{I[1]},(void*)ob)',
        'bl_g1msm': lambda: f'zkvm_bls12_g1_msm({I[0]},{n},(void*)ob)',
        'bl_g2msm': lambda: f'zkvm_bls12_g2_msm({I[0]},{n},(void*)ob)',
        'bl_pair': lambda: f'zkvm_bls12_pairing({I[0]},{n},&okb)',
        'bl_map1': lambda: f'zkvm_bls12_map_fp_to_g1({I[0]},(void*)ob)',
        'bl_map2': lambda: f'zkvm_bls12_map_fp2_to_g2({I[0]},(void*)ob)',
        'kzg': lambda: f'zkvm_kzg_point_eval({I[0]},{I[1]},{I[2]},{I[3]},&okb)',
    }[kind]()
    body.append(f'  for(unsigned k=0;k<256;k++) ob[k]=0xAA; okb=0;')
    if outlen == 2:
        body.append(f'  {{ zkvm_status s={call}; rec2({i},s,okb,{e}); }}')
    else:
        body.append(f'  {{ zkvm_status s={call}; rec({i},s,ob,{outlen},{e}); }}')
L.append('int main(void){')
L += body
L += ['  uint8_t h[32]; zkvm_keccak256(T,tn,(void*)h);',
      '  volatile uint32_t *O=(volatile uint32_t*)(0xA0410000ULL);',
      f'  O[0]=fails; O[1]=first_fail; O[2]={len(cases)}; O[3]=tn;',
      '  for(unsigned k=0;k<8;k++) O[4+k]=(uint32_t)h[4*k]|((uint32_t)h[4*k+1]<<8)|((uint32_t)h[4*k+2]<<16)|((uint32_t)h[4*k+3]<<24);',
      '  return 0;}']
open(OUT, 'w').write('\n'.join(L) + '\n')
print(f'{len(cases)} cases, {sum(1 for c in cases if c[4] is not None)} with expected', file=sys.stderr)
