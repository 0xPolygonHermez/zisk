#!/usr/bin/env python3
"""tc.py - tower compiler: straight-line Fp2 kernels for the bn254 / bls12_381 towers.

A kernel is written as Fp2-level SSA (with CSE). Every Fp2 op is one precompile
f1 = f1 <op> f2 that updates f1 in place, so the compiler
  * runs an op in place on an operand that dies there (the first operand, or
    either one for add/mul), else copies the first operand into a fresh
    location with one DMA op;
  * gives each resulting chain of in-place ops one location: the output
    component it ends in (written directly through the result pointer) or a
    static temp slot (interval-allocated);
  * passes the two pointers to the precompile directly (a = the in-place
    operand, b = the other one): a static slot as an immediate, a dynamic one
    (argument pointer + offset) through r14 for a / r5 for b, each kept while
    it stays valid (r5 is clobbered by every DMA op).
Result pointers are guarded against aliasing an input pointer: if equal, the
input is first copied into a private buffer.
"""

CURVES = {
    'bn254': dict(sz=64, fp=32, pre='bn254_complex', xi=(9, 1), pfx='BNK',
                  neg='BN_F2_NEGMUL', p=0x30644E72E131A029B85045B68181585D97816A916871CA8D3C208C16D87CFD47),
    'bls12_381': dict(sz=96, fp=48, pre='bls12_381_complex', xi=(1, 1), pfx='BLK',
                      neg='BLS_F2_NEGMUL',
                      p=0x1A0111EA397FE69A4B1BA7B6434BACD764774B84F38512BF6730D2A0F6B0F6241EABFFFEB153FFFFB9FEFFFFFFFFAAAA),
}
COMM = {'add', 'mul'}


def limbs(x, n):
    return [(x >> (64 * i)) & (2**64 - 1) for i in range(n)]


class Kernel:
    def __init__(self, tw, name, args, doc):
        """args: list of (argname, reg, ncomp, 'in'|'out')."""
        self.tw, self.name, self.doc = tw, name, doc
        self.args = args
        self.defs = []            # vid -> ('in', arg, idx) | ('const', sym) | (op, x, y)
        self.cse = {}
        self.outs = []            # (arg, idx, vid)

    # ---- value constructors ----
    def _new(self, d):
        if d in self.cse:
            return self.cse[d]
        self.defs.append(d)
        v = len(self.defs) - 1
        self.cse[d] = v
        return v

    def inp(self, arg, idx):
        return self._new(('in', arg, idx))

    def const(self, sym):
        return self._new(('const', sym))

    def op(self, o, x, y):
        if o in COMM and x > y:
            x, y = y, x
        return self._new((o, x, y))

    def add(self, x, y): return self.op('add', x, y)
    def sub(self, x, y): return self.op('sub', x, y)
    def mul(self, x, y): return self.op('mul', x, y)
    def dbl(self, x): return self.op('add', x, x)
    def sq(self, x): return self.op('mul', x, x)
    def cmul(self, x, name): return self.mul(x, self.const(self.tw.cst(name)))
    def neg(self, x): return self.mul(x, self.const(CURVES[self.tw.curve]['neg']))

    def out(self, arg, idx, v):
        self.outs.append((arg, idx, v))

    # ---- compile ----
    def compile(self):
        tw, c = self.tw, CURVES[self.tw.curve]
        SZ = c['sz']
        argreg = {a: r for a, r, _, _ in self.args}
        ops = [v for v, d in enumerate(self.defs) if d[0] not in ('in', 'const')]
        live_ops = set()
        # keep only ops reachable from the outputs
        need = set(v for _, _, v in self.outs)
        for v in reversed(range(len(self.defs))):
            d = self.defs[v]
            if v in need and d[0] not in ('in', 'const'):
                need.add(d[1]); need.add(d[2])
        ops = [v for v in ops if v in need]
        END = len(ops)
        pos = {v: i for i, v in enumerate(ops)}
        last = {}
        for i, v in enumerate(ops):
            _, x, y = self.defs[v]
            last[x] = i
            last[y] = i
        outvals = {}
        for a, i, v in self.outs:
            last[v] = END
            outvals.setdefault(v, []).append((a, i))

        def writable(v):
            return self.defs[v][0] not in ('in', 'const')

        # static in-place decisions
        target, other, fresh = {}, {}, {}
        for i, v in enumerate(ops):
            o, x, y = self.defs[v]
            if writable(x) and last[x] == i:
                target[v], other[v], fresh[v] = x, y, False
            elif o in COMM and writable(y) and last[y] == i:
                target[v], other[v], fresh[v] = y, x, False
            else:
                if o in COMM and self.defs[x][0] == 'const':
                    x, y = y, x
                target[v], other[v], fresh[v] = x, y, True
        # chains
        chain = {}
        chains = []
        for v in ops:
            if fresh[v]:
                chain[v] = len(chains)
                chains.append([v])
            else:
                chain[v] = chain[target[v]]
                chains[chain[v]].append(v)
        # locations
        loc = {}
        for v, d in enumerate(self.defs):
            if d[0] == 'in':
                a, idx = d[1], d[2]
                loc[v] = ('dyn', argreg[a], idx * SZ)
            elif d[0] == 'const':
                loc[v] = ('st', d[1])
        chain_loc = {}
        used_out = set()
        for ci, ch in enumerate(chains):
            fin = ch[-1]
            for a, idx in outvals.get(fin, []):
                if (a, idx) not in used_out:
                    used_out.add((a, idx))
                    chain_loc[ci] = ('dyn', argreg[a], idx * SZ)
                    break
        # interval-allocate temp slots for the other chains
        free, slots_used = [], 0
        events = sorted((pos[ch[0]], ci) for ci, ch in enumerate(chains) if ci not in chain_loc)
        ends = []
        for start, ci in events:
            ends.sort()
            while ends and ends[0][0] < start:
                free.append(ends.pop(0)[1])
            if free:
                s = free.pop()
            else:
                s = slots_used
                slots_used += 1
            chain_loc[ci] = ('st', f'{tw.pfx}_T{s}')
            fin = chains[ci][-1]
            ends.append((last[fin], s))
        tw.nslots = max(tw.nslots, slots_used)
        for v in ops:
            loc[v] = chain_loc[chain[v]]

        # ---- emit ----
        L = []
        e = L.append
        e(f'; {self.doc}')
        e(f'zisklib_{self.name}_{tw.curve}:')
        ins = [(a, r, n) for a, r, n, k in self.args if k == 'in']
        outs = [(a, r, n) for a, r, n, k in self.args if k == 'out']
        alias = []
        for oa, orr, on in outs:
            for j, (ia, ir, inn) in enumerate(ins):
                lab = f'{tw.lp}_{self.name}_al{j}'
                e(f'\teq({orr}, {ir}), j({lab})')
                e(f'{tw.lp}_{self.name}_c{j}:')
                alias.append((lab, f'{tw.lp}_{self.name}_c{j}', ir, inn, f'{tw.pfx}_SAVE{j}'))
                tw.need_save(j, inn * SZ)

        held = {}                 # scratch register -> the dynamic location it holds

        def dyn_reg(l, scratch):
            _, r, off = l
            if off == 0:
                return r
            if held.get(scratch) != l:
                e(f'\tadd({r}, {off}) -> {scratch}')
                held[scratch] = l
            return scratch

        def dma(dst, src, n):
            e(f'\tdma_xmemcpy({dst}, {src}) -> r5, j({n}, 4)')
            held.pop('r5', None)

        nops = ncopies = 0
        for v in ops:
            o = self.defs[v][0]
            t, y = target[v], other[v]
            dst = loc[v]
            if fresh[v]:
                src = loc[t]
                if src != dst:
                    ds, dstat = (dst[1], True) if dst[0] == 'st' else (None, False)
                    ss, sstat = (src[1], True) if src[0] == 'st' else (None, False)
                    dr = ds if dstat else dyn_reg(dst, 'r14')
                    sr = ss if sstat else dyn_reg(src, 'r5')
                    dma(dr, sr, SZ)
                    ncopies += 1
            ol = loc[y]
            da = dst[1] if dst[0] == 'st' else dyn_reg(dst, 'r14')
            ob = ol[1] if ol[0] == 'st' else dyn_reg(ol, 'r5')
            e(f'\t{c["pre"]}_{o}({da}, {ob})')
            nops += 1
        # outputs not produced in place
        for a, idx, v in self.outs:
            want = ('dyn', argreg[a], idx * SZ)
            if loc[v] != want:
                src = loc[v]
                dr = dyn_reg(want, 'r14')
                sr = src[1] if src[0] == 'st' else dyn_reg(src, 'r5')
                dma(dr, sr, SZ)
                ncopies += 1
        e('\tret')
        for lab, back, ir, n, buf in alias:
            e(f'{lab}:')
            e(f'\tdma_xmemcpy({buf}, {ir}) -> r5, j({n * SZ}, 4)')
            e(f'\tcopyb(0, {buf}) -> {ir}')
            e(f'\tjump({back})')
        self.stats = (nops, ncopies)
        return L


class Tower:
    def __init__(self, curve):
        self.curve = curve
        c = CURVES[curve]
        self.pfx = c['pfx']
        self.lp = self.pfx.lower()
        self.consts = {}      # name -> fp2 value (re, im)
        self.nslots = 0
        self.saves = {}
        self.kernels = []

    def need_save(self, j, nbytes):
        self.saves[j] = max(self.saves.get(j, 0), nbytes)

    def cst(self, name):
        """Fp2 constants by name: 'xi', '3', '3xi', '2xi', ... (k or kxi)."""
        c = CURVES[self.curve]
        p = c['p']
        if name.endswith('xi'):
            k = int(name[:-2] or '1')
            val = ((k * c['xi'][0]) % p, (k * c['xi'][1]) % p)
        else:
            val = (int(name) % p, 0)
        sym = f'{self.pfx}_C{name.upper()}'
        self.consts[sym] = val
        return sym

    def kernel(self, name, args, doc):
        k = Kernel(self, name, args, doc)
        self.kernels.append(k)
        return k

    def emit(self, header):
        c = CURVES[self.curve]
        nl = c['sz'] // 8
        body = []
        for k in self.kernels:
            body += k.compile()
            body.append('')
        L = list(header)
        L.append('')
        for sym, (re_, im) in self.consts.items():
            L.append(f'const u64 {sym}[{nl}] = ' + ', '.join(f'0x{x:x}' for x in limbs(re_, nl // 2) + limbs(im, nl // 2)))
        for i in range(self.nslots):
            L.append(f'u64 {self.pfx}_T{i}[{nl}] = ' + ', '.join(['0'] * nl))
        for j, nb in sorted(self.saves.items()):
            L.append(f'u64 {self.pfx}_SAVE{j}[{nb // 8}] = ' + ', '.join(['0'] * (nb // 8)))
        L.append('')
        return L + body
