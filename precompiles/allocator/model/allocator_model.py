#!/usr/bin/env python3
"""Model of Allocator (precompiles/allocator/pil/allocator.pil).

Deterministic allocator: ordered free list (best fit) + forced coalescing. See ../README.md.

Tokens (type, addr, size, prev, dirty, ver, next_size, next_addr), all in one multiset bus:

  FREE  free block and node of the free list, which is ordered by key (size, addr). next_* is
        the key of the following node. The head is FREE(addr 0, size 0), the tail is only a key.
  USE   block handed out.
  GUARD end of a created area (a block that is never free).

  prev is the size of the block just below (0 at the start of an area).
  ver  is the version that produced the token: 4 * step + k, k = 1..3 the update of the row.

verify() is the constraint system of the PIL (equalities, booleans and range checks over
Goldilocks, plus the buses), written per operation instead of with selectors. Executor fills the
witness. reference() is an independent best-fit allocator: the addresses of an accepted trace must
be exactly its addresses. oracle() checks what the guest needs (disjoint live blocks, calloc zeroed).

Run: python3 allocator_model.py
"""
import bisect
import random
import sys
from collections import Counter

P = 2**64 - 2**32 + 1
F, U, G = 1, 2, 3
TAIL = (2**32, 0)
HEAD = (F, 0, 0, 0, 0, 0, *TAIL)
VER_BITS = 40               # 16 + 24
KEY_BITS = 62               # 16 + 22 + 24
OP_CREATE_CLEAN, OP_CREATE_DIRTY, OP_ALLOC, OP_CALLOC, OP_DEALLOC = 0xDB, 0xDC, 0xDD, 0xDE, 0xDF

STRICT = dict(version=True, best_fit=True)


def key(size, addr):
    # sizes are multiples of 8 and addr < 2**33, so this is the lexicographic order (size, addr)
    return size * 2**30 + addr


def verify(rows, main_ops, opts=STRICT):
    errors = []
    bus = Counter({HEAD: 1})         # proved once by a global constraint
    ops = Counter()
    memsets = []

    for i, r in enumerate(rows):
        def err(msg):
            errors.append(f'row {i} ({r["op"]}): {msg}')

        def eq0(name, value):
            if value % P:
                err(name)

        def rng(name, value, bits):
            if not 0 <= value % P < 2**bits:
                err(f'range {name}')

        def boolean(*names):
            for d, name in names:
                if d[name] % P not in (0, 1):
                    err(f'{name} not boolean')

        def tok(*fields):
            return tuple(x % P for x in fields)

        def consume(t, w, sel=1):
            if sel:
                bus[tok(*t)] -= sel
                if opts['version']:
                    rng(f'version of consumed token {t[:2]}', w - t[5] - 1, VER_BITS)

        def emit(t, sel):
            if sel:
                bus[tok(*t)] += sel

        def remove(s, node, w):
            """node = (addr, size, prev, dirty, ver, nsize, naddr). s holds its predecessor."""
            boolean((s, 'emit_p'))
            consume((F, s['p_addr'], s['p_size'], s['p_prev'], s['p_dirty'], s['p_ver'], node[1], node[0]), w)
            consume((F, *node), w)
            emit((F, s['p_addr'], s['p_size'], s['p_prev'], s['p_dirty'], w, node[5], node[6]), s['emit_p'])

        def insert(s, addr, size, prev, dirty, w):
            boolean((s, 'emit_q'), (s, 'emit_m'))
            consume((F, s['q_addr'], s['q_size'], s['q_prev'], s['q_dirty'], s['q_ver'], s['qn_size'], s['qn_addr']), w)
            rng('insert: pred < node', key(size, addr) - key(s['q_size'], s['q_addr']) - 1, KEY_BITS)
            rng('insert: node < next', key(s['qn_size'], s['qn_addr']) - key(size, addr) - 1, KEY_BITS)
            emit((F, s['q_addr'], s['q_size'], s['q_prev'], s['q_dirty'], w, size, addr), s['emit_q'])
            emit((F, addr, size, prev, dirty, w, s['qn_size'], s['qn_addr']), s['emit_m'])

        def update_upper(s, addr, old_prev, new_prev, w):
            """First non free block above: USE or GUARD. Its prev is updated."""
            boolean((s, 'guard'), (s, 'emit'))
            eq0('guard has no size', s['guard'] * s['size'])
            t = U + s['guard'] * (G - U)
            consume((t, addr, s['size'], old_prev, 0, s['ver'], 0, 0), w)
            emit((t, addr, s['size'], new_prev, 0, w, 0, 0), s['emit'])

        # no clock token: the version IS the time, version = 4 * step + k
        step = r['step']
        v = 4 * step

        if r['op'] == 'create':
            boolean((r, 'dirty'), (r, 'emit_guard'))
            insert(r['ins'], r['addr'], r['size'], 0, r['dirty'], v + 3)
            emit((G, r['addr'] + r['size'], 0, r['size'], 0, v + 3, 0, 0), r['emit_guard'])
            ops[tok(OP_CREATE_CLEAN + r['dirty'], r['addr'], r['size'], 0, step)] += 1

        elif r['op'] == 'alloc':
            boolean((r, 'calloc'), (r, 'has_rest'), (r, 'emit_use'), (r, 'n_dirty'))
            size, fsize, addr = r['size'], r['n_size'], r['n_addr']
            rng('size = 8 * (chunks + 1)', (size - 8) * pow(8, P - 2, P), 32)
            # best fit: the predecessor is too small, the node is big enough
            if opts['best_fit']:
                rng('pred.size < size', size - r['rm']['p_size'] - 1, 32)
            rng('size <= node.size, rest >= 8', fsize - size - 8 * r['has_rest'], 32)
            eq0('no rest ==> exact fit', (1 - r['has_rest']) * (fsize - size))
            remove(r['rm'], (addr, fsize, r['n_prev'], r['n_dirty'], r['n_ver'], r['nn_size'], r['nn_addr']), v + 1)
            emit((U, addr, size, r['n_prev'], 0, v + 1, 0, 0), r['emit_use'])
            if r['has_rest']:
                update_upper(r['up'], addr + fsize, fsize, fsize - size, v + 3)
                insert(r['ins'], addr + size, fsize - size, size, r['n_dirty'], v + 3)
            if r['calloc'] and r['n_dirty']:
                memsets.append((addr, size, step))
            ops[tok(OP_ALLOC + r['calloc'], size, 0, addr, step)] += 1

        elif r['op'] == 'dealloc':
            boolean((r, 'has_below'), (r, 'join_below'), (r, 'join_above'))
            addr, size, prev = r['addr'], r['size'], r['prev']
            consume((U, addr, size, prev, 0, r['use_ver'], 0, 0), v + 1)
            eq0('no below ==> prev == 0', (1 - r['has_below']) * prev)
            eq0('join_below ==> has_below', r['join_below'] * (1 - r['has_below']))
            b = r['below']
            if r['join_below']:
                remove(r['rm_below'], (addr - prev, prev, b['prev'], b['dirty'], b['ver'], b['nsize'], b['naddr']), v + 1)
            elif r['has_below']:
                # the block below is in use: show it (consumed and produced with a new version)
                boolean((b, 'emit'))
                consume((U, addr - prev, prev, b['prev'], 0, b['ver'], 0, 0), v + 1)
                emit((U, addr - prev, prev, b['prev'], 0, v + 1, 0, 0), b['emit'])
            a = r['above']
            if r['join_above']:
                remove(r['rm_above'], (addr + size, a['size'], size, a['dirty'], a['ver'], a['nsize'], a['naddr']), v + 2)
            jb, ja = r['join_below'], r['join_above']
            m_addr = addr - jb * prev
            m_size = size + jb * prev + ja * a['size']
            m_prev = prev + jb * (b['prev'] - prev)
            # the first non free block above proves that there is nothing else to join
            update_upper(r['up'], addr + size + ja * a['size'], size + ja * (a['size'] - size), m_size, v + 3)
            insert(r['ins'], m_addr, m_size, m_prev, 1, v + 3)
            ops[tok(OP_DEALLOC, addr, 0, 0, step)] += 1
        else:
            err('unknown op')

    for (op, a, b, c, step) in main_ops:
        ops[(op, a % P, b % P, c % P, step)] -= 1
    errors += [f'bus unbalanced {t}: {m}' for t, m in bus.items() if m % P]
    errors += [f'operation bus unbalanced {t}: {m}' for t, m in ops.items() if m % P]
    return errors, memsets


def oracle(main_ops, memsets):
    """What the guest needs: blocks inside the areas, live blocks disjoint, calloc zeroed."""
    areas, live, errors, nonzero = [], [], [], set()
    memsets = {(addr, step): size for (addr, size, step) in memsets}
    for (op, a, b, c, step) in sorted(main_ops, key=lambda o: o[4]):
        if op in (OP_CREATE_CLEAN, OP_CREATE_DIRTY):
            areas.append((a, b))
            if op == OP_CREATE_DIRTY:
                nonzero.update(range(a, a + b, 8))
        elif op in (OP_ALLOC, OP_CALLOC):
            addr, size = c, a
            if not any(base <= addr and addr + size <= base + asize for base, asize in areas):
                errors.append(f'step {step}: block {addr:#x}+{size} outside the created areas')
            for laddr, lsize in live:
                if addr < laddr + lsize and laddr < addr + size:
                    errors.append(f'step {step}: block {addr:#x}+{size} overlaps live block {laddr:#x}+{lsize}')
            if (addr, step) in memsets:
                nonzero.difference_update(range(addr, addr + memsets[(addr, step)], 8))
            if op == OP_CALLOC and any(w in nonzero for w in range(addr, addr + size, 8)):
                errors.append(f'step {step}: calloc block {addr:#x}+{size} not zeroed')
            live.append((addr, size))
            nonzero.update(range(addr, addr + size, 8))
        elif op == OP_DEALLOC:
            block = next((blk for blk in live if blk[0] == a), None)
            if block is None:
                errors.append(f'step {step}: dealloc of non live address {a:#x}')
            else:
                live.remove(block)
    return errors


def reference(main_ops):
    """Independent best-fit allocator with eager coalescing. Returns the errors of main_ops
    against it: the address of each alloc is fully determined by the previous operations."""
    blocks, errors = [], []                     # [addr, size, free, area]
    for (op, a, b, c, step) in sorted(main_ops, key=lambda o: o[4]):
        if op in (OP_CREATE_CLEAN, OP_CREATE_DIRTY):
            blocks.append([a, b, True, a])
        elif op in (OP_ALLOC, OP_CALLOC):
            fit = min((blk for blk in blocks if blk[2] and blk[1] >= a), key=lambda blk: (blk[1], blk[0]), default=None)
            if fit is None:
                errors.append(f'step {step}: alloc({a}) accepted without a free block')
                continue
            if fit[0] != c:
                errors.append(f'step {step}: alloc({a}) = {c:#x}, the best fit is {fit[0]:#x}')
                continue
            if fit[1] > a:
                blocks.append([fit[0] + a, fit[1] - a, True, fit[3]])
            fit[1], fit[2] = a, False
        elif op == OP_DEALLOC:
            blk = next((x for x in blocks if x[0] == a and not x[2]), None)
            if blk is None:
                errors.append(f'step {step}: dealloc of non live address {a:#x}')
                continue
            blk[2] = True
            for other in [x for x in blocks if x[2] and x[3] == blk[3] and x is not blk]:
                if other[0] + other[1] == blk[0]:
                    blk[0], blk[1] = other[0], blk[1] + other[1]
                    blocks.remove(other)
                elif blk[0] + blk[1] == other[0]:
                    blk[1] += other[1]
                    blocks.remove(other)
    return errors


class Executor:
    """Fills the witness. Keeps, for each live token, the row flag that says whether it's
    produced: the flag is set when (and only if) somebody consumes the token.

    The malicious options make it take other decisions, filling the witness as well as it can:
      choose='biggest'   alloc takes the biggest free block instead of the best fit
      no_join=True       dealloc never joins (free neighbours are shown as if they were in use)
    """

    def __init__(self, choose='best', no_join=False):
        self.choose, self.no_join = choose, no_join
        self.rows, self.main_ops = [], []
        self.keys = [(0, 0)]                    # ordered keys of the free list, head included
        self.reg = {}                           # entity -> (token, holder, flag)
        self.reg[(F, 0)] = (HEAD, {}, 'x')

    def take(self, entity):
        t, holder, flag = self.reg.pop(entity)
        holder[flag] = 1
        return t

    def put(self, t, holder, flag):
        holder[flag] = 0
        self.reg[(t[0], t[1])] = (t, holder, flag)

    def _remove(self, addr, w):
        node = self.take((F, addr))
        i = self.keys.index((node[2], node[1]))
        pred = self.take((F, self.keys[i - 1][1]))
        del self.keys[i]
        s = dict(p_addr=pred[1], p_size=pred[2], p_prev=pred[3], p_dirty=pred[4], p_ver=pred[5])
        self.put((F, *pred[1:5], w, node[6], node[7]), s, 'emit_p')
        return s, node

    def _insert(self, addr, size, prev, dirty, w):
        i = bisect.bisect_left(self.keys, (size, addr))
        pred = self.take((F, self.keys[i - 1][1]))
        self.keys.insert(i, (size, addr))
        s = dict(q_addr=pred[1], q_size=pred[2], q_prev=pred[3], q_dirty=pred[4], q_ver=pred[5],
                 qn_size=pred[6], qn_addr=pred[7])
        self.put((F, *pred[1:5], w, size, addr), s, 'emit_q')
        self.put((F, addr, size, prev, dirty, w, pred[6], pred[7]), s, 'emit_m')
        return s

    def _update_upper(self, addr, new_prev, w):
        # the GUARD of an area and the first block of a contiguous area share the address: the
        # block above is the one with prev != 0 (the first block of an area has prev == 0)
        is_use = (U, addr) in self.reg and self.reg[(U, addr)][0][3] != 0
        t = self.take((U, addr)) if is_use else self.take((G, addr))
        s = dict(guard=int(t[0] == G), size=t[2], ver=t[5])
        self.put((t[0], addr, t[2], new_prev, 0, w, 0, 0), s, 'emit')
        return s

    def _row(self, op, step, **kw):
        v = 4 * step
        r = dict(op=op, step=step, **kw)
        self.rows.append(r)
        return r, v

    def create(self, step, addr, size, dirty):
        r, v = self._row('create', step, addr=addr, size=size, dirty=dirty)
        r['ins'] = self._insert(addr, size, 0, dirty, v + 3)
        self.put((G, addr + size, 0, size, 0, v + 3, 0, 0), r, 'emit_guard')
        self.main_ops.append((OP_CREATE_DIRTY if dirty else OP_CREATE_CLEAN, addr, size, 0, step))

    def alloc(self, step, size, calloc=0):
        fits = [k for k in self.keys if k[0] >= size]
        if not fits:
            return None
        fsize, addr = fits[0] if self.choose == 'best' else fits[-1]
        r, v = self._row('alloc', step, size=size, calloc=calloc, has_rest=int(fsize > size))
        r['rm'], node = self._remove(addr, v + 1)
        r.update(n_addr=addr, n_size=fsize, n_prev=node[3], n_dirty=node[4], n_ver=node[5],
                 nn_size=node[6], nn_addr=node[7])
        self.put((U, addr, size, node[3], 0, v + 1, 0, 0), r, 'emit_use')
        if fsize > size:
            r['up'] = self._update_upper(addr + fsize, fsize - size, v + 3)
            r['ins'] = self._insert(addr + size, fsize - size, size, node[4], v + 3)
        self.main_ops.append((OP_CALLOC if calloc else OP_ALLOC, size, 0, addr, step))
        return addr

    def dealloc(self, step, addr):
        r, v = self._row('dealloc', step, addr=addr)
        use = self.take((U, addr))
        size, prev = use[2], use[3]
        below_free = prev > 0 and (F, addr - prev) in self.reg
        # a FREE at addr + size with prev == 0 is the first block of a contiguous area: not a neighbour
        above_free = (F, addr + size) in self.reg and self.reg[(F, addr + size)][0][3] != 0
        jb = int(below_free and not self.no_join)
        ja = int(above_free and not self.no_join)
        r.update(size=size, prev=prev, use_ver=use[5], has_below=int(prev > 0), join_below=jb, join_above=ja)
        b = r['below'] = dict(prev=0, dirty=0, ver=0, nsize=0, naddr=0, emit=0)
        a = r['above'] = dict(size=0, dirty=0, ver=0, nsize=0, naddr=0)
        if jb:
            r['rm_below'], node = self._remove(addr - prev, v + 1)
            b.update(prev=node[3], dirty=node[4], ver=node[5], nsize=node[6], naddr=node[7])
        elif prev > 0:
            if below_free:      # forgery: there is no USE token of that block
                b.update(prev=self.reg[(F, addr - prev)][0][3], ver=0)
            else:
                t = self.take((U, addr - prev))
                b.update(prev=t[3], ver=t[5])
                self.put((U, addr - prev, prev, t[3], 0, v + 1, 0, 0), b, 'emit')
        if ja:
            r['rm_above'], node = self._remove(addr + size, v + 2)
            a.update(size=node[2], dirty=node[4], ver=node[5], nsize=node[6], naddr=node[7])
        m_addr, m_size = addr - jb * prev, size + jb * prev + ja * a['size']
        if above_free and not ja:   # forgery: the free block above shown as if it were in use
            t = self.reg[(F, addr + size)][0]
            r['up'] = dict(guard=0, size=t[2], ver=0, emit=0)
        else:
            r['up'] = self._update_upper(addr + size + ja * a['size'], m_size, v + 3)
        r['ins'] = self._insert(m_addr, m_size, b['prev'] if jb else prev, 1, v + 3)
        self.main_ops.append((OP_DEALLOC, addr, 0, 0, step))

    def finish(self):
        return self.rows, self.main_ops


def random_program(seed, n_ops=400):
    rnd = random.Random(seed)
    ex = Executor()
    step = 1
    ex.create(step, 0xA010_0000, 8 * rnd.randint(64, 512), 0)
    if rnd.random() < 0.5:
        step += rnd.randint(1, 50)
        # contiguous with the first one or not: areas never join each other
        ex.create(step, ex.rows[0]['addr'] + ex.rows[0]['size'] if rnd.random() < 0.5 else 0xA020_0000,
                  8 * rnd.randint(64, 512), 1)
    live = []
    for _ in range(n_ops):
        step += rnd.randint(1, 2**20 if rnd.random() < 0.05 else 100)
        if live and rnd.random() < 0.45:
            ex.dealloc(step, live.pop(rnd.randrange(len(live))))
        else:
            addr = ex.alloc(step, 8 * rnd.randint(1, 48), calloc=rnd.randint(0, 1))
            if addr is not None:
                live.append(addr)
    rows, main_ops = ex.finish()
    return rows, main_ops


# ------------------------------------------------------------------------------------------------
# Attacks: (description, rows, main_ops, weak_opts or None)

BASE = 0xA010_0000


def attack_biggest_block():
    """alloc takes a block that is not the best fit (the prover choosing the address)."""
    ex = Executor(choose='biggest')
    ex.create(1, BASE, 256, 0)
    a = ex.alloc(2, 64)
    ex.alloc(3, 64)
    ex.dealloc(4, a)            # free: 64 @BASE and 128 @BASE+128
    ex.alloc(5, 64)             # best fit is BASE, takes BASE+128
    return ex.rows, ex.main_ops, dict(STRICT, best_fit=False)


def attack_no_join():
    """dealloc that leaves two adjacent free blocks without joining them, to change what the
    following allocs get."""
    ex = Executor(no_join=True)
    ex.create(1, BASE, 256, 0)
    a = ex.alloc(2, 64)
    b = ex.alloc(3, 64)
    ex.alloc(4, 128)
    ex.dealloc(5, a)
    ex.dealloc(6, b)            # must join with the free block below
    return ex.rows, ex.main_ops, None


def attack_future_token():
    """The program does: A = alloc@10, free(A)@20, B = alloc@30, C = alloc@40, free(B)@50.
    At step 40 the area is full (B is live). The prover builds the row of alloc@40 AFTER the row
    of free(B)@50, so it consumes tokens produced at step 50 and C gets the memory of B."""
    ex = Executor()
    last_addr = None
    for op, step in [('create', 1), ('alloc', 10), ('dealloc', 20), ('alloc', 30), ('dealloc', 50), ('alloc', 40)]:
        if op == 'create':
            ex.create(step, BASE, 64, 0)
        elif op == 'alloc':
            last_addr = ex.alloc(step, 64)
        else:
            ex.dealloc(step, last_addr)
    return ex.rows, ex.main_ops, dict(STRICT, version=False)


def attack_wrong_address():
    """Honest witness, but the alloc returns to the main another address."""
    rows, main_ops = random_program(7, 40)
    i = next(i for i, o in enumerate(main_ops) if o[0] in (OP_ALLOC, OP_CALLOC))
    op, a, b, c, step = main_ops[i]
    main_ops[i] = (op, a, b, c + 64, step)
    return rows, main_ops, None


def attack_keep_rest():
    """alloc that drops the rest of the block (has_rest = 0 with a bigger block)."""
    rows, main_ops = random_program(7, 40)
    r = next(r for r in rows if r['op'] == 'alloc' and r['has_rest'])
    r['has_rest'] = 0
    return rows, main_ops, None


def attack_duplicate_token():
    """Non boolean flag: the new FREE produced twice."""
    rows, main_ops = random_program(7, 40)
    rows[0]['ins']['emit_m'] = 2
    return rows, main_ops, None


def attack_dirty_as_clean():
    """calloc over a dirty block declared clean to skip the memset."""
    ex = Executor()
    ex.create(1, BASE, 64, 0)
    a = ex.alloc(2, 64)
    ex.dealloc(3, a)
    ex.alloc(4, 64, calloc=1)
    ex.rows[-1]['n_dirty'] = 0
    return ex.rows, ex.main_ops, None


ATTACKS = [attack_biggest_block, attack_no_join, attack_future_token, attack_wrong_address,
           attack_keep_rest, attack_duplicate_token, attack_dirty_as_clean]


def main():
    ok = True
    n_rows = joins = 0
    for seed in range(300):
        rows, main_ops = random_program(seed)
        errors, memsets = verify(rows, main_ops)
        errors += oracle(main_ops, memsets) + reference(main_ops)
        n_rows += len(rows)
        joins += sum(r['join_below'] + r['join_above'] for r in rows if r['op'] == 'dealloc')
        if errors:
            ok = False
            print(f'HONEST seed {seed}: {len(errors)} errors, first: {errors[0]}')
    print(f'honest traces: 300 programs, {n_rows} rows, {joins} joins: {"OK" if ok else "FAIL"}')

    print('\nattacks (strict = all the constraints, weak = without the named one)')
    for attack in ATTACKS:
        rows, main_ops, weak = attack()
        errors, _ = verify(rows, main_ops)
        print(f'  {attack.__name__[7:]:16} strict: ' + (f'REJECTED ({errors[0]})' if errors else 'ACCEPTED !!'))
        ok = ok and bool(errors)
        if weak is not None:
            werrors, wmemsets = verify(rows, main_ops, weak)
            harm = oracle(main_ops, wmemsets) + reference(main_ops)
            disabled = [k for k in STRICT if not weak[k]][0]
            if werrors or not harm:
                ok = False
                print(f'      weak (no {disabled}): not as expected: {werrors[:1]} {harm[:1]}')
            else:
                print(f'      weak (no {disabled}): ACCEPTED, and the program gets: {harm[0]}')
    print('\nRESULT:', 'OK' if ok else 'FAIL')
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
