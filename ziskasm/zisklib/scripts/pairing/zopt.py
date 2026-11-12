#!/usr/bin/env python3
"""Peephole optimizer for the generated bn254 / bls12_381 tower .zisk files.

Symbolically executes straight-line blocks, keeping argument-register setup
lazy, and replaces calls to the Fp2 leaves (add/sub/mul/dbl/square/neg/
scalar_mul) and bn254_memcpy with inline DMA + precompile code. The Fp2
precompiles take their two pointers directly (a = the in-place operand, b = the
other one): a static operand is an immediate, a dynamic one a register. The Fp
leaves (arith256_mod / arith384_mod) still take a 5-pointer header: headers for
static operands are const ROM data; mixed ones get a per-site RAM header.

Value domain (logical register contents):
  ('sym', S, K)  address constant S+K (S=None: plain number K)
  ('mem', S, K)  (contents of the u64 slot at symbol S) + K
  ('val', i, K)  opaque value i + K (i is the value a register held at block
                 entry or after an instruction we don't model)
  None           garbage (clobbered, must not be read)
"""
import re, sys

CURVES = {
    'bn254': dict(sz=64, half=32, pre='bn254_complex', sb='BN_F2_SB', neg='BN_F2_NEGMUL'),
    'bls12_381': dict(sz=96, half=48, pre='bls12_381_complex', sb='BLS_F2_SB', neg='BLS_F2_NEGMUL'),
}
FPLEAF = {
    'bn254': dict(pre='arith256_mod', n=4, add=['r10', 'BN_ONE', 'r11', 'BN_P', 'r12'],
                  neg=['r10', 'BN_P_MINUS_ONE', 'BN_ZERO', 'BN_P', 'r11'],
                  mul=['r10', 'r11', 'BN_ZERO', 'BN_P', 'r12'], square=['r10', 'r10', 'BN_ZERO', 'BN_P', 'r11']),
    'bls12_381': dict(pre='arith384_mod', n=6, add=['r10', 'BLS_ONE', 'r11', 'BLS_P', 'r12'],
                      dbl=['r10', 'BLS_ONE', 'r10', 'BLS_P', 'r11'], sub=['r11', 'BLS_P_MINUS_ONE', 'r10', 'BLS_P', 'r12'],
                      neg=['r10', 'BLS_P_MINUS_ONE', 'BLS_ZERO', 'BLS_P', 'r11'],
                      mul=['r10', 'r11', 'BLS_ZERO', 'BLS_P', 'r12'], square=['r10', 'r10', 'BLS_ZERO', 'BLS_P', 'r11']),
}
FPRE = re.compile(r'^zisklib_(add|dbl|sub|neg|mul|square)_fp_(bn254|bls12_381)$')
LEAF = re.compile(r'^zisklib_(add|sub|mul|dbl|square|neg|scalar_mul)_fp2_(bn254|bls12_381)$')
REGS = [f'r{i}' for i in range(1, 40)]
ARGS = ('r10', 'r11', 'r12')
SCRATCH = ('r14', 'r5')

class Opt:
    def __init__(self, pfx):
        self.pfx = pfx
        self.out = []
        self.consts = {}      # 5-pointer Fp header key -> name  (const headers)
        self.rams = []        # (name, slots...)  (RAM headers, dynamic slots written per call)
        self.nid = 0
        self.stats = dict(inl=0, memcpy=0, kept=0)
        self.reset()

    # ---------------- state ----------------
    def fresh(self):
        self.nid += 1
        return ('val', self.nid, 0)

    def reset(self):
        self.log = {}
        self.phys = {}
        for r in REGS:
            v = self.fresh()
            self.log[r] = v
            self.phys[r] = v
        self.memf = {}        # slot symbol -> value stored there (forwarding)

    def emit(self, s):
        self.out.append('\t' + s)

    # ---------------- values ----------------
    @staticmethod
    def add(v, k):
        if v is None:
            return None
        return (v[0], v[1], v[2] + k)

    @staticmethod
    def fmt_sym(v):
        _, s, k = v
        if s is None:
            return str(k)
        return s if k == 0 else (f'{s} + {k}' if k > 0 else f'{s} - {-k}')

    def holders(self, v):
        """physical registers r with phys[r] = same base value (any offset)."""
        return [r for r in REGS if self.phys[r] is not None and v is not None and
                self.phys[r][0] == v[0] and self.phys[r][1] == v[1]]

    def srcs(self, r):
        """physical registers the materialization of log[r] would read."""
        v = self.log[r]
        if v is None or v[0] != 'val' or self.phys[r] == v:
            return []
        hs = [h for h in self.holders(v)]
        return hs[:1]

    def mat_code(self, r, v):
        """code putting value v into physical register r (r's own phys may be read)."""
        if v[0] == 'sym':
            return f'copyb(0, {self.fmt_sym(v)}) -> {r}'
        if v[0] == 'mem':
            return f'copyb(0, [{v[1]}]) -> {r}' if v[2] == 0 else f'add([{v[1]}], {v[2]}) -> {r}'
        hs = self.holders(v)
        if hs:
            q = r if r in hs else hs[0]
            k = v[2] - self.phys[q][2]
            if q == r and k == 0:
                return None
            return f'copyb(0, {q}) -> {r}' if k == 0 else f'add({q}, {k}) -> {r}'
        for s, mv in self.memf.items():
            if mv is not None and mv[0] == 'val' and mv[1] == v[1]:
                k = v[2] - mv[2]
                return f'copyb(0, [{s}]) -> {r}' if k == 0 else f'add([{s}], {k}) -> {r}'
        raise RuntimeError(f'lost value {v} for {r}')

    def materialize(self, r):
        v = self.log[r]
        if v is None or self.phys[r] == v:
            return
        c = self.mat_code(r, v)
        if c:
            self.emit(c)
        self.phys[r] = v

    def needed_elsewhere(self, r):
        """does some other pending register need phys[r] as its source?"""
        pv = self.phys[r]
        if pv is None or pv[0] != 'val':
            return False
        for q in REGS:
            if q == r:
                continue
            lv = self.log[q]
            if lv is None or self.phys[q] == lv or lv[0] != 'val' or lv[1] != pv[1]:
                continue
            others = [h for h in self.holders(lv) if h != r]
            mem = any(mv is not None and mv[0] == 'val' and mv[1] == pv[1] for mv in self.memf.values())
            if not others and not mem:
                return True
        return False

    def flush(self, regs=None):
        pend = [r for r in (regs or REGS) if self.log[r] is not None and self.phys[r] != self.log[r]]
        while pend:
            for r in pend:
                if not self.needed_elsewhere(r):
                    self.materialize(r)
                    pend.remove(r)
                    break
            else:
                raise RuntimeError(f'register cycle {pend}')

    def flush_mem(self, slot=None):
        """materialize pending loads of slot (all slots if None) before a store."""
        for r in REGS:
            v = self.log[r]
            if v is not None and v[0] == 'mem' and (slot is None or v[1] == slot) and self.phys[r] != v:
                self.flush([r]) if not self.needed_elsewhere(r) else self.flush()
        if slot is None:
            self.memf = {}
        else:
            self.memf.pop(slot, None)
        self.relabel(slot)

    def relabel(self, slot=None):
        """registers loaded from slot keep their (now opaque) value."""
        for r in REGS:
            v = self.phys[r]
            if v is not None and v[0] == 'mem' and (slot is None or v[1] == slot):
                nv = self.fresh()
                nv = (nv[0], nv[1], v[2])
                if self.log[r] == v:
                    self.log[r] = nv
                self.phys[r] = nv
            lv = self.log[r]
            if lv is not None and lv[0] == 'mem' and (slot is None or lv[1] == slot):
                raise RuntimeError('pending load across store')

    def clobber(self, r):
        """physical r is about to be overwritten by inline code; log[r] -> garbage."""
        if self.needed_elsewhere(r):
            self.flush()
        self.log[r] = None
        self.phys[r] = None

    # ---------------- operand helpers ----------------
    def operand(self, v, avoid=(), scratch=SCRATCH):
        """an assembler operand for address value v: immediate if static, else a register."""
        if v[0] == 'sym':
            return self.fmt_sym(v), True
        for r in REGS:
            if self.phys[r] == v and r not in avoid:
                return r, False
        for s in scratch:
            if s in avoid:
                continue
            if self.needed_elsewhere(s):
                continue
            c = self.mat_code(s, v)
            if c:
                self.emit(c)
            self.phys[s] = v
            self.log[s] = v
            return s, False
        raise RuntimeError('no scratch')

    # ---------------- inline leaves ----------------
    def to_mem(self, v, dest):
        """store address value v into memory [dest] with one instruction."""
        if v[0] == 'sym':
            self.emit(f'copyb(0, {self.fmt_sym(v)}) -> [{dest}]')
            return
        c = self.mat_code('rX', v)
        if c.endswith('-> rX'):
            c = c[:-len('rX')] + f'[{dest}]'
        self.emit(c)

    def prep_scratch(self, scratch):
        for s in scratch:
            if self.needed_elsewhere(s):
                self.flush()
                break
        for s in scratch:
            self.log[s] = None

    def inline_fp2(self, op, curve):
        c = CURVES[curve]
        a, b, d = (self.log[r] for r in ARGS)
        if op in ('dbl', 'square', 'neg'):
            d, b = b, a
        if any(v is None for v in (a, b, d)):
            return False
        if op not in ('dbl', 'square', 'neg') and d == b and d != a:
            return False                        # result aliases b: keep the call
        # the leaf clobbers the scratch registers anyway
        self.prep_scratch(SCRATCH)
        pre = c['pre']
        if op == 'neg':
            b = ('sym', c['neg'], 0)
        elif op == 'scalar_mul':
            bo, _ = self.operand(b)
            self.emit(f'dma_xmemcpy({c["sb"]}, {bo}) -> r5, j({c["half"]}, 4)')
            self.phys['r5'] = self.log['r5'] = None
            b = ('sym', c['sb'], 0)
        if d != a:
            so, ss = self.operand(d)
            ao, _ = self.operand(a, avoid=() if ss else (so,))
            self.emit(f'dma_xmemcpy({so}, {ao}) -> r5, j({c["sz"]}, 4)')
            self.phys['r5'] = self.log['r5'] = None      # the DMA op wrote r5
        kind = {'add': 'add', 'sub': 'sub', 'mul': 'mul', 'dbl': 'add', 'square': 'mul',
                'neg': 'mul', 'scalar_mul': 'mul'}[op]
        # the precompile takes both pointers directly: a = &f1 (updated in place), b = &f2
        do, ds = self.operand(d)
        bo = do if b == d else self.operand(b, avoid=() if ds else (do,))[0]
        self.emit(f'{pre}_{kind}({do}, {bo})')
        self.after_inline(d)
        return True

    def after_inline(self, dst):
        for s in SCRATCH:
            self.log[s] = None
            self.phys[s] = None
        if dst[0] == 'sym':
            self.memf.pop(dst[1], None)
            self.relabel(dst[1])
        self.stats['inl'] += 1

    def inline_memcpy(self):
        src, dst, n = (self.log[r] for r in ARGS)
        if n is None or n[0] != 'sym' or n[1] is not None or src is None or dst is None:
            return False
        if n[2] == 0:
            return True
        # r5 is clobbered by bn254_memcpy; r14 is not, so it is scratch only when dead
        sc = ('r5',) + (('r14',) if self.log['r14'] is None else ())
        self.prep_scratch(('r5',))
        if dst[0] != 'sym' and src[0] != 'sym' and len(sc) < 2 and \
                not any(self.phys[r] in (dst, src) for r in REGS):
            return False
        used = []
        do, ds = self.operand(dst, scratch=sc)
        if not ds:
            used.append(do)
        so, _ = self.operand(src, avoid=used, scratch=sc)
        self.emit(f'dma_xmemcpy({do}, {so}) -> r5, j({8 * n[2]}, 4)')
        self.log['r5'] = self.phys['r5'] = None
        if dst[0] == 'sym':
            self.memf.pop(dst[1], None)
            self.relabel(dst[1])
        self.stats['memcpy'] += 1
        return True

    def inline_eqn(self):
        """r10 = (n limbs at r10 == at r11); bn254_eqn clobbers r5 and writes r10."""
        a, b, n = (self.log[r] for r in ARGS)
        if n is None or n[0] != 'sym' or n[1] is not None or a is None or b is None:
            return False
        if self.needed_elsewhere('r10') or self.needed_elsewhere('r5'):
            self.flush()
        self.log['r5'] = None
        if n[2] == 0:
            self.emit('copyb(0, 1) -> r10')
        else:
            sc = ('r5', 'r10')
            ao, as_ = self.operand(a, scratch=sc)
            bo, _ = self.operand(b, avoid=() if as_ else (ao,), scratch=sc)
            self.emit(f'dma_xmemcmp({ao}, {bo}) -> r5, j({8 * n[2]}, 4)')
            self.emit('eq(r5, 0) -> r10')
        v = self.fresh()
        self.log['r10'] = self.phys['r10'] = v
        self.log['r5'] = self.phys['r5'] = None
        self.stats['eqn'] = self.stats.get('eqn', 0) + 1
        return True

    def inline_fp(self, op, curve):
        t = FPLEAF[curve]
        tmpl = t[op]
        vals = [self.log[x] if x.startswith('r') else ('sym', x, 0) for x in tmpl]
        if any(v is None for v in vals):
            return False
        self.prep_scratch(SCRATCH)
        stat = [v[0] == 'sym' for v in vals]
        if all(stat):
            key = tuple(self.fmt_sym(v) for v in vals)
            if key not in self.consts:
                self.consts[key] = f'{self.pfx}_H{len(self.consts) + len(self.rams)}'
            h = self.consts[key]
        else:
            h = f'{self.pfx}_H{len(self.consts) + len(self.rams)}'
            self.rams.append((h, *[self.fmt_sym(v) if st else '0' for v, st in zip(vals, stat)]))
            for i, (v, st) in enumerate(zip(vals, stat)):
                if not st:
                    self.to_mem(v, f'{h} + {8 * i}' if i else h)
        self.emit(f'{t["pre"]}(0, {h})')
        res = vals[4]
        for sc in SCRATCH:
            self.log[sc] = self.phys[sc] = None
        if res[0] == 'sym':
            self.memf.pop(res[1], None)
            self.relabel(res[1])
        self.stats['fp'] = self.stats.get('fp', 0) + 1
        return True

    def inline_marshal(self, to_limbs):
        """be_to_limbs(be=r10, limbs=r11, n=r12) / limbs_to_be(limbs=r10, be=r11, n=r12), n const.
        limb[i] = rev8(be word n-1-i). Both helpers clobber r5, r6, r7, r14."""
        src, dst, n = (self.log[r] for r in ARGS)
        if n is None or n[0] != 'sym' or n[1] is not None or src is None or dst is None:
            return False
        n = n[2]
        self.prep_scratch(('r5', 'r14'))
        self.log['r6'] = self.log['r7'] = None
        if self.needed_elsewhere('r6') or self.needed_elsewhere('r7'):
            self.flush()
        sst, dstt = src[0] == 'sym', dst[0] == 'sym'
        so = None if sst else self.operand(src, scratch=('r5',))[0]
        do = None if dstt else self.operand(dst, avoid=(so,), scratch=('r14', 'r6'))[0]
        for i in range(n):
            s_off = ((n - 1 - i) if to_limbs else i) * 8
            d_off = (i if to_limbs else (n - 1 - i)) * 8
            sref = f'[{self.fmt_sym(self.add(src, s_off))}]' if sst else f'8[a + {s_off}]'
            sa = '0' if sst else so
            if dstt:
                self.emit(f'rev8({sa}, {sref}) -> [{self.fmt_sym(self.add(dst, d_off))}]')
            elif sst:
                self.emit(f'rev8({do}, {sref}) -> 8[a + {d_off}]')
            else:
                self.emit(f'rev8({sa}, {sref}) -> r7')
                self.emit(f'copyb({do}, r7) -> 8[a + {d_off}]')
        for r in ('r5', 'r6', 'r7', 'r14'):
            self.log[r] = self.phys[r] = None
        if dstt:                        # (dynamic destinations are data buffers, never pointer slots)
            self.memf.pop(dst[1], None)
            self.relabel(dst[1])
        self.stats['marshal'] = self.stats.get('marshal', 0) + 1
        return True

    def inline_ltn(self):
        """r10 = (a < b) over n limbs MSW-first, both static; bn254_ltn clobbers r5..r7, r14, r15."""
        a, b, n = (self.log[r] for r in ARGS)
        if n is None or n[0] != 'sym' or n[1] is not None or a is None or b is None \
                or a[0] != 'sym' or b[0] != 'sym' or n[2] == 0:
            return False
        self.flush()
        k = self.stats.get('ltn', 0)
        lt, ge, end = f'{self.pfx.lower()}_lt{k}', f'{self.pfx.lower()}_ge{k}', f'{self.pfx.lower()}_le{k}'
        for i in reversed(range(n[2])):
            A, B = self.fmt_sym(self.add(a, 8 * i)), self.fmt_sym(self.add(b, 8 * i))
            self.emit(f'ltu([{A}], [{B}]), j({lt})')
            self.emit(f'ltu([{B}], [{A}]), j({ge})')
        self.out.append(f'{ge}:')
        self.emit('copyb(0, 0) -> r10')
        self.emit(f'jump({end})')
        self.out.append(f'{lt}:')
        self.emit('copyb(0, 1) -> r10')
        self.out.append(f'{end}:')
        self.reset()
        # r11 / r12 keep their values (bn254_ltn preserves them)
        self.log['r11'], self.log['r12'] = b, n
        self.stats['ltn'] = k + 1
        return True

    def fail_restore(self):
        raise RuntimeError('r14 needed')

    # ---------------- driver ----------------
    def generic(self, line, writes_all=False):
        body = line.split(';')[0].strip()
        ctl = writes_all or 'j(' in body or re.match(r'^(jump|ret|call|push|pop)\b', body) \
            or re.search(r',\s*end\b', body)
        m = re.search(r'->\s*(r\d+)', body)
        stores = bool(re.search(r'->\s*(\d*\[|\[)', body))
        if ctl:
            self.flush()
        else:
            reads = [r for r in re.findall(r'\br\d+\b', body.split('->')[0])]
            if m and self.needed_elsewhere(m.group(1)):
                self.flush()
            for r in reads:
                if self.needed_elsewhere(r) and self.phys[r] != self.log[r]:
                    self.flush()
                    break
            self.flush(reads)
            if stores:
                self.flush_mem(None)
        self.out.append(line)
        if writes_all:
            for r in REGS:
                v = self.fresh()
                self.log[r] = self.phys[r] = v
            self.memf = {}
        elif m:
            v = self.fresh()
            self.log[m.group(1)] = self.phys[m.group(1)] = v
        if stores or ctl:
            self.memf = {}
            self.relabel(None)

    def run(self, lines):
        for line in lines:
            s = line.strip()
            body = s.split(';')[0].strip()
            if not body:
                self.out.append(line)
                continue
            if re.match(r'^[A-Za-z_][\w]*:$', body):
                self.flush()
                self.out.append(line)
                self.reset()
                continue
            if not line.startswith('\t') and not line.startswith(' '):
                self.out.append(line)      # data / directives
                continue
            m = re.match(r'^call\s+(\w+)$', body)
            if m:
                fn = m.group(1)
                lm = LEAF.match(fn)
                ok = False
                if lm:
                    ok = self.inline_fp2(lm.group(1), lm.group(2))
                elif fn == 'bn254_memcpy':
                    ok = self.inline_memcpy()
                elif fn == 'bn254_eqn':
                    ok = self.inline_eqn()
                elif fn == 'bn254_ltn':
                    ok = self.inline_ltn()
                elif fn in ('zkvm_be_to_limbs', 'zkvm_limbs_to_be'):
                    ok = self.inline_marshal(fn == 'zkvm_be_to_limbs')
                elif FPRE.match(fn) and FPRE.match(fn).group(1) in FPLEAF[FPRE.match(fn).group(2)]:
                    ok = self.inline_fp(*FPRE.match(fn).groups())
                if not ok:
                    self.stats['kept'] += 1
                    self.generic(line, writes_all=True)
                continue
            # copyb(0, IMM) -> rX   /  copyb(0, rY) -> rX
            m = re.match(r'^copyb\(0,\s*([^\[\]]+?)\)\s*->\s*(r\d+)$', body)
            if m:
                src, rx = m.group(1).strip(), m.group(2)
                if re.fullmatch(r'r\d+', src):
                    v = self.log[src]
                    if v is None:
                        self.generic(line)
                        continue
                    self.log[rx] = v
                else:
                    self.log[rx] = parse_imm(src)
                continue
            # add(rY, K) -> rX
            m = re.match(r'^add\((r\d+),\s*(-?\d+)\)\s*->\s*(r\d+)$', body)
            if m and self.log[m.group(1)] is not None:
                self.log[m.group(3)] = self.add(self.log[m.group(1)], int(m.group(2)))
                continue
            # load: copyb(rY, 8[a + 0]) -> rX  with rY = sym
            m = re.match(r'^copyb\((r\d+),\s*8\[a \+ (\d+)\]\)\s*->\s*(r\d+)$', body)
            if m and self.log[m.group(1)] is not None and self.log[m.group(1)][0] == 'sym' \
                    and self.log[m.group(1)][1] is not None and self.log[m.group(1)][2] + int(m.group(2)) == 0:
                slot = self.log[m.group(1)][1]
                fv = self.memf.get(slot)
                self.log[m.group(3)] = fv if fv is not None else ('mem', slot, 0)
                continue
            # store: copyb(rY, rX) -> 8[a + 0]  with rY = sym
            m = re.match(r'^copyb\((r\d+),\s*(r\d+|\d+)\)\s*->\s*8\[a \+ 0\]$', body)
            if m and self.log[m.group(1)] is not None and self.log[m.group(1)][0] == 'sym' \
                    and self.log[m.group(1)][1] is not None and self.log[m.group(1)][2] == 0:
                slot = self.log[m.group(1)][1]
                src = m.group(2)
                self.flush_mem(slot)
                if src.startswith('r'):
                    v = self.log[src]
                    if v is None:
                        raise RuntimeError('store of garbage')
                    if v[0] == 'sym':
                        self.emit(f'copyb(0, {self.fmt_sym(v)}) -> [{slot}]')
                    else:
                        self.materialize(src) if not self.needed_elsewhere(src) else self.flush()
                        self.emit(f'copyb(0, {src}) -> [{slot}]')
                    self.memf[slot] = v
                else:
                    self.emit(f'copyb(0, {src}) -> [{slot}]')
                    self.memf[slot] = ('sym', None, int(src))
                continue
            self.generic(line, writes_all=body.startswith('call'))
        self.flush()
        return self.out

def parse_imm(s):
    s = s.strip()
    if re.fullmatch(r'-?(0x[0-9a-fA-F]+|\d+)', s):
        return ('sym', None, int(s, 0))
    m = re.fullmatch(r'([A-Za-z_]\w*)\s*(?:([+-])\s*(\d+))?', s)
    if not m:
        raise RuntimeError(f'imm {s}')
    k = int(m.group(3)) * (1 if m.group(2) == '+' else -1) if m.group(3) else 0
    return ('sym', m.group(1), k)

def optimize(text, pfx):
    lines = text.split('\n')
    o = Opt(pfx)
    out = o.run(lines)
    # header data goes right after the leading comment block
    i = 0
    while i < len(out) and (out[i].startswith(';') or not out[i].strip()):
        i += 1
    decl = []
    if o.consts or o.rams:
        decl.append('; inline-op headers (zopt): arith256_mod / arith384_mod parameter blocks')
        for key, n in o.consts.items():
            decl.append(f'const u64 {n}[{len(key)}] = ' + ', '.join(key))
        for n, *slots in o.rams:
            decl.append(f'u64 {n}[{len(slots)}] = ' + ', '.join(slots))
        decl.append('')
    return '\n'.join(out[:i] + decl + out[i:]), o.stats

if __name__ == '__main__':
    src, dst, pfx = sys.argv[1], sys.argv[2], sys.argv[3]
    txt, st = optimize(open(src).read(), pfx)
    open(dst, 'w').write(txt)
    print(f'{src}: {st}', file=sys.stderr)
