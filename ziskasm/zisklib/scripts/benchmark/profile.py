#!/usr/bin/env python3
"""profile.py SYMS HIST [TOP]: fold a ziskemu -H pc histogram into per-routine totals.
SYMS is the library symbol map (`cargo run --release -p zisk-asm --example dump_syms`),
HIST the ziskemu output."""
import sys,re,bisect
syms=[]
for l in open(sys.argv[1]):
    a,n=l.split()
    if re.match(r'^(zisklib_|ziskasm_|bn254_|bls|bn_|b12_)',n) : syms.append((int(a,16),n))
syms.sort(); addrs=[a for a,_ in syms]
agg={}
tot=0
for l in open(sys.argv[2]):
    m=re.match(r'\s*([\d,]+)\s+[\d.]+%\s+0x([0-9a-f]+):',l)
    if not m: continue
    c=int(m.group(1).replace(',',''));pc=int(m.group(2),16)
    i=bisect.bisect_right(addrs,pc)-1
    n=syms[i][1] if i>=0 else '?'
    agg[n]=agg.get(n,0)+c; tot+=c
print('total',tot)
for n,c in sorted(agg.items(),key=lambda x:-x[1])[:int(sys.argv[3]) if len(sys.argv)>3 else 40]:
    print(f'{c:10d} {100*c/tot:6.2f}% {n}')
