#!/usr/bin/env python3
"""Model of the Allocator precompile (precompiles/allocator/pil/allocator.pil).

 - verify():   re-implements the row constraints of the PIL over Goldilocks and the balance of the
               three buses (main operations, alloc tokens, memset operations).
 - Executor:   honest allocator that fills the witness rows (first fit, lazy joins).
 - oracle():   what the guest program needs: live blocks are disjoint, inside the created areas,
               and a calloc block is zeroed.
 - attacks:    forged traces. Each one must be rejected by the PIL as written; the same forgery is
               also run against weakened variants of the checker (the literal spec) to show that
               the constraint it hits is the one that stops it.
"""
import random
import sys
from collections import Counter

P = 2**64 - 2**32 + 1
STEP_CHUNK = 16      # low chunk of the step distance; the high one has 24 bits
STEP_CHUNK_HI = 24
SIZE_CHUNK = 16
FREE, USE = 1, 2
OP_CREATE_CLEAN, OP_CREATE_DIRTY, OP_ALLOC, OP_CALLOC, OP_DEALLOC, OP_XMEMSET = 0xDB, 0xDC, 0xDD, 0xDE, 0xDF, 0xD9

COLS = ['sel_create', 'sel_alloc', 'sel_dealloc', 'sel_join', 'calloc', 'memset', 'will_be_use',
        'will_be_freed', 'free_is_newer', 'addr', 'size', 'step', 'dirty', 'free_addr', 'free_size',
        'free_dirty', 'step_diff0', 'step_diff1', 'rest0', 'rest1', 'size0', 'size1', 'out_addr',
        'out_size', 'out_dirty']

# Checker variants. STRICT is the PIL; the others drop one protection each.
STRICT = dict(consume_always=True, causality=True, nonzero=True)


def new_row(**kw):
    r = dict.fromkeys(COLS, 0)
    r.update(kw)
    return r


def verify(rows, main_ops, opts=STRICT):
    """Returns the list of errors (empty == the proof would be accepted).

    main_ops: operations assumed by the main, tuples (op, a, b, c, step).
    The memset operations assumed by the allocator are returned too, they are proved by Dma.
    """
    errors = []
    tokens = Counter()          # proves - assumes of the alloc bus
    ops = Counter()             # proves (allocator) - assumes (main) of the operation bus
    memsets = []

    for i, r in enumerate(rows):
        f = {k: v % P for k, v in r.items()}

        def eq0(name, value):
            if value % P:
                errors.append(f'row {i}: {name}')

        def rng(name, value, bits):
            if not 0 <= value % P < 2**bits:
                errors.append(f'row {i}: range {name}')

        sc, sa, sd, sj = f['sel_create'], f['sel_alloc'], f['sel_dealloc'], f['sel_join']
        sel_op = sc + sa + sd
        sel = sel_op + sj
        for b in ['sel_create', 'sel_alloc', 'sel_dealloc', 'sel_join', 'calloc', 'will_be_use',
                  'will_be_freed', 'free_is_newer', 'dirty', 'free_dirty', 'out_dirty']:
            eq0(f'{b} boolean', f[b] * (1 - f[b]))
        eq0('one op', sel * (1 - sel))
        eq0('calloc only alloc', f['calloc'] * (1 - sa))
        eq0('will_be_freed only alloc', f['will_be_freed'] * (1 - sa))
        eq0('free_is_newer only join', f['free_is_newer'] * (1 - sj))
        eq0('will_be_use only op', f['will_be_use'] * (1 - sel))
        eq0('join produces', sj * (1 - f['will_be_use']))
        eq0('dealloc free_dirty', sd * f['free_dirty'])

        if opts['causality']:
            rng('step_diff0', f['step_diff0'], STEP_CHUNK)
            rng('step_diff1', f['step_diff1'], STEP_CHUNK_HI)
            distance = 1 + f['step_diff0'] + f['step_diff1'] * 2**STEP_CHUNK
            free_step = (f['step'] - (1 - 2 * f['free_is_newer']) * distance) % P
            newest_step = (f['step'] + f['free_is_newer'] * distance) % P
        else:
            # without the check free_step is just a free witness
            free_step = r.get('free_step_raw', 0) % P
            newest_step = f['step']

        rng('rest0', f['rest0'], SIZE_CHUNK)
        rng('rest1', f['rest1'], SIZE_CHUNK)
        eq0('alloc size <= free_size',
            sa * (f['free_size'] - f['size'] - f['will_be_use'] - f['rest0'] - f['rest1'] * 2**SIZE_CHUNK))
        if opts['nonzero']:
            rng('size0', f['size0'], SIZE_CHUNK)
            rng('size1', f['size1'], SIZE_CHUNK)
            eq0('alloc size = 8 * (chunks + 1)', sa * (f['size'] - (f['size0'] + f['size1'] * 2**SIZE_CHUNK + 1) * 8))

        eq0('create out_addr', sc * (f['out_addr'] - f['addr']))
        eq0('create out_size', sc * (f['out_size'] - f['size']))
        eq0('alloc out_addr', sa * (f['out_addr'] - f['free_addr'] - f['size']))
        eq0('alloc out_size', sa * (f['out_size'] - f['free_size'] + f['size']))
        eq0('dealloc out_addr', sd * (f['out_addr'] - f['free_addr']))
        eq0('dealloc out_size', sd * (f['out_size'] - f['free_size']))
        eq0('join adjacent', sj * (f['free_addr'] - f['addr'] - f['size']))
        eq0('join out_addr', sj * (f['out_addr'] - f['addr']))
        eq0('join out_size', sj * (f['out_size'] - f['size'] - f['free_size']))
        eq0('out_dirty >= dirty', f['dirty'] * (1 - f['out_dirty']))
        eq0('out_dirty >= free_dirty', f['free_dirty'] * (1 - f['out_dirty']))
        eq0('dealloc out_dirty', sd * (1 - f['out_dirty']))
        eq0('memset', f['memset'] - f['calloc'] * f['free_dirty'])

        # alloc bus
        consume = sa + sd + sj
        if not opts['consume_always']:
            # literal spec: the alloc only consumes its FREE if will_be_use == 1
            consume = sa * f['will_be_use'] + sd + sj
        if consume:
            tokens[((FREE + sd * (USE - FREE)) % P, f['free_addr'], f['free_size'], free_step, f['free_dirty'])] -= consume
        if sj:
            tokens[(FREE, f['addr'], f['size'], f['step'], f['dirty'])] -= sj
        if f['will_be_use']:
            tokens[(FREE, f['out_addr'], f['out_size'], newest_step, f['out_dirty'])] += f['will_be_use']
        if f['will_be_freed']:
            tokens[(USE, f['free_addr'], f['size'], f['step'], 0)] += f['will_be_freed']

        # operation bus
        if sel_op:
            op = (sc * (OP_CREATE_CLEAN + f['dirty']) + sa * (OP_ALLOC + f['calloc']) + sd * OP_DEALLOC) % P
            a = (sc * f['addr'] + sa * f['size'] + sd * f['free_addr']) % P
            ops[(op, a, 0, (sc * f['size']) % P, 0, (sa * f['free_addr']) % P, 0, f['step'])] += sel_op
        if f['memset']:
            memsets.append((OP_XMEMSET, f['free_addr'], f['size'], f['step']))

    for (op, a, b, c, step) in main_ops:
        # main chunks are 32 bits: a 64-bit value is (lo, hi) and the allocator proves hi == 0
        ops[(op, a & 0xFFFFFFFF, a >> 32, b & 0xFFFFFFFF, b >> 32, c & 0xFFFFFFFF, c >> 32, step)] -= 1

    errors += [f'alloc bus unbalanced {t}: {m}' for t, m in tokens.items() if m % P]
    errors += [f'operation bus unbalanced {t}: {m}' for t, m in ops.items() if m % P]
    return errors, memsets


def oracle(main_ops, memsets):
    """Replays the main operations as the guest program sees them."""
    areas, live, errors = [], [], []
    nonzero = set()             # addresses of 8-byte words that may be non zero
    memsets = {(addr, step): size for (_, addr, size, step) in memsets}

    def words(addr, size):
        return range(addr, addr + size, 8)

    for (op, a, b, c, step) in main_ops:
        if op in (OP_CREATE_CLEAN, OP_CREATE_DIRTY):
            areas.append((a, b))
            if op == OP_CREATE_DIRTY:
                nonzero.update(words(a, b))
        elif op in (OP_ALLOC, OP_CALLOC):
            addr, size = c, a
            if not any(base <= addr and addr + size <= base + asize for base, asize in areas):
                errors.append(f'step {step}: block {addr:#x}+{size} outside the created areas')
            for laddr, lsize in live:
                if addr < laddr + lsize and laddr < addr + size:
                    errors.append(f'step {step}: block {addr:#x}+{size} overlaps live block {laddr:#x}+{lsize}')
            if (addr, step) in memsets:
                nonzero.difference_update(words(addr, memsets[(addr, step)]))
            if op == OP_CALLOC and any(w in nonzero for w in words(addr, size)):
                errors.append(f'step {step}: calloc block {addr:#x}+{size} not zeroed')
            live.append((addr, size))
            nonzero.update(words(addr, size))       # the program writes all over its block
        elif op == OP_DEALLOC:
            # the program releases its oldest live block at that address
            block = next((blk for blk in live if blk[0] == a), None)
            if block is None:
                errors.append(f'step {step}: dealloc of non live address {a:#x}')
            else:
                live.remove(block)
    return errors


class Executor:
    """Honest allocator. Free blocks are kept as the tokens that are live in the bus, each one
    with the row that produced it, to set will_be_use/will_be_freed when it is consumed."""

    def __init__(self):
        self.rows, self.main_ops = [], []
        self.free = []          # [addr, size, step, dirty, producer_row]
        self.used = {}          # addr -> (size, step, producer_row)

    @staticmethod
    def _older(r, step, free_step):
        d = step - free_step - 1
        r['step_diff0'], r['step_diff1'] = d & (2**STEP_CHUNK - 1), d >> STEP_CHUNK

    def create(self, step, addr, size, dirty):
        r = new_row(sel_create=1, addr=addr, size=size, step=step, dirty=dirty,
                    out_addr=addr, out_size=size, out_dirty=dirty)
        self.rows.append(r)
        self.free.append([addr, size, step, dirty, r])
        self.main_ops.append((OP_CREATE_DIRTY if dirty else OP_CREATE_CLEAN, addr, size, 0, step))

    def _join(self, lo, hi):
        newer = 1 if hi[2] > lo[2] else 0
        r = new_row(sel_join=1, will_be_use=1, addr=lo[0], size=lo[1], step=lo[2], dirty=lo[3],
                    free_addr=hi[0], free_size=hi[1], free_dirty=hi[3], free_is_newer=newer,
                    out_addr=lo[0], out_size=lo[1] + hi[1], out_dirty=lo[3] | hi[3])
        d = abs(hi[2] - lo[2]) - 1
        r['step_diff0'], r['step_diff1'] = d & (2**STEP_CHUNK - 1), d >> STEP_CHUNK
        lo[4]['will_be_use'] = hi[4]['will_be_use'] = 1
        self.rows.append(r)
        return [lo[0], lo[1] + hi[1], max(lo[2], hi[2]), lo[3] | hi[3], r]

    def _find(self, size):
        """First fit over single tokens; if none, joins the first run of adjacent tokens that
        fits. Joins are lazy because the token of a join must be consumed."""
        self.free.sort(key=lambda t: t[0])
        for i, t in enumerate(self.free):
            if t[1] >= size:
                return self.free.pop(i)
        for i in range(len(self.free)):
            total, j = self.free[i][1], i
            while total < size and j + 1 < len(self.free) and \
                    self.free[j][0] + self.free[j][1] == self.free[j + 1][0]:
                j += 1
                total += self.free[j][1]
            if total >= size:
                run = self.free[i:j + 1]
                del self.free[i:j + 1]
                block = run[0]
                for t in run[1:]:
                    block = self._join(block, t)
                return block
        return None

    def alloc(self, step, size, calloc=0):
        t = self._find(size)
        if t is None:
            return None
        addr, fsize, fstep, fdirty, producer = t
        producer['will_be_use'] = 1
        r = new_row(sel_alloc=1, calloc=calloc, memset=calloc & fdirty, size=size, step=step,
                    free_addr=addr, free_size=fsize, free_dirty=fdirty,
                    size0=(size // 8 - 1) & 0xFFFF, size1=(size // 8 - 1) >> 16,
                    out_addr=addr + size, out_size=fsize - size, out_dirty=fdirty)
        self._older(r, step, fstep)
        r['rest0'], r['rest1'] = (fsize - size) & 0xFFFF, (fsize - size) >> 16
        self.rows.append(r)
        self.used[addr] = (size, step, r)
        if fsize > size:
            # the rest, pending of being used: the consumer will set will_be_use and fix the rest
            self.free.append([addr + size, fsize - size, step, fdirty, r])
        self.main_ops.append((OP_CALLOC if calloc else OP_ALLOC, size, 0, addr, step))
        return addr

    def dealloc(self, step, addr):
        size, astep, producer = self.used.pop(addr)
        producer['will_be_freed'] = 1
        r = new_row(sel_dealloc=1, step=step, free_addr=addr, free_size=size,
                    out_addr=addr, out_size=size, out_dirty=1)
        self._older(r, step, astep)
        self.rows.append(r)
        self.free.append([addr, size, step, 1, r])
        self.main_ops.append((OP_DEALLOC, addr, 0, 0, step))

    def finish(self):
        # will_be_use on an alloc row takes one unit of the rest (free_size = size + wbu + rest)
        for r in self.rows:
            if r['sel_alloc']:
                rest = r['free_size'] - r['size'] - r['will_be_use']
                r['rest0'], r['rest1'] = rest & 0xFFFF, rest >> 16
        return self.rows, self.main_ops


def random_program(seed, n_ops=400):
    rnd = random.Random(seed)
    ex = Executor()
    step = 1
    ex.create(step, 0xA010_0000, 8 * rnd.randint(64, 512), 0)
    if rnd.random() < 0.5:
        step += rnd.randint(1, 50)
        ex.create(step, 0xA020_0000, 8 * rnd.randint(64, 512), 1)
    live = []
    for _ in range(n_ops):
        step += rnd.randint(1, 2**20 if rnd.random() < 0.05 else 100)
        if live and rnd.random() < 0.45:
            ex.dealloc(step, live.pop(rnd.randrange(len(live))))
        else:
            addr = ex.alloc(step, 8 * rnd.randint(1, 48), calloc=rnd.randint(0, 1))
            if addr is not None:
                live.append(addr)
    return ex.finish()


# ------------------------------------------------------------------------------------------------
# Attacks. Each returns (rows, main_ops, opts_that_would_accept_it or None).
# main_ops is what the main executes; the prover only chooses the rows and the alloc results.

BASE = 0xA010_0000


def older(r, step, free_step):
    d = (step - free_step - 1) % P
    r['step_diff0'], r['step_diff1'] = d & (2**STEP_CHUNK - 1), d >> STEP_CHUNK
    r['free_step_raw'] = free_step
    return r


def alloc_row(step, size, faddr, fsize, fstep, fdirty=0, wbu=0, wbf=0, calloc=0):
    rest = fsize - size - wbu
    return older(new_row(sel_alloc=1, calloc=calloc, memset=calloc & fdirty, size=size, step=step,
                         free_addr=faddr, free_size=fsize, free_dirty=fdirty, will_be_use=wbu,
                         will_be_freed=wbf, size0=(size // 8 - 1) & 0xFFFF, size1=((size // 8 - 1) >> 16) & 0xFFFF,
                         rest0=rest & 0xFFFF, rest1=(rest >> 16) & 0xFFFF,
                         out_addr=faddr + size, out_size=fsize - size, out_dirty=fdirty), step, fstep)


def dealloc_row(step, addr, size, astep, wbu=0):
    return older(new_row(sel_dealloc=1, step=step, free_addr=addr, free_size=size, will_be_use=wbu,
                         out_addr=addr, out_size=size, out_dirty=1), step, astep)


def create_row(step, addr, size, dirty=0, wbu=1):
    return new_row(sel_create=1, addr=addr, size=size, step=step, dirty=dirty, will_be_use=wbu,
                   out_addr=addr, out_size=size, out_dirty=dirty)


def attack_alloc_without_consuming():
    """Literal spec: '- FREE' gated by will_be_use. Two allocs get the same block."""
    rows = [create_row(1, BASE, 64),
            alloc_row(2, 64, BASE, 64, 1, wbu=0),       # consumes nothing in the literal spec
            alloc_row(3, 64, BASE, 64, 1, wbu=0)]
    rows[0]['will_be_use'] = 0
    main = [(OP_CREATE_CLEAN, BASE, 64, 0, 1), (OP_ALLOC, 64, 0, BASE, 2), (OP_ALLOC, 64, 0, BASE, 3)]
    return rows, main, dict(STRICT, consume_always=False)


def attack_cycle():
    """No causality: alloc@2 consumes the FREE that dealloc@5 of that same alloc produces. The
    block is outside any created area (here it lands over another live block)."""
    rows = [create_row(1, BASE, 64),
            alloc_row(2, 64, BASE, 64, 5, fdirty=1, wbf=1),     # FREE(BASE, 64, 5, 1) from the future
            alloc_row(3, 64, BASE, 64, 1),                      # the real block
            dealloc_row(5, BASE, 64, 2, wbu=1)]
    main = [(OP_CREATE_CLEAN, BASE, 64, 0, 1), (OP_ALLOC, 64, 0, BASE, 2), (OP_ALLOC, 64, 0, BASE, 3),
            (OP_DEALLOC, BASE, 0, 0, 5)]
    return rows, main, dict(STRICT, causality=False)


def attack_zero_size():
    """alloc(0) and alloc(64) share the address; dealloc of the empty one releases the full one,
    that is handed out again while still live."""
    rows = [create_row(1, BASE, 64),
            alloc_row(2, 0, BASE, 64, 1, wbu=1),                # USE(BASE, 0, 2) not produced
            alloc_row(3, 64, BASE, 64, 2, wbf=1),               # USE(BASE, 64, 3)
            dealloc_row(4, BASE, 64, 3, wbu=1),                 # program frees the EMPTY block
            alloc_row(5, 64, BASE, 64, 4, fdirty=1)]            # same memory, the full one is live
    main = [(OP_CREATE_CLEAN, BASE, 64, 0, 1), (OP_ALLOC, 0, 0, BASE, 2), (OP_ALLOC, 64, 0, BASE, 3),
            (OP_DEALLOC, BASE, 0, 0, 4), (OP_ALLOC, 64, 0, BASE, 5)]
    return rows, main, dict(STRICT, nonzero=False)


def attack_size_bigger_than_free():
    """alloc of 128 bytes over a 64-byte block (free_size - size wraps the field)."""
    rows = [create_row(1, BASE, 64), alloc_row(2, 128, BASE, 64, 1)]
    main = [(OP_CREATE_CLEAN, BASE, 64, 0, 1), (OP_ALLOC, 128, 0, BASE, 2)]
    return rows, main, None


def attack_size_alias():
    """alloc(p + 64): as a field element the size is 64. The program believes it has ~2**64 bytes."""
    rows = [create_row(1, BASE, 64), alloc_row(2, 64, BASE, 64, 1)]
    main = [(OP_CREATE_CLEAN, BASE, 64, 0, 1), (OP_ALLOC, P + 64, 0, BASE, 2)]
    return rows, main, None


def attack_double_free():
    rows = [create_row(1, BASE, 64), alloc_row(2, 64, BASE, 64, 1, wbf=1),
            dealloc_row(3, BASE, 64, 2), dealloc_row(4, BASE, 64, 2, wbu=1),
            alloc_row(5, 64, BASE, 64, 4, fdirty=1)]
    main = [(OP_CREATE_CLEAN, BASE, 64, 0, 1), (OP_ALLOC, 64, 0, BASE, 2), (OP_DEALLOC, BASE, 0, 0, 3),
            (OP_DEALLOC, BASE, 0, 0, 4), (OP_ALLOC, 64, 0, BASE, 5)]
    return rows, main, None


def attack_dirty_as_clean():
    """calloc over a block released by a dealloc, declared clean to skip the memset."""
    rows = [create_row(1, BASE, 64), alloc_row(2, 64, BASE, 64, 1, wbf=1),
            dealloc_row(3, BASE, 64, 2, wbu=1),
            alloc_row(4, 64, BASE, 64, 3, fdirty=0, calloc=1)]
    main = [(OP_CREATE_CLEAN, BASE, 64, 0, 1), (OP_ALLOC, 64, 0, BASE, 2), (OP_DEALLOC, BASE, 0, 0, 3),
            (OP_CALLOC, 64, 0, BASE, 4)]
    return rows, main, None


def attack_join_cleans():
    """join of a dirty and a clean block producing a clean one."""
    rows = [create_row(1, BASE, 64, dirty=1), create_row(2, BASE + 64, 64, dirty=0),
            new_row(sel_join=1, will_be_use=1, addr=BASE, size=64, step=1, dirty=1, free_addr=BASE + 64,
                    free_size=64, free_dirty=0, free_is_newer=1, out_addr=BASE, out_size=128, out_dirty=0),
            alloc_row(3, 128, BASE, 128, 2, fdirty=0, calloc=1)]
    main = [(OP_CREATE_DIRTY, BASE, 64, 0, 1), (OP_CREATE_CLEAN, BASE + 64, 64, 0, 2), (OP_CALLOC, 128, 0, BASE, 3)]
    return rows, main, None


def attack_join_backdated():
    """join of FREE(BASE, 64, 9) [produced by the dealloc of step 9] with FREE(BASE+64, 64, 2)
    keeping the older step 2, so the alloc of step 3 takes the whole area while the first block
    is still live. The PIL forces the newest step (9) in the joined token."""
    rows = [create_row(1, BASE, 128),
            alloc_row(2, 64, BASE, 128, 1, wbu=1, wbf=1),
            alloc_row(3, 128, BASE, 128, 2, fdirty=1),
            dealloc_row(9, BASE, 64, 2, wbu=1),
            new_row(sel_join=1, will_be_use=1, addr=BASE, size=64, step=9, dirty=1, free_addr=BASE + 64,
                    free_size=64, free_dirty=0, free_is_newer=0, step_diff0=6,      # free_step = 2
                    out_addr=BASE, out_size=128, out_dirty=1)]
    main = [(OP_CREATE_CLEAN, BASE, 128, 0, 1), (OP_ALLOC, 64, 0, BASE, 2), (OP_ALLOC, 128, 0, BASE, 3),
            (OP_DEALLOC, BASE, 0, 0, 9)]
    return rows, main, None


def attack_negative_selector():
    """sel_dealloc = -1 turns the assume of a USE into a prove: a USE token out of nothing, then
    dealloc it to get a FREE block anywhere."""
    rows = [create_row(1, BASE, 64, wbu=0),
            older(new_row(sel_dealloc=P - 1, step=2, free_addr=BASE + 0x1000, free_size=64,
                          out_addr=BASE + 0x1000, out_size=64, out_dirty=1), 2, 1),
            dealloc_row(3, BASE + 0x1000, 64, 1, wbu=1),
            alloc_row(4, 64, BASE + 0x1000, 64, 3, fdirty=1)]
    main = [(OP_CREATE_CLEAN, BASE, 64, 0, 1), (OP_ALLOC, 64, 0, BASE + 0x1000, 4)]
    return rows, main, None


ATTACKS = [attack_alloc_without_consuming, attack_cycle, attack_zero_size, attack_size_bigger_than_free,
           attack_size_alias, attack_double_free, attack_dirty_as_clean, attack_join_cleans,
           attack_join_backdated, attack_negative_selector]


def main():
    ok = True

    n_rows = n_joins = 0
    for seed in range(300):
        rows, main_ops = random_program(seed)
        errors, memsets = verify(rows, main_ops)
        errors += oracle(main_ops, memsets)
        n_rows += len(rows)
        n_joins += sum(r['sel_join'] for r in rows)
        if errors:
            ok = False
            print(f'HONEST seed {seed}: {len(errors)} errors, first: {errors[0]}')
    print(f'honest traces: 300 programs, {n_rows} rows ({n_joins} joins): {"OK" if ok else "FAIL"}')

    print('\nattacks (PIL = constraints as written, weak = same checker without the named protection)')
    for attack in ATTACKS:
        rows, main_ops, weak = attack()
        errors, memsets = verify(rows, main_ops)
        line = f'  {attack.__name__[7:]:28} PIL: {"REJECTED" if errors else "ACCEPTED !!"}'
        if not errors:
            ok = False
        else:
            line += f' ({errors[0]})'
        print(line)
        if weak is not None:
            werrors, wmemsets = verify(rows, main_ops, weak)
            broken = oracle(main_ops, wmemsets)
            disabled = [k for k in STRICT if not weak[k]][0]
            if werrors or not broken:
                ok = False
                print(f'      weak (no {disabled}): attack did not work as expected: {werrors[:1]} {broken[:1]}')
            else:
                print(f'      weak (no {disabled}): ACCEPTED, and the program gets: {broken[0]}')

    print('\nRESULT:', 'OK' if ok else 'FAIL')
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
