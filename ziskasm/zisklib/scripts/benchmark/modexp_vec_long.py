"""modexp_vec_long.py 'MODLENS' OUT.c: self-checking zkvm_modexp guest for long moduli
(odd -> Montgomery path, even -> general path), expected values from Python pow.
MODLENS is a Python list of modulus lengths, e.g. '[49,64,97,256,1024]'. Output as
modexp_vec.py."""
import random, sys
MLS = eval(sys.argv[1]); OUT = sys.argv[2]
random.seed(11)
def rnd(n, lz=0):
    b = bytes(random.randrange(256) for _ in range(n))
    return bytes(lz) + b if n else bytes(lz)
cases = []
def add(b, e, m):
    ib, ie, im = (int.from_bytes(x, 'big') for x in (b, e, m))
    r = 0 if im == 0 else pow(ib, ie, im)
    out = r.to_bytes(len(m), 'big') if len(m) else b''
    cases.append((b, e, m, out))

def odd(m, want):
    m = bytearray(m); m[-1] = (m[-1] | 1) if want else (m[-1] & 0xfe); return bytes(m)
for ml in MLS:
    for par in (1, 0):
        for el in ([1, 3, 8, 20] if par else [3]):
            if ml > 256 and el > 8: continue
            if not par and ml > 520: continue
            add(rnd(min(1056, random.choice([ml - 5, ml, ml + 40, 3]))), rnd(el), odd(rnd(ml, random.choice([0, 0, 2])), par))
add(rnd(256), bytes([1, 0, 1]), odd(rnd(256), 1))      # RSA verify shape
add(rnd(300), bytes(3), odd(rnd(100), 1))               # exp 0
add(b'', rnd(9), odd(rnd(70), 1))                       # base empty
add(bytes(80), rnd(9), odd(rnd(70), 1))                 # base 0
add(rnd(70), rnd(9), b'\xff' * 70)                     # all-ones modulus
add(rnd(70), rnd(9), bytes(5) + b'\x80' + bytes(62) + b'\x01')  # sparse modulus with leading zeros
def arr(name, b): return f"static const uint8_t {name}[{max(len(b),1)}] = {{{','.join(map(str, b)) or '0'}}};"
o = ['#include "zkvm_accelerators.h"']
for i, (b, e, m, r) in enumerate(cases):
    o += [arr(f'b{i}', b), arr(f'e{i}', e), arr(f'm{i}', m), arr(f'r{i}', r)]
o.append('static uint8_t out[1100];')
o.append('int main(void) { volatile uint8_t *O = (volatile uint8_t *)0xA0410000ULL;')
for i, (b, e, m, r) in enumerate(cases):
    o.append(f'  {{ for (int j = 0; j < 1100; j++) out[j] = 0xa5; int s = zkvm_modexp(b{i},{len(b)},e{i},{len(e)},m{i},{len(m)},out); int d = s;'
             f' for (int j = 0; j < {len(r)}; j++) d |= out[j] ^ r{i}[j]; d |= out[{len(r)}] ^ 0xa5; O[{i}] = (uint8_t)(d != 0); }}')
o.append(f'  O[{len(cases)}] = 1; return 0; }}')
open(OUT, 'w').write('\n'.join(o))
print(len(cases))
