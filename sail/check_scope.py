#!/usr/bin/env python3
"""Check a program against the scope of the proofs' runs.

Reads the ROM tools/sail-difftest exports (build/difftest/<prog>/rom.sail) and
checks every instruction against `MemProgEnd` (lean/Proofs/MemEnd.lean), the
scope of `run_to_end_memory`:

* the op is one of the six modeled ones (copyb, flag, add, ltu, eq, and),
  pubout (given `PubProved`: every prover on the operation bus proves it
  correctly), or halt, which Main cannot execute (`halt_impossible`);
* source a is immediate, c, a register x1-x31, or memory at a 32-bit address;
* source b is immediate, c, a register x1-x31, memory at a 32-bit address, or
  indirect of width 8 with a 32-bit offset;
* the store is none, a register x1-x31, or memory (direct, or indirect of
  width 8) with an offset below 2^32;
* set_pc only with a jmp_off1 of at most 31 bits either way;
* both jumps land in the 32-bit pc range;
* an end instruction makes no memory access;
* the boot instruction, at ROM_ENTRY (0x1000), loads nothing from memory
  (`BootNoLoad`, lean/Proofs/MemSystem.lean): Main's first lane runs it, and
  its loads would come before the program's data reaches the RAM.

Prints each instruction outside the scope, and why. Exit status 1 if any.

Usage: check_scope.py [build/difftest/<prog>/rom.sail]
"""

import re
import sys

ROM_ENTRY = 0x1000

OPS = {'ZOP_COPYB', 'ZOP_FLAG', 'ZOP_ADD', 'ZOP_LTU', 'ZOP_EQ', 'ZOP_AND', 'ZOP_HALT', 'ZOP_PUBOUT'}
ENTRY = re.compile(r"(0x[0-9a-f_]+) => struct \{([^}]*)\}", re.S)
FIELD = re.compile(r"(\w+) = ([^,]+?)(?:,|$)")


def num(s):
    return int(s.replace('_', ''), 16) if s.startswith('0x') else int(s)


def signed(v):
    return v - (1 << 64) if v >= 1 << 63 else v


def check(pc, f):
    why = []
    op = f['op'].rstrip('()')
    if op not in OPS:
        why.append(f'op {op} not modeled')
    a, b, st = f['a_src'], f['b_src'], f['store']
    a_imm, b_imm, off = num(f['a_imm']), num(f['b_imm']), num(f['store_off'])
    w = int(f['ind_width'])
    if a == 'A_REG' and not 0 < a_imm < 32:
        why.append('a register index outside x1-x31')
    if a == 'A_MEM' and a_imm >= 1 << 32:
        why.append('a memory address beyond 32 bits')
    if a == 'A_STEP':
        why.append('a source A_STEP')
    if b == 'B_REG' and not 0 < b_imm < 32:
        why.append('b register index outside x1-x31')
    if b in ('B_MEM', 'B_IND') and b_imm >= 1 << 32:
        why.append('b memory offset beyond 32 bits')
    if b == 'B_IND' and w != 8:
        why.append(f'indirect b of width {w}')
    if st == 'ST_REG' and not 0 < off < 32:
        why.append('store register index outside x1-x31')
    if st in ('ST_MEM', 'ST_IND') and off >= 1 << 32:
        why.append('store offset beyond 32 bits')
    if st == 'ST_IND' and w != 8:
        why.append(f'indirect store of width {w}')
    j1, j2 = signed(num(f['jmp_off1'])), signed(num(f['jmp_off2']))
    if f['set_pc'] == 'true' and not -(1 << 31) <= j1 < 1 << 31:
        why.append('set_pc with a long jmp_off1')
    for j in (j1, j2):
        if not 0 <= pc + j < 1 << 32:
            why.append('a jump outside the 32-bit pc range')
    if f['is_end'] == 'true' and (a == 'A_MEM' or b in ('B_MEM', 'B_IND')
                                  or st in ('ST_MEM', 'ST_IND')):
        why.append('an end instruction with a memory access')
    if pc == ROM_ENTRY and (a == 'A_MEM' or b in ('B_MEM', 'B_IND')):
        why.append('the boot instruction loads from memory')
    return why


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else 'build/difftest/doubler-min/rom.sail'
    text = open(path).read()
    bad = 0
    n = 0
    for m in ENTRY.finditer(text):
        pc = num(m.group(1))
        f = {k: v.strip() for k, v in FIELD.findall(m.group(2))}
        n += 1
        why = check(pc, f)
        if why:
            bad += 1
            print(f'{pc:#010x}  {f["op"]:<14} ' + '; '.join(why))
    if not any(num(m.group(1)) == ROM_ENTRY for m in ENTRY.finditer(text)):
        bad += 1
        print(f'no instruction at the boot pc {ROM_ENTRY:#x}')
    print(f'{n} instructions, {bad} outside the scope of run_to_end_memory')
    sys.exit(1 if bad else 0)


if __name__ == '__main__':
    main()
