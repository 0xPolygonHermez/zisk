#!/usr/bin/env python3
"""Checks the output of zkvm_random_guest.c against SHAKE256 of the seed.

usage: zkvm_random_check.py <seed> <out.bin>

<seed> is the 32 seed bytes as all 64 hex digits (optionally 0x-prefixed), as ziskemu
prints them.
"""
import hashlib
import struct
import sys

N, SHOWN = 500, 20

seed = bytes.fromhex(sys.argv[1].removeprefix("0x").removeprefix("0X"))
assert len(seed) == 32, "the seed is 32 bytes (64 hex digits)"
stream = hashlib.shake_256(seed).digest(8 * N)
exp = list(struct.unpack(f"<{N}Q", stream))
xor = 0
for v in exp:
    xor ^= v

out = open(sys.argv[2], "rb").read()
got = list(struct.unpack(f"<{SHOWN + 2}Q", out[: 8 * (SHOWN + 2)]))
ok = got[:SHOWN] == exp[:SHOWN] and got[SHOWN] == xor and got[SHOWN + 1] == 0
for i in range(SHOWN):
    mark = "" if got[i] == exp[i] else "   <-- expected %016x" % exp[i]
    print(f"  [{i:2}] {got[i]:016x}{mark}")
print(f"  xor  {got[SHOWN]:016x} (expected {xor:016x})")
print(f"  checks {got[SHOWN + 1]} (expected 0)")
print("PASS" if ok else "FAIL")
sys.exit(0 if ok else 1)
