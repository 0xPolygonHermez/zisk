#!/usr/bin/env python3
"""Write a ZisK program that runs every precompile on random operands.

Each call's operands are data of their own; after the call, the program loads
every word of the result into a register, r12 on, so the difftest compares
each result word with ziskemu's, register by register, and add256's carry is
c itself. The operands are what the operation accepts: field elements are
canonical (ed25519_add's need not be: it reduces them, so they are any 256
bits), moduli nonzero, carries 0 or 1. Curve operations get random field
elements rather than points: the formulas compute the same on any operands.

Usage: gen_precompile_test.py <out.zisk> [vectors per operation] [seed]
    (`make difftest-all`, as test/programs/precompiles)
"""

import random
import sys

SECP256K1_P = 2**256 - 2**32 - 977
SECP256R1_P = 0xffffffff00000001000000000000000000000000ffffffffffffffffffffffff
BN254_P = 21888242871839275222246405745257275088696311157297823662689037894645226208583
BN254_R = 21888242871839275222246405745257275088548364400416034343698204186575808495617
BLS12_381_P = 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab
KOALABEAR_P = 2130706433
MASK64 = 2**64 - 1

# Precompiles that take two operand blocks at a and b and write over a:
# (op, words at a, words at b, words written over a, modulus of each half or None).
AB = [
    ('sha256', 4, 8, 4, None),
    ('blake2s', 8, 8, 8, None),
    ('blake3', 8, 8, 8, None),
    ('secp256k1_add', 8, 8, 8, SECP256K1_P),
    ('secp256r1_add', 8, 8, 8, SECP256R1_P),
    ('bn254_curve_add', 8, 8, 8, BN254_P),
    ('bn254_complex_add', 8, 8, 8, BN254_P),
    ('bn254_complex_sub', 8, 8, 8, BN254_P),
    ('bn254_complex_mul', 8, 8, 8, BN254_P),
    ('bls12_381_curve_add', 12, 12, 12, BLS12_381_P),
    ('bls12_381_complex_add', 12, 12, 12, BLS12_381_P),
    ('bls12_381_complex_sub', 12, 12, 12, BLS12_381_P),
    ('bls12_381_complex_mul', 12, 12, 12, BLS12_381_P),
    ('babyjubjub_add', 8, 8, 8, BN254_R),
    ('ed25519_add', 8, 8, 8, 2**256),
]

# Precompiles that work in place on one block at b: (op, words, modulus or None).
IN_PLACE = [
    ('keccak', 25, None),
    ('poseidon2', 16, None),
    ('poseidon1', 16, None),
    ('secp256k1_dbl', 8, SECP256K1_P),
    ('secp256r1_dbl', 8, SECP256R1_P),
    ('bn254_curve_dbl', 8, BN254_P),
    ('bls12_381_curve_dbl', 12, BLS12_381_P),
]


class Program:
    def __init__(self):
        self.data, self.code, self.n = [], [], 0

    def block(self, words):
        """A data block of these words; returns its label."""
        self.n += 1
        label = f'V{self.n}'
        rows = ',\n\t'.join(', '.join(f'0x{w:016x}' for w in words[i:i + 4]) for i in range(0, len(words), 4))
        self.data.append(f'u64 {label}[{len(words)}] =\n\t{rows}\n')
        return label

    def load(self, label, words):
        """Load the words of a block into r12 on."""
        assert words <= 28
        self.code.append(f'\tcopyb(0, {label}) -> r4')
        for i in range(words):
            self.code.append(f'\tcopyb(r4, 8[a + {8 * i}]) -> r{12 + i}')

    def table(self, entries):
        """A table of operand addresses and values at a block; returns its label."""
        label = self.block([0] * len(entries))
        self.code.append(f'\tcopyb(0, {label}) -> r5')
        for i, e in enumerate(entries):
            self.code.append(f'\tcopyb(0, {e}) -> r6')
            self.code.append(f'\tcopyb(r5, r6) -> 8[a + {8 * i}]')
        return label


def limbs(x, n):
    return [(x >> (64 * i)) & MASK64 for i in range(n)]


def elements(rng, modulus, words, halves=2):
    """`halves` field elements below modulus, words / halves words each."""
    per = words // halves
    return [w for _ in range(halves) for w in limbs(rng.randrange(modulus), per)]


def main():
    out = sys.argv[1]
    count = int(sys.argv[2]) if len(sys.argv) > 2 else 8
    rng = random.Random(int(sys.argv[3]) if len(sys.argv) > 3 else 1)
    p = Program()
    p.code.append('main:')

    for _ in range(count):
        for op, wa, wb, wout, modulus in AB:
            if modulus is None:
                a, b = [rng.getrandbits(64) for _ in range(wa)], [rng.getrandbits(64) for _ in range(wb)]
            else:
                a, b = elements(rng, modulus, wa), elements(rng, modulus, wb)
            la, lb = p.block(a), p.block(b)
            p.code.append(f'\t{op}({la}, {lb}) -> r7')
            p.load(la, wout)

        for op, words, modulus in IN_PLACE:
            if modulus is None:
                v = [rng.getrandbits(64) for _ in range(words)]
                if op.startswith('poseidon'):
                    # Some words at or above Goldilocks's p, which the
                    # permutation reduces.
                    v[rng.randrange(words)] = MASK64 - rng.randrange(2**32)
            else:
                v = elements(rng, modulus, words)
            lb = p.block(v)
            p.code.append(f'\t{op}(0, {lb}) -> r7')
            p.load(lb, words)

        # poseidon2_koalabear: 16 canonical elements, two per word.
        lanes = [rng.randrange(KOALABEAR_P) for _ in range(16)]
        lb = p.block([lanes[2 * i] | (lanes[2 * i + 1] << 32) for i in range(8)])
        p.code.append(f'\tposeidon2_koalabear(0, {lb}) -> r7')
        p.load(lb, 8)

        # blake2b: the round index in the extended argument, mod 10.
        la = p.block([rng.getrandbits(64) for _ in range(16)])
        lb = p.block([rng.getrandbits(64) for _ in range(16)])
        p.code.append(f'\tblake2b({la}, {lb}) -> r7, j({rng.randrange(100)}, 4)')
        p.load(la, 16)

        # add256: [x, y, cin, z]; c is the carry out.
        x, y, z = (p.block([rng.getrandbits(64) for _ in range(4)]) for _ in range(3))
        t = p.table([x, y, rng.randrange(2), z])
        p.code.append(f'\tadd256(0, {t}) -> r7')
        p.load(z, 4)

        # arith256: [x, y, z, dl, dh].
        x, y, z, dl, dh = (p.block([rng.getrandbits(64) for _ in range(4)]) for _ in range(5))
        t = p.table([x, y, z, dl, dh])
        p.code.append(f'\tarith256(0, {t}) -> r7')
        p.load(dl, 4)
        p.load(dh, 4)

        # arith256_mod and arith384_mod: [x, y, z, m, d], m nonzero, of any size.
        for op, n in (('arith256_mod', 4), ('arith384_mod', 6)):
            x, y, z, d = (p.block([rng.getrandbits(64) for _ in range(n)]) for _ in range(4))
            m = p.block(limbs(rng.randrange(1, 2**rng.choice([8, 64, 64 * n])), n))
            t = p.table([x, y, z, m, d])
            p.code.append(f'\t{op}(0, {t}) -> r7')
            p.load(d, n)

    p.code.append('\tret')
    with open(out, 'w') as f:
        f.write('; Generated by test/gen_precompile_test.py. Do not edit.\n'
                f'; Every precompile on {count} random operands each; the results in r12 on.\n\n')
        f.write('\n'.join(p.data))
        f.write('\n')
        f.write('\n'.join(p.code))
        f.write('\n')
    print(f'wrote {out}: {count} vectors for each of {len(AB) + len(IN_PLACE) + 6} precompiles')


if __name__ == '__main__':
    main()
