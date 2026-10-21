#!/usr/bin/env python3
"""Compare a difftest's two runs: ziskemu.trace, the reference, with
sail.trace, the model's C emulator's, whose stderr is sail.err and exit
status sail.status (`make difftest`).

The runs agree when the traces are equal and the model ended normally; or,
when ziskemu stopped on a fault — a step it refuses, its trace's last line
`fault <message>` — when the model ran the same steps before it and then
stopped on an assertion, as it does on what ZisK refuses.

With --expect-fault, the runs must stop on a fault: a test of what ZisK
refuses.

Usage: compare_traces.py <difftest directory> [--expect-fault]
"""

import difflib
import os
import re
import sys


def main():
    d = sys.argv[1]
    ref = open(os.path.join(d, 'ziskemu.trace')).read().splitlines()
    got = open(os.path.join(d, 'sail.trace')).read().splitlines()
    err = open(os.path.join(d, 'sail.err')).read()
    status = int(open(os.path.join(d, 'sail.status')).read().strip() or 0)
    fault = ref[-1][len('fault '):] if ref and ref[-1].startswith('fault ') else None
    expected = ref[:-1] if fault is not None else ref
    steps = sum(1 for l in expected if re.match(r'\d+ ', l))

    if got != expected:
        sys.stdout.writelines(list(difflib.unified_diff(
            [l + '\n' for l in expected], [l + '\n' for l in got], 'ziskemu.trace', 'sail.trace'))[:40])
        sys.exit(f'FAIL: the model diverges from ziskemu (traces in {d})')
    asserted = re.search(r'^Assertion failed: (.*)$', err, re.M)
    if '--expect-fault' in sys.argv and fault is None:
        sys.exit(f'FAIL: ziskemu runs the program to its end; it should refuse it (traces in {d})')
    if fault is not None:
        if status == 0 or not asserted:
            sys.exit(f'FAIL: ziskemu stops at step {steps} ({fault}), but the model does not (traces in {d})')
        print(f'OK: {steps} steps agree with ziskemu, then both stop: ziskemu panics '
              f'({fault[:100]}), the model asserts ({asserted.group(1)})')
        return
    if status != 0:
        sys.exit(f'FAIL: the model stops after {steps} steps, ziskemu does not: '
                 f'{asserted.group(1) if asserted else err.strip()[:200]} (traces in {d})')
    print(f'OK: {steps} steps and the output agree with ziskemu')


if __name__ == '__main__':
    main()
