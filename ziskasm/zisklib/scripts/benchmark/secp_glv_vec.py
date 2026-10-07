#!/usr/bin/env python3
"""secp_glv_vec.py OUT.c: self-checking secp256k1 guest for the double-scalar
multiplication behind zkvm_secp256k1_ecrecover / zkvm_secp256k1_verify, with
expected results from Python. Covers the degenerate inputs of its 16-entry table
(R or the public key equal to +-G, +-phi(G), phi^2(G)), zero and equal scalars,
results at infinity or equal to +-G, R = c*G for small c (equal x coordinates
inside the ladder), and random recoveries.
Output: one byte per case at 0xa0410000 (1 = wrong), then 1 once all have run."""
import random, sys

P = 2**256 - 2**32 - 977
N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
G = (0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798,
     0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8)
BETA = 0x7AE96A2B657C07106E64479EAC3434E99CF0497512F58995C1396C28719501EE


def add(p, q):
    if p is None: return q
    if q is None: return p
    (x1, y1), (x2, y2) = p, q
    if x1 == x2:
        if (y1 + y2) % P == 0: return None
        l = 3 * x1 * x1 * pow(2 * y1, -1, P) % P
    else:
        l = (y2 - y1) * pow(x2 - x1, -1, P) % P
    x3 = (l * l - x1 - x2) % P
    return x3, (l * (x1 - x3) - y1) % P


def mul(k, p):
    r = None
    for b in bin(k % N)[2:] if k % N else "":
        r = add(r, r)
        if b == '1': r = add(r, p)
    return r


def neg(p): return None if p is None else (p[0], (-p[1]) % P)
def phi(p): return (BETA * p[0] % P, p[1])
be = lambda v: v.to_bytes(32, 'big')


def lift(x, parity):
    a = (x * x * x + 7) % P
    y = pow(a, (P + 1) // 4, P)
    if y * y % P != a: return None
    return (x, y if y % 2 == parity else P - y)


def recover(z, r, s, recid):
    """EF zkvm_secp256k1_ecrecover: pubkey or None (EFAIL)."""
    if not (0 < r < N and 0 < s < N) or recid > 1: return None
    R = lift(r, recid)
    if R is None: return None
    ri = pow(r, -1, N)
    return add(mul(-z * ri, G), mul(s * ri, R))


def sig_for(z, R):
    """(r, recid) of point R (which must have x < N)."""
    assert R[0] < N
    return R[0], R[1] & 1


cases = []  # ('rec', z, r, s, recid, expected point or None) / ('ver', z, r, s, pk, expected bool)
random.seed(4)


def rec(z, R, s):
    r, rid = sig_for(z, R)
    cases.append(('rec', z % 2**256, r, s, rid, recover(z, r, s, rid)))


def ver(z, r, s, pk):
    ok = False
    if 0 < r < N and 0 < s < N:
        si = pow(s, -1, N)
        X = add(mul(z * si, G), mul(r * si, pk))
        ok = X is not None and X[0] % N == r
    cases.append(('ver', z, r, s, pk, ok))


# R on the fixed half of the table: +-G, +-phi(G), phi^2(G) (x < N for all of them).
for R in (G, neg(G), phi(G), neg(phi(G)), phi(phi(G)), neg(phi(phi(G)))):
    if R[0] >= N: continue
    rec(random.randrange(1, N), R, random.randrange(1, N))
    rec(0, R, random.randrange(1, N))                      # u1 = 0
    rec(5, R, N - 5)                                       # s = -z: u1 = u2
    rec(7, R, 7)                                           # s = z: u2 = -u1
# R = c*G for small c: the result is (s*c - z)/r * G; pick s so that it is
# infinity, +-G, or small multiples (equal x coordinates in the ladder).
for c in range(2, 12):
    R = mul(c, G)
    if R[0] >= N: continue
    r = R[0]
    for target in (0, 1, -1, 2, 3, c, -c, 2 * c):
        z = random.randrange(1, N)
        s = (target * r + z) * pow(c, -1, N) % N          # (s*c - z)/r = target
        if s == 0: continue
        rec(z, R, s)
# random recoveries
for _ in range(40):
    while True:
        R = mul(random.randrange(1, N), G)
        if R[0] < N: break
    z = random.randrange(0, N)
    s = random.randrange(1, N)
    rec(z, R, s)
# verify with the public key on the fixed half of the table, valid and invalid
for pk in (G, neg(G), phi(G), mul(2, G)):
    for _ in range(3):
        k = random.randrange(1, N)
        Rk = mul(k, G)
        r = Rk[0] % N
        z = random.randrange(1, N)
        priv = 2 if pk == mul(2, G) else (1 if pk == G else (N - 1 if pk == neg(G) else None))
        if priv is None:  # phi(G) = lambda*G
            priv = 0x5363AD4CC05C30E0A5261C028812645A122E22EA20816678DF02967C1B23BD72
        s = pow(k, -1, N) * (z + r * priv) % N
        ver(z, r, s, pk)
        ver(z, r, (s + 1) % N, pk)

o = []
w = o.append
w('#include "zkvm_accelerators.h"')
w('static volatile uint8_t* const O = (volatile uint8_t*)0xA0410000ULL;')
w('typedef struct { uint8_t kind, recid, exp_ok; uint8_t z[32], r[32], s[32], pt[64]; } C;')
w('static const C K[] = {')
for c in cases:
    if c[0] == 'rec':
        _, z, r, s, rid, Q = c
        pt = (be(Q[0]) + be(Q[1])) if Q else bytes(64)
        kind, ok = 0, Q is not None
    else:
        _, z, r, s, pk, ok = c
        pt, rid, kind = be(pk[0]) + be(pk[1]), 0, 1
    arr = lambda b: '{' + ','.join(str(x) for x in b) + '}'
    w(f'  {{{kind},{rid},{int(ok)},{arr(be(z))},{arr(be(r))},{arr(be(s))},{arr(pt)}}},')
w('};')
w('''int main(void) {
    unsigned n = sizeof K / sizeof K[0], i;
    for (i = 0; i < n; i++) {
        const C* c = &K[i];
        zkvm_secp256k1_hash h; zkvm_secp256k1_signature sg; zkvm_secp256k1_pubkey pk;
        for (int j = 0; j < 32; j++) { h.data[j] = c->z[j]; sg.data[j] = c->r[j]; sg.data[32 + j] = c->s[j]; }
        int bad = 0;
        if (c->kind == 0) {
            zkvm_status st = zkvm_secp256k1_ecrecover(&h, &sg, c->recid, &pk);
            if ((st == ZKVM_EOK) != c->exp_ok) bad = 1;
            if (st == ZKVM_EOK) for (int j = 0; j < 64; j++) if (pk.data[j] != c->pt[j]) bad = 1;
        } else {
            for (int j = 0; j < 64; j++) pk.data[j] = c->pt[j];
            bool ok = 0;
            if (zkvm_secp256k1_verify(&h, &sg, &pk, &ok) != ZKVM_EOK || ok != c->exp_ok) bad = 1;
        }
        O[i] = (uint8_t)bad;
    }
    O[i] = 1;
    return 0;
}''')
open(sys.argv[1], 'w').write('\n'.join(o) + '\n')
print(len(cases), 'cases', file=sys.stderr)
