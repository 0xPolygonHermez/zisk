"""modexp_vec.py [OUT.c]: self-checking zkvm_modexp guest (moduli up to 64 bytes and
edge cases), expected values from Python pow. Output byte i at 0xa0410000 = 1 if case
i failed; the byte after the last case is 1."""
import random, sys
random.seed(7)
def rnd(n, lz=0):
    b = bytes(random.randrange(256) for _ in range(n))
    return bytes(lz) + b if n else bytes(lz)
cases = []
def add(b, e, m):
    ib, ie, im = (int.from_bytes(x, 'big') for x in (b, e, m))
    r = 0 if im == 0 else pow(ib, ie, im)
    out = r.to_bytes(len(m), 'big') if len(m) else b''
    cases.append((b, e, m, out))
for ml in [1, 2, 7, 8, 9, 16, 31, 32, 33, 40, 47, 48, 49, 56, 64]:
    for _ in range(6):
        bl = random.choice([0, 1, 5, 31, 32, 33, 64, 65, 100, 200])
        el = random.choice([0, 1, 2, 8, 19, 32, 33])
        add(rnd(bl, random.choice([0, 0, 3])), rnd(el, random.choice([0, 0, 2])), rnd(ml, random.choice([0, 0, 1, 5, 20])))
add(rnd(32), rnd(32), bytes(32))                    # mod 0
add(rnd(32), rnd(32), bytes(31) + b'\x01')          # mod 1
add(rnd(32), bytes(4), rnd(32))                     # exp 0
add(b'', rnd(8), rnd(32))                           # base empty
add(bytes(40), rnd(8), rnd(48))                     # base 0
add(rnd(32), b'', rnd(20))                          # exp empty
add(rnd(32), rnd(32), b'\xff' * 32)                 # mod 2^256 - 1
add(rnd(48), rnd(48), b'\xff' * 48)
add(b'\xff' * 32, b'\xff' * 32, b'\xff' * 31 + b'\xfe')
add(rnd(300), rnd(64), rnd(33))
add(rnd(3), b'\x80', rnd(32))                       # single top-bit exponent
add(rnd(3), b'\x01', rnd(32))
add(rnd(3), b'\x00\x01', rnd(48))
for ml in (32, 48):
    for e in (b'\x07' + bytes(7), b'\x70\x00\x0f\xf0\x00\x01\x10\x00', b'\x01' + rnd(6), b'\x01' + rnd(7),
              b'\x00\x00\x0f' + rnd(9), b'\xf0' + bytes(8) + b'\x0f', b'\x10' * 12, rnd(7), rnd(8)):
        add(rnd(ml + 3), e, rnd(ml))
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
open(sys.argv[1] if len(sys.argv) > 1 else 'modexp_vec.c', 'w').write('\n'.join(o))
print(len(cases))
