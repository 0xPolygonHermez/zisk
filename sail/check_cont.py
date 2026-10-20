#!/usr/bin/env python3
"""Check the continuation buses' census against lean/Proofs/MemCont.lean.

MemCont.lean takes buses 11 and 12 as sound over the senders it lists:
`contSenders` (bus 11) is every RAM instance's `MemOnMem.cont` and every
InputData instance's `InputDataOnMem.cont`; `rdContSenders` (bus 12) is every
RomData instance's `RomDataOnMem.cont`. That is right only if no other
argument of any AIR sends on those buses. This script reads every argument of
every generated AIR (build/pil/Pil/*.lean), collects the ones on buses 11 and
12, and checks they are exactly the ones the three `cont` definitions name.

Usage: check_cont.py   (from sail/; `make cont`)
"""

import glob
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PIL = os.path.join(HERE, 'build', 'pil', 'Pil')
CONT = os.path.join(HERE, 'lean', 'Proofs', 'MemCont.lean')

# The `cont` definition of each AIR's instance structure, and its bus.
CONT_DEFS = {
    'MemOnMem.cont': ('Mem', 11),
    'InputDataOnMem.cont': ('InputData', 11),
    'RomDataOnMem.cont': ('RomData', 12),
}


def census():
    """Every argument on bus 11 or 12: {bus: {(air, arg)}}."""
    found = {11: set(), 12: set()}
    for path in sorted(glob.glob(os.path.join(PIL, '*.lean'))):
        air = os.path.basename(path)[:-len('.lean')]
        src = open(path).read()
        for m in re.finditer(r'def (bus\d+) : BusArg F \(Ctx F\) \(Trace F\) where\n(.*?)\n\n', src, re.S):
            ops = re.search(r'opids := \[([\d, ]*)\]', m.group(2))
            if not ops:
                sys.exit(f'{air}.{m.group(1)}: no opids')
            for b in (int(x) for x in ops.group(1).split(',') if x.strip()):
                if b in found:
                    found[b].add((air, m.group(1)))
    return found


def listed():
    """The arguments MemCont.lean's `cont` definitions name: {bus: {(air, arg)}}."""
    src = open(CONT).read()
    out = {11: set(), 12: set()}
    for name, (air, bus) in CONT_DEFS.items():
        m = re.search(r'def ' + re.escape(name) + r' \(I : \w+ F\) : List \(Sender F\) :=\n(.*?)\n\n', src, re.S)
        if not m:
            sys.exit(f'MemCont.lean: no definition of {name}')
        args = re.findall(re.escape(air) + r'\.(bus\d+)', m.group(1))
        if not args:
            sys.exit(f'MemCont.lean: {name} names no argument of {air}')
        out[bus] |= {(air, a) for a in args}
    return out


def main():
    if not os.path.isdir(PIL):
        sys.exit('build/pil is missing — run make pil')
    found, named = census(), listed()
    ok = True
    for bus in (11, 12):
        for air, arg in sorted(found[bus] - named[bus]):
            print(f'bus {bus}: {air}.{arg} sends, but MemCont.lean does not list it')
            ok = False
        for air, arg in sorted(named[bus] - found[bus]):
            print(f'bus {bus}: MemCont.lean lists {air}.{arg}, which does not send on it')
            ok = False
        if ok:
            print(f'bus {bus}: ' + ', '.join(f'{a}.{g}' for a, g in sorted(found[bus])))
    if not ok:
        sys.exit(1)
    print('OK: the continuation buses carry exactly the arguments MemCont.lean lists')


if __name__ == '__main__':
    main()
