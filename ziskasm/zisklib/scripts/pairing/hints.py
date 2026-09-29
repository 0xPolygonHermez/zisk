#!/usr/bin/env python3
"""hints.py file: replace the looped line-coefficient hint readers with unrolled ones."""
import re, sys
p = sys.argv[1]
s = open(p).read()
c = 'bls12_381' if 'bls12_381' in p else 'bn254'
n, buf, fc = (24, 'BLS_ML_LAMMU', 'FCALL_BLS12_381_TWIST_') if c == 'bls12_381' else (16, 'ML_LAMMU', 'FCALL_BN254_TWIST_')
def body(name, params, fcall):
    L = [f'zisklib_ml_{name}_coeffs_{c}:']
    for r in params:
        L.append(f'\tfcall_param({n}, {r})')
    L.append(f'\tfcall({fcall}, 0)')
    for i in range(n - 1):
        L.append(f'\tfcall_get(0, [FREE_INPUT]) -> [{buf}' + (f' + {8*i}]' if i else ']'))
    L.append(f'\tcopyb(0, [FREE_INPUT]) -> [{buf} + {8*(n-1)}]')
    L.append('\tret')
    return '\n'.join(L) + '\n'
for name, params, fcall in (('dbl', ['r10'], fc + 'DBL_LINE_COEFFS'), ('add', ['r10', 'r11'], fc + 'ADD_LINE_COEFFS')):
    pat = re.compile(rf'^zisklib_ml_{name}_coeffs_{c}:\n.*?\n\tret\n', re.S | re.M)
    # the routine ends at the ret after its read-loop exit label
    m = re.search(rf'^zisklib_ml_{name}_coeffs_{c}:\n(?:.*\n)*?\w+_done:\n\tret\n', s, re.M)
    assert m, name
    s = s[:m.start()] + body(name, params, fcall) + s[m.end():]
open(p, 'w').write(s)
