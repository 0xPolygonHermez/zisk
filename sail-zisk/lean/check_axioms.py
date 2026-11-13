#!/usr/bin/env python3
"""Check the axioms a theorem rests on against an allowlist.

Reads the output of Lean's `#print axioms` (a file, or stdin) and fails if
the theorem depends on an axiom that is neither in the allowlist nor one of
bv_decide's (`..._native.bv_decide.ax_...`, which trust its compiled
checker). `sorryAx`, a proof left unfinished, is never in the allowlist.

  check_axioms.py lean/axioms.txt build/axioms.out
"""

import re
import sys


def main():
    allowed = set()
    with open(sys.argv[1]) as f:
        for line in f:
            line = line.split('#', 1)[0].strip()
            if line:
                allowed.add(line)
    text = open(sys.argv[2]).read() if len(sys.argv) > 2 else sys.stdin.read()

    m = re.search(r"'([^']+)' depends on axioms: \[([^\]]*)\]", text)
    if not m:
        sys.exit(f"no `#print axioms` output found:\n{text}")
    name = m.group(1)
    axioms = [a.strip() for a in m.group(2).split(',') if a.strip()]
    native = [a for a in axioms if '._native.bv_decide.ax_' in a]
    other = [a for a in axioms if a not in native and a not in allowed]
    if other:
        print(f"{name} depends on axioms outside {sys.argv[1]}:", file=sys.stderr)
        for a in other:
            print(f"  {a}", file=sys.stderr)
        sys.exit(1)
    print(f"{name}: {len(axioms) - len(native)} axioms of the allowlist, "
          f"{len(native)} of bv_decide's")


if __name__ == '__main__':
    main()
