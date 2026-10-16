#!/usr/bin/env python3
"""smul.py: unrolled G1 double-and-add loop (replaces <p>_general..<p>_done in curve.zisk).

k (4 LE limbs, already reduced, >= 3) is scanned MSB-first through r15: each of the
256 bit slots is `dbl ; lt(r15, 0) -> add path ; sll`, and the add path (out of
line) is `add ; sll ; jump next`. Entry skips the leading zeros and the MSB (Q = P)
with one computed jump through a 256-entry slot table; each 64-slot block starts by
loading its limb. ~4 steps/bit instead of ~12.
"""
def gen(curve):
    bn = curve == 'bn254'
    L, SM, PT = ('bsm', 'BN_SM_', 64) if bn else ('lsm', 'BLS_SM_', 96)
    pre = 'bn254_curve' if bn else 'bls12_381_curve'
    T = L.upper() + '_TAB'
    data = [f'u64 {SM}P[{PT // 8}] = ' + ', '.join(['0'] * (PT // 8)),
            f'const u64 {SM}HDRC[2] = {SM}Q, {SM}P']
    ent = []
    for t in range(256):
        ent.append(f'{L}_done' if t == 0 else (f'{L}_t{t}' if t % 64 == 0 else f'{L}_s{t}'))
    data.append(f'const u64 {T}[256] = ' + ', '.join(ent))
    c = [f'{L}_general:',
         f'\tcopyb(0, [{SM}PPTR]) -> r5',
         f'\tdma_xmemcpy({SM}P, r5) -> r5, j({PT}, 4)',
         f'\tdma_xmemcpy({SM}Q, {SM}P) -> r5, j({PT}, 4)']
    for j, base in ((3, 0), (2, 64), (1, 128)):
        c += [f'\tcopyb(0, [{SM}K + {8 * j}]) -> r15', f'\tcopyb(0, {base}) -> r13', f'\tltu(0, r15), j({L}_lz)']
    c += [f'\tcopyb(0, [{SM}K]) -> r15', '\tcopyb(0, 192) -> r13',
          f'{L}_lz:', f'\tlt(r15, 0), j({L}_msb)', '\tsll(r15, 1) -> r15', '\tadd(r13, 1) -> r13', f'\tjump({L}_lz)',
          f'{L}_msb:', '\tsll(r15, 1) -> r15', '\tadd(r13, 1) -> r13', f'\teq(r13, 256), j({L}_done)',
          '\tsll(r13, 3) -> r13', f'\tadd(r13, {T}) -> r13', '\tcopyb(r13, 8[a + 0]), setpc(0)']
    nxt = lambda t: f'{L}_done' if t == 256 else (f'{L}_t{t}' if t % 64 == 0 else f'{L}_s{t}')
    for t in range(1, 256):
        if t % 64 == 0:
            c += [f'{L}_t{t}:', f'\tcopyb(0, [{SM}K + {8 * (3 - t // 64)}]) -> r15']
        c += [f'{L}_s{t}:', f'\t{pre}_dbl(0, {SM}Q)', f'\tlt(r15, 0), j({L}_a{t})', '\tsll(r15, 1) -> r15']
    c += [f'\tjump({L}_done)']
    for t in range(1, 256):
        c += [f'{L}_a{t}:', f'\t{pre}_add(0, {SM}HDRC)', '\tsll(r15, 1) -> r15', f'\tjump({nxt(t + 1)})']
    return data, c

def patch(text, curve):
    L = 'bsm' if curve == 'bn254' else 'lsm'
    data, code = gen(curve)
    a = text.index(f'{L}_general:\n')
    b = text.index(f'{L}_done:\n')
    text = text[:a] + '\n'.join(code) + '\n' + text[b:]
    # data goes before the routine's entry label
    e = text.index(f'zisklib_scalar_mul_{curve}:\n')
    return text[:e] + '\n'.join(data) + '\n\n' + text[e:]
