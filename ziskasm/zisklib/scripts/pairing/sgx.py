#!/usr/bin/env python3
"""sgx.py: unroll the BLS12-381 G1 subgroup-check multiply by the constant (x²-1)/3.

Replaces lsg_x2_go..(ret after lsg_x2_done) in subgroup.zisk. Same complete
semantics as the dbl_complete/add_complete loop it replaces: r15 = 1 while R is
the identity (then doublings are skipped and an add sets R = P). E(Fp) has odd
order (no 2-torsion), so a doubling never yields the identity; an add yields it
only for R = -P. Rare equal-x / identity paths are out of line.
"""
import re
def patch(text):
    bits = [int(x) for x in re.search(r'const u64 BLS_X2DIV3_BE\[126\] = ([0-9, ]+)', text).group(1).split(',')]
    assert len(bits) == 126 and bits[0] == 1
    R, PS = 'BLS_SG1_R', 'BLS_SG1_PS'
    c = ['lsg_x2_go:',
         '\tcopyb(0, [BLS_SGP]) -> r5',
         f'\tdma_xmemcpy({PS}, r5) -> r5, j(96, 4)',
         f'\tdma_xmemcpy({R}, {PS}) -> r5, j(96, 4)',
         '\tcopyb(0, 0) -> r15']
    rare = []
    for t in range(1, 126):
        c += [f'\tltu(0, r15), j(lsx_d{t})', f'\tbls12_381_curve_dbl(0, {R})', f'lsx_d{t}:']
        if bits[t]:
            c += [f'\tltu(0, r15), j(lsx_i{t})',
                  f'\tdma_xmemcmp({R}, {PS}) -> r5, j(48, 4)',
                  f'\teq(r5, 0), j(lsx_x{t})',
                  '\tbls12_381_curve_add(0, BLS_SG1_HRP)',
                  f'lsx_n{t}:']
            rare += [f'lsx_i{t}:', f'\tdma_xmemcpy({R}, {PS}) -> r5, j(96, 4)', '\tcopyb(0, 0) -> r15', f'\tjump(lsx_n{t})',
                     f'lsx_x{t}:', f'\tdma_xmemcmp({R} + 48, {PS} + 48) -> r5, j(48, 4)', f'\teq(r5, 0), j(lsx_y{t})',
                     '\tcopyb(0, 1) -> r15', f'\tdma_xmemset({R}, 96) -> r5, j(0, 4)', f'\tjump(lsx_n{t})',
                     f'lsx_y{t}:', f'\tbls12_381_curve_dbl(0, {R})', f'\tjump(lsx_n{t})']
    c += ['\tcopyb(0, [BLS_SGR]) -> r5', f'\tdma_xmemcpy(r5, {R}) -> r5, j(96, 4)', '\tpop r1', '\tret'] + rare
    a = text.index('lsg_x2_go:\n')
    d = text.index('lsg_x2_done:\n')
    e = text.index('\tret\n', d) + len('\tret\n')
    text = text[:a] + '\n'.join(c) + '\n' + text[e:]
    f = text.index('zisklib_scalar_mul_by_x2div3_complete_bls12_381:\n')
    data = [f'u64 {PS}[12] = ' + ', '.join(['0'] * 12), f'const u64 BLS_SG1_HRP[2] = {R}, {PS}', '']
    return text[:f] + '\n'.join(data) + text[f:]
