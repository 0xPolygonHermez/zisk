#!/usr/bin/env python3
"""strip.py file routine... : drop the named zisklib_ routines (label to next routine) in place."""
import re, sys
p, names = sys.argv[1], set(sys.argv[2:])
lines = open(p).read().split('\n')
out, skip = [], False
for i, l in enumerate(lines):
    m = re.match(r'^(zisklib_\w+):$', l)
    if m:
        skip = m.group(1) in names
        if skip:
            # drop the routine's leading comment block too
            while out and out[-1].startswith(';') and not out[-1].startswith('; ===='):
                out.pop()
    if not skip:
        out.append(l)
open(p, 'w').write('\n'.join(out))
