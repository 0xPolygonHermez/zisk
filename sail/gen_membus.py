#!/usr/bin/env python3
"""Generate lean/Proofs/MemBus.lean: no AIR but Main sends a register tuple.

And the memory bus over every AIR, reduced to Main's register chain.

Main keeps the registers as a chain of tuples `(3, reg, step, 8, value)` on
the memory bus (10). The chain argument (lean/Proofs/MemCheck.lean) needs the
tuples with op 3 to be Main's alone. This script reads every argument on bus
10 of every AIR but Main from the generated AIRs (build/pil/Pil/*.lean), reads
its op element (the tuple's first) as a linear form over the AIR's columns,
and writes, per argument, a theorem `<Air>_<bus>_not_reg`: at a row where the
argument sends, its op is not 3. How depends on the form:

* a constant, or a constant plus selectors times coefficients, each selector a
  bit by the AIR's booleanity constraint: `op_ne` enumerates the values;
* the same, where some values are 3 unless the selectors are exclusive, and the
  AIR constrains their sum to a bit (`<Air>_sel`): `op_ne_sel`;
* a form over fixed columns only (the precompiles' clocks): the op is 1 or 2 at
  every row of the setup's fixed data, a hypothesis `FixedOp` that
  `check_tables.py` checks;
* the Rom's data rows, whose op is their `flags`: 2 or 4, the program's data,
  a hypothesis `RomDataFlags`.

The script stops on an op of any other form.

Usage: gen_membus.py   (from sail/; `make membus`)
"""

import glob
import itertools
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PIL = os.path.join(HERE, 'build', 'pil', 'Pil')
OUT = os.path.join(HERE, 'lean', 'Proofs', 'MemBus.lean')
OUT_REGS = os.path.join(HERE, 'lean', 'Proofs', 'MainRegs.lean')
CENSUS = os.path.join(HERE, 'build', 'membus.json')

MEMORY_BUS = '10'
REG_OP = 3

ATOM = r"\(t (?:i|\(i - \d+\))\)\.[A-Za-z_][A-Za-z_0-9']*(?: \d+)*"
DEF = r"^def (\S+) \(x : Ctx F\) \(t : Trace F\) \(i : Int\) : (?:F|Prop) :=\n((?:  .*\n)+)"


def tokenize(s):
    toks = []
    pos = 0
    pat = re.compile(r"\s*(?:(" + ATOM + r")|(\d+)|([A-Za-z_][A-Za-z_0-9']*) x t i|([-+*()]))")
    while pos < len(s.rstrip()):
        m = pat.match(s, pos)
        if not m:
            raise ValueError(f"cannot read {s[pos:]!r}")
        pos = m.end()
        if m.group(1):
            toks.append(('atom', m.group(1)))
        elif m.group(2):
            toks.append(('num', int(m.group(2))))
        elif m.group(3):
            toks.append(('def', m.group(3)))
        else:
            toks.append(('op', m.group(4)))
    return toks


class Linear:
    """A linear form: constant + {atom: coefficient}."""

    def __init__(self, c=0, terms=None):
        self.c, self.terms = c, dict(terms or {})

    def __add__(self, o):
        t = dict(self.terms)
        for a, k in o.terms.items():
            t[a] = t.get(a, 0) + k
        return Linear(self.c + o.c, {a: k for a, k in t.items() if k})

    def scale(self, k):
        return Linear(self.c * k, {a: v * k for a, v in self.terms.items()})

    def __mul__(self, o):
        if not self.terms:
            return o.scale(self.c)
        if not o.terms:
            return self.scale(o.c)
        raise ValueError("not linear")


def parse(s, defs):
    toks = tokenize(s)
    pos = [0]

    def peek():
        return toks[pos[0]] if pos[0] < len(toks) else (None, None)

    def take():
        pos[0] += 1
        return toks[pos[0] - 1]

    def expr():
        v = term()
        while peek() in (('op', '+'), ('op', '-')):
            o = take()[1]
            w = term()
            v = v + (w if o == '+' else w.scale(-1))
        return v

    def term():
        v = factor()
        while peek() == ('op', '*'):
            take()
            v = v * factor()
        return v

    def factor():
        k, x = take()
        if k == 'num':
            return Linear(x)
        if k == 'atom':
            return Linear(0, {x: 1})
        if k == 'def':
            return parse(defs[x], defs)
        if (k, x) == ('op', '('):
            v = expr()
            assert take() == ('op', ')')
            return v
        if (k, x) == ('op', '-'):
            return factor().scale(-1)
        raise ValueError(f"unexpected {x}")

    v = expr()
    if pos[0] != len(toks):
        raise ValueError("trailing tokens")
    return v


def subset_sums(cs):
    return {sum(c for c, b in zip(cs, bits) if b) for bits in itertools.product((0, 1), repeat=len(cs))}


def closure(names, defs):
    seen, todo = [], list(names)
    while todo:
        n = todo.pop()
        if n in seen or n not in defs:
            continue
        seen.append(n)
        todo.extend(re.findall(r"\b([A-Za-z_][A-Za-z_0-9']*) x t i", defs[n]))
    return seen


def bool_name(atom):
    m = re.fullmatch(r"\(t i\)\.(\w+)((?: \d+)*)", atom)
    if not m:
        return None
    return 'bool_' + m.group(1) + ''.join('_' + d for d in m.group(2).split())


def lit(n):
    return str(n) if n >= 0 else f"-{-n}"


def main():
    theorems, census, airs, perair = [], [], [], {}
    for f in sorted(glob.glob(os.path.join(PIL, '*.lean'))):
        air = os.path.basename(f)[:-5]
        if air in ('Main', 'Prelude'):
            continue
        s = open(f).read()
        defs = {m.group(1): ' '.join(m.group(2).split()) for m in re.finditer(DEF, s, re.M)}
        fixed = set(re.findall(r"^  (\w+) : .*\(fixed\)$", s, re.M))
        bools = set(re.findall(r"^theorem everyRow\.(bool_\w+)", s, re.M))
        consts = {m.group(1): int(m.group(2)) for m in
                  re.finditer(r"^def (c\d+) \(x : Ctx F\) \(t : Trace F\) \(i : Int\) : Prop :=\n  (.*)$", s, re.M)
                  if False}
        found = False
        for m in re.finditer(r"^def (bus\d+) : BusArg F \(Ctx F\) \(Trace F\) where\n  opids := \[([^\]]*)\]\n"
                             r"  bus := .*\n  piop := \.(\w+)\n  side := \.(\w+)\n  reps := (.*)\n(?:  once := \w+\n)?  tuple := \[\n"
                             r"    fun x t i => (.*?),?\n", s, re.M):
            bus, opids, piop, side, reps, op = m.groups()
            if MEMORY_BUS not in opids.split(', '):
                continue
            found = True
            name = f"{air}_{bus}_not_reg"
            arg = f"{air}.{bus}"
            form = parse(op, defs)
            atoms = list(form.terms)
            entry = dict(air=air, bus=bus, side=side, op=op, form=[form.c, form.terms])
            unfold = ', '.join([f"{air}.{bus}"] + [f"{air}.{n}" for n in closure(
                re.findall(r"\b([A-Za-z_][A-Za-z_0-9']*) x t i", op), defs)])
            head = (f"theorem {name} {{x : {air}.Ctx F}} {{t : {air}.Trace F}} {{i : Int}}\n"
                    f"    (h : {air}.everyRow x t i) (hr : {arg}.reps x t i ≠ 0)")
            concl = f" :\n    opOf {arg} x t i ≠ (({REG_OP} : Int) : F) := by\n"
            fixed_atoms = [a for a in atoms if re.search(r"\.(\w+)", a).group(1) in fixed]
            if air == 'Rom' and op.strip() == '(t i).flags + 0':
                entry['kind'] = 'romdata'
                theorems.append(head + f"\n    (hd : RomDataFlags t)" + concl +
                    f"  have hflags := hd i (by simpa [{arg}, Lean.Grind.Semiring.add_zero] using hr)\n"
                    f"  simp only [opOf, {unfold}, List.headD, Lean.Grind.Semiring.add_zero]\n"
                    f"  exact flags_ne_three hflags\n")
            elif atoms and len(fixed_atoms) == len(atoms):
                entry['kind'] = 'fixed'
                theorems.append(head + f"\n    (hfix : FixedOp {arg} x t i)" + concl +
                    f"  exact fixedOp_ne hfix\n")
            elif fixed_atoms:
                sys.exit(f"gen_membus: {arg} mixes fixed and witness columns: {op}")
            else:
                names = [bool_name(a) for a in atoms]
                if any(n is None or n not in bools for n in names):
                    sys.exit(f"gen_membus: {arg}: a selector without booleanity: {op}")
                cs = [form.terms[a] for a in atoms]
                vals = {form.c + z for z in subset_sums(cs)}
                bool_cases = '\n'.join(f"    · exact h.{n}" for n in names)
                pat = ' | '.join(['rfl'] * len(atoms))
                if REG_OP not in vals:
                    entry['kind'] = 'bits'
                    ss = '[' + ', '.join(atoms) + ']'
                    cl = '[' + ', '.join(lit(c) for c in cs) + ']'
                    body = (f"  apply op_ne (F := F) {lit(form.c)} {cl} {lit(form.c)} {cl} {ss} {REG_OP} _\n"
                            f"    (by cast_lits) (by cast_lits) rfl\n")
                    if atoms:
                        body += (f"  · simp only [List.mem_cons, List.not_mem_nil, or_false]\n"
                                 f"    rintro s ({pat})\n{bool_cases}\n")
                    else:
                        body += "  · simp\n"
                    body += (f"  · simp only [opOf, {unfold}, List.headD, linF]\n"
                             f"    grind\n  · decide\n  · decide\n")
                    theorems.append(head + concl + body)
                else:
                    # Exclusive selectors: the AIR's `<Air>_sel`, a sum of selectors, is a bit.
                    seln = f"{air}_sel"
                    if seln not in defs:
                        sys.exit(f"gen_membus: {arg} can be 3 and {air} has no `sel`: {op}")
                    sform = parse(defs[seln], defs)
                    cons = [c for c, body in re.findall(DEF, s, re.M)
                            if re.fullmatch(r"c\d+", c) and ' '.join(body.split()) in
                            (f"{seln} x t i * (1 - {seln} x t i) = 0",
                             f"{seln} x t i * ({seln} x t i - 1) = 0")]
                    if not cons:
                        sys.exit(f"gen_membus: {air}: no booleanity of `sel`")
                    allatoms = list(dict.fromkeys(atoms + list(sform.terms)))
                    names = [bool_name(a) for a in allatoms]
                    if any(n is None or n not in bools for n in names):
                        sys.exit(f"gen_membus: {arg}: a selector without booleanity: {defs[seln]}")
                    cds = [(form.terms.get(a, 0), sform.terms.get(a, 0)) for a in allatoms]
                    for bits in itertools.product((0, 1), repeat=len(allatoms)):
                        o = form.c + sum(c for (c, _), b in zip(cds, bits) if b)
                        sv = sform.c + sum(d for (_, d), b in zip(cds, bits) if b)
                        if sv in (0, 1) and o == REG_OP:
                            sys.exit(f"gen_membus: {arg} can be 3 with `sel` a bit: {op}")
                    entry['kind'] = 'sel'
                    ss = '[' + ', '.join(allatoms) + ']'
                    cdl = '[' + ', '.join(f"({lit(c)}, {lit(d)})" for c, d in cds) + ']'
                    fl = '[' + ', '.join(lit(c) for c, _ in cds) + ']'
                    el = '[' + ', '.join(lit(d) for _, d in cds) + ']'
                    bool_cases = '\n'.join(f"    · exact h.{n}" for n in names)
                    pat = ' | '.join(['rfl'] * len(allatoms))
                    sunf = ', '.join(f"{air}.{n}" for n in closure([seln], defs))
                    body = (f"  have hs : {air}.{seln} x t i * (1 - {air}.{seln} x t i) = 0 := by\n"
                            f"    have e := h.{cons[0]}\n"
                            f"    simp only [{air}.{cons[0]}] at e\n"
                            f"    grind\n"
                            f"  apply op_ne_sel (F := F) {lit(form.c)} {lit(sform.c)} {cdl} {lit(form.c)} "
                            f"{lit(sform.c)} {fl} {el} {ss} {REG_OP} _ ({air}.{seln} x t i)\n"
                            f"    (by cast_lits) (by cast_lits) (by cast_lits) (by cast_lits) rfl\n"
                            f"  · simp only [List.mem_cons, List.not_mem_nil, or_false]\n"
                            f"    rintro s ({pat})\n{bool_cases}\n"
                            f"  · simp only [opOf, {unfold}, List.headD, linF]\n"
                            f"    grind\n"
                            f"  · simp only [{sunf}, linF]\n"
                            f"    grind\n"
                            f"  · exact hs\n  · decide\n  · decide\n")
                    theorems.append(head + concl + body)
            census.append(entry)
            perair.setdefault(air, []).append((bus, entry['kind']))
        if found:
            airs.append(air)
    os.makedirs(os.path.dirname(CENSUS), exist_ok=True)
    json.dump(census, open(CENSUS, 'w'), indent=1)
    imports = '\n'.join(f"import Pil.{a}" for a in airs)
    kinds = {}
    for e in census:
        kinds[e['kind']] = kinds.get(e['kind'], 0) + 1
    text = HEAD.format(imports=imports, n=len(census), nairs=len(airs),
                       kinds=', '.join(f"{v} {k}" for k, v in sorted(kinds.items()))) + \
        '\n'.join(theorems) + "\nend Provers\n\n" + system(airs, perair) + "\nend Correspondence\n"
    open(OUT, 'w').write(text)
    npairs = main_regs()
    print(f"wrote {os.path.relpath(OUT_REGS, HERE)}: {npairs} pairs of Main's, each good")
    print(f"wrote {os.path.relpath(OUT, HERE)}: {len(census)} arguments in {len(airs)} AIRs "
          f"({', '.join(f'{v} {k}' for k, v in sorted(kinds.items()))})")


def main_pairs():
    """Main's register-access pairs on the memory bus, with each one's range check.

    Lane `l`, slot `k` (a, b, store): the argument proving the access before,
    the one assuming the access (or a memory access), the range check of the
    gap. Flush window `f`, register `r`: the same three, sent once."""
    s = open(os.path.join(PIL, 'Main.lean')).read()
    docs = [(m.group(5), m.group(2).strip(), m.group(3), m.group(4)) for m in re.finditer(
        r"^/-- ([\w ]+) on bus ([\d, ]+), (\w+): `(.*?)` -/\ndef (bus\d+) ", s, re.M)]
    by_doc = {(b, side, tup): name for name, b, side, tup in docs}
    steps = int(re.search(r"^def Main_segment_next_step .*\n  \(x.Main_segment_initial_step \+ (\d+)\)",
                          s, re.M).group(1))
    lanes, flushes, windows = [], [], {}
    slots = [('a', 'a_offset_imm0', 'a_reg_prev_mem_step', 'a_mem_step'),
             ('b', 'b_offset_imm0', 'b_reg_prev_mem_step', 'b_mem_step'),
             ('store', 'store_offset', 'store_reg_prev_mem_step', 'store_mem_step')]
    for l in range(4):
        for k, (nm, addr, prev, mstep) in enumerate(slots):
            pre = next(n for n, b, side, tup in docs if b == '10' and side == 'proves'
                       and tup.startswith(f"(3, {addr}[{l}], {prev}[{l}], 8,"))
            idx = [d[0] for d in docs].index(pre)
            cur = docs[idx + 1][0]
            assert docs[idx + 1][2] == 'assumes' and f"{mstep}[{l}]" in docs[idx + 1][3], (pre, cur)
            rng = by_doc[('102', 'assumes', f"(({mstep}[{l}] - {prev}[{l}]) - 1)")]
            lanes.append((l, k, pre, cur, rng))
    for f in range(8):
        m = re.search(rf"^def Main_flush_reg_mem_step_{f} .*\n  1 \+ 4 \* \(x.Main_segment_initial_step \+ (\d+) - 1\) \+ 4 - 1",
                      s, re.M)
        if not m:
            break
        windows[f] = int(m.group(1))
        for r in range(39):
            pre = by_doc[('10', 'proves', f"(3, {r + 1}, Main.last_reg_mem_step[{f}][{r}], 8, "
                          f"Main.last_reg_value[{f}][{r}][0], Main.last_reg_value[{f}][{r}][1])")]
            cur = by_doc[('10', 'assumes', f"(3, {r + 1}, flush_reg_mem_step[{f}], 8, "
                          f"Main.last_reg_value[{f}][{r}][0], Main.last_reg_value[{f}][{r}][1])")]
            rng = by_doc[('102', 'assumes', f"((flush_reg_mem_step[{f}] - Main.last_reg_mem_step[{f}][{r}]) - 1)")]
            flushes.append((f, r, pre, cur, rng))
    used = [x for _, _, p, c, _ in lanes + flushes for x in (p, c)]
    on10 = main_buses()
    if sorted(used) != sorted(on10) or len(set(used)) != len(used):
        sys.exit(f"gen_membus: Main's pairs cover {len(set(used))} of its {len(on10)} memory-bus arguments")
    return lanes, flushes, windows, steps


def main_buses():
    """Main's arguments on the memory bus, in hint order."""
    s = open(os.path.join(PIL, 'Main.lean')).read()
    return [m.group(1) for m in re.finditer(r"^def (bus\d+) : BusArg F \(Ctx F\) \(Trace F\) where\n"
                                            r"  opids := \[([^\]]*)\]", s, re.M)
            if MEMORY_BUS in m.group(2).split(', ')]


def system(airs, perair):
    """The instances, their senders, and the reduction of a sound bus to Main's."""
    out = [SYSTEM_HEAD]
    fields, senders, nevers = [], [], []
    for air in airs:
        inst = f"{air}OnMem"
        extra = []
        for bus, kind in perair[air]:
            if kind == 'fixed':
                extra.append(f"  fix_{bus} : ∀ j, FixedOp {air}.{bus} x t j")
        if any(k == 'romdata' for _, k in perair[air]):
            extra.append("  data : RomDataFlags t")
        out.append(f"""/-- An instance of `{air}` on the memory bus: its trace, its constraints at every
row, and the data its ops rest on. -/
structure {inst} (F : Type) [Lean.Grind.Field F] where
  x : {air}.Ctx F
  t : {air}.Trace F
  rows : ∀ j, {air}.everyRow x t j{''.join(chr(10) + e for e in extra)}

/-- `{air}`'s arguments on the memory bus. -/
def {inst}.senders (I : {inst} F) : List (Sender F) :=
  [{', '.join(f"⟨⟨{air}.{b}, I.x, I.t⟩, {air}.numRows⟩" for b, _ in perair[air])}]

theorem {inst}.never (I : {inst} F) :
    ∀ s ∈ I.senders, ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ ((3 : Int) : F) := by
  intro s hs j hr
  rw [headD_tupleOf]
  simp only [{inst}.senders, List.mem_cons, List.not_mem_nil, or_false] at hs
  rcases hs with {' | '.join(['rfl'] * len(perair[air]))}
""")
        for bus, kind in perair[air]:
            args = "(I.rows j) hr"
            if kind == 'fixed':
                args += f" (I.fix_{bus} j)"
            if kind == 'romdata':
                args += " I.data"
            out.append(f"  · exact {air}_{bus}_not_reg {args}\n")
        out.append("\n")
        fname = air[0].lower() + air[1:]
        fields.append(f"  {fname} : List ({inst} F)")
        senders.append(f"(S.{fname}.flatMap {inst}.senders)")
        nevers.append(f"never_flatMap (fun I _ => {inst}.never I)")
    prov = '\n'.join(('  ' if i == 0 else '    ') + x + (' ++ (' if i < len(senders) - 1 else '')
                     for i, x in enumerate(senders)) + ')' * (len(senders) - 1)
    nev = ('  ' + ' <|\n    '.join(f"never_append ({n})" for n in nevers[:-1]) + ' <|\n    ' + nevers[-1])
    lanes, flushes, windows, steps = main_pairs()
    tags = [f".lane {l} {k}" for l, k, *_ in lanes] + [f".flush {f} {r}" for f, r, *_ in flushes]
    arms = lambda col: '\n'.join([f"  | .lane {l} {k} => Main.{t[col]}" for t in lanes for l, k in [t[:2]]] +
                                  [f"  | .flush {f} {r} => Main.{t[col]}" for t in flushes for f, r in [t[:2]]] +
                                  [f"  | _ => Main.{lanes[0][col]}"])
    plist = tags
    wins = list(windows.values())
    if wins != [(f + 1) * wins[0] for f in range(len(wins))]:
        sys.exit(f"gen_membus: flush windows not evenly spaced: {wins}")
    out.append(SYSTEM_TAIL.format(fields='\n'.join(fields), senders=prov, never=nev,
                                  nmain=2 * len(plist), npairs=len(plist), nlane=len(lanes),
                                  nflush=len(flushes), nwin=len(windows), steps=steps,
                                  window=wins[0],
                                  taglist=', '.join(plist), prearms=arms(2), curarms=arms(3),
                                  rangearms=arms(4)))
    return ''.join(out)


def main_regs():
    """MainRegs.lean: every pair of Main's is good, from Main's constraints."""
    lanes, flushes, windows, steps = main_pairs()
    s = open(os.path.join(PIL, 'Main.lean')).read()
    defs = {m.group(1): ' '.join(m.group(2).split()) for m in re.finditer(DEF, s, re.M)}
    docs = {m.group(2): m.group(1) for m in re.finditer(r"^/-- `([^`]*)` — `(.*?)` -/\ndef (c\d+) ", s, re.M)}
    cname = {}
    for m in re.finditer(r"^/-- `[^`]*` — `(.*?)` -/\ndef (c\d+) ", s, re.M):
        cname[m.group(1)] = m.group(2)
    bus_reps = {m.group(1): m.group(2) for m in re.finditer(
        r"^def (bus\d+) : BusArg F \(Ctx F\) \(Trace F\) where\n(?:  .*\n){4}  reps := fun x t i => (.*)\n", s, re.M)}
    bus_op = {m.group(1): m.group(2).rstrip(',') for m in re.finditer(
        r"^def (bus\d+) : BusArg F \(Ctx F\) \(Trace F\) where\n(?:  .*\n){6}  tuple := \[\n    fun x t i => (.*)\n", s, re.M)}

    def unf(names):
        return ', '.join(f"Main.{n}" for n in closure(names, defs))

    def at(e, row):
        e = re.sub(r"\bx t i\b", f"x t {row}", e)
        e = re.sub(r"(?<![.\w])((?:Main|Zisk)_\w+) x t", r"Main.\1 x t", e)
        return e.replace("(t i)", f"(t {row})")

    def lit_cast(n):
        return {0: 'Lean.Grind.Semiring.natCast_zero', 1: 'Lean.Grind.Semiring.natCast_one'}.get(n)

    out = [REGS_HEAD]
    names = []
    excl = {0: "(a_src_mem[{l}]+a_src_reg[{l}])*((a_src_mem[{l}]+a_src_reg[{l}])-1)",
            1: "((sel_mem_b[{l}])+b_src_reg[{l}])*(((sel_mem_b[{l}])+b_src_reg[{l}])-1)",
            2: "((store_mem[{l}]+store_ind[{l}])+store_reg[{l}])*(((store_mem[{l}]+store_ind[{l}])+store_reg[{l}])-1)"}
    regcol = {0: 'a_src_reg', 1: 'b_src_reg', 2: 'store_reg'}
    memcols = {1: ('b_src_mem', 'b_src_ind'), 2: ('store_mem', 'store_ind')}
    mstep = {0: 'a_mem_step', 1: 'b_mem_step', 2: 'store_mem_step'}
    for l, k, pre, cur, _ in lanes:
        name = f"lane_good_{l}_{k}"
        names.append(name)
        cx = cname[excl[k].format(l=l)]
        rp, rc, op = bus_reps[pre], bus_reps[cur], bus_op[cur]
        casts = []
        for v in (l, k):
            c = lit_cast(v)
            casts.append(c if c else f"← natCast_ofNat' {v}")
        cast_simp = [c for c in casts if not c.startswith('←')]
        cast_rw = list(dict.fromkeys(c for c in casts if c.startswith('←')))
        stepdefs = unf([f"Main_{mstep[k]}_{l}"])
        rc_unf = unf(re.findall(r"((?:Main|Zisk)_\w+) x t i", rc))
        rcop_unf = unf(re.findall(r"((?:Main|Zisk)_\w+) x t i", rc + ' ' + op))
        rc_simp = f"\n    simp only [{rc_unf}]" if rc_unf else ""
        rcop_simp = f"\n    simp only [{rcop_unf}] at hr h3" if rcop_unf else ""
        body = f"""theorem {name} {{k : Nat}} {{x : Main.Ctx F}} {{t : Main.Trace F}} (h : MainSegOk k x t) :
    (Pair.mk ⟨⟨Main.{pre}, x, t⟩, Main.numRows⟩ ⟨⟨Main.{cur}, x, t⟩, Main.numRows⟩
      (laneStep k {l} {k})).Good where
  same := rfl
  side_pre := rfl
  side_cur := rfl
  bits_pre := fun j _ => by
    have e := (h.rows j).bool_{regcol[k]}_{l}
    show {at(rp, 'j')} = 0 ∨ {at(rp, 'j')} = 1
    rw [Lean.Grind.Semiring.add_zero]; exact bit_or e
  bits_cur := fun j _ => by
    have e := (h.rows j).{cx}
    simp only [{unf([cx])}] at e
    show {at(rc, 'j')} = 0 ∨ {at(rc, 'j')} = 1{rc_simp}
    exact bit_or (by grind)
  shape := fun j _ => rfl
  cur_reg := fun j hj hr h3 => by
    obtain ⟨m, hm, rfl⟩ := mem_at_rows (s := ⟨⟨Main.{pre}, x, t⟩, Main.numRows⟩) rfl hj
    change {at(rc, 'm')} = 1 at hr
    change {at(op, 'm')} = 3 at h3
    have h3' := h3{rcop_simp}
    have hreg : (t m).{regcol[k]} {l} = 1 := by grind
"""
        if k in memcols:
            a, b2 = memcols[k]
            body += f"""    obtain ⟨-, hind⟩ := bits_zero (h.rows m).bool_{a}_{l} (h.rows m).bool_{b2}_{l} (by grind)
"""
        body += f"""    refine ⟨by show (t m).{regcol[k]} {l} + 0 = 1; rw [hreg]; grind, ?_⟩
    have hs := laneStep_cast (F := F) k {l} {k} m
"""
        if k == 1:
            ca = cname[f"addr1[{l}]-(b_offset_imm0[{l}]+(b_src_ind[{l}]*a[{l}][0]))"]
            body += f"""    have ha := (h.rows m).{ca}
    simp only [Main.{ca}] at ha
"""
        body += f"""    simp only [tupleOf, Main.{cur}, Main.{pre}, el, List.map_cons, List.map_nil,
      List.getD_cons_zero, List.getD_cons_succ, List.cons.injEq, and_true]
"""
        if k == 0:
            body += "    refine ⟨h3', trivial, ?_⟩\n"
        elif k == 1:
            body += "    refine ⟨h3', by rw [hind] at ha; grind, ?_, by rw [hind]; grind⟩\n"
        else:
            body += f"    refine ⟨h3', by simp only [{unf([f'Main_addr2_{l}'])}, hind]; grind, ?_, by rw [hind]; grind⟩\n"
        body += f"""    simp only [{stepdefs}, h.init, h.segStep m hm, hs{''.join(', ' + c for c in cast_simp)}]
"""
        if cast_rw:
            body += f"    all_goals rw [{', '.join(cast_rw)}]\n"
        body += "    all_goals grind\n"
        out.append(body)
        # The step the access sends, whatever its op: the lane's memory step.
        sbody = f"""theorem lane_step_{l}_{k} {{k : Nat}} {{x : Main.Ctx F}} {{t : Main.Trace F}} (h : MainSegOk k x t)
    {{m : Nat}} (hm : m < Main.numRows) :
    el 2 (tupleOf ⟨Main.{cur}, x, t⟩ m) = ((laneStep k {l} {k} m : Nat) : F) := by
  have hs := laneStep_cast (F := F) k {l} {k} m
  simp only [tupleOf, Main.{cur}, el, List.map_cons, List.map_nil, List.getD_cons_zero,
    List.getD_cons_succ]
  simp only [{stepdefs}, h.init, h.segStep m hm, hs{''.join(', ' + c for c in cast_simp)}]
"""
        if cast_rw:
            sbody += f"  all_goals rw [{', '.join(cast_rw)}]\n"
        sbody += "  all_goals grind\n"
        out.append(sbody)
    for f, r, pre, cur, _ in flushes:
        name = f"flush_good_{f}_{r}"
        names.append(name)
        last = bus_reps[cur].strip() != '1'
        bits_cur = ("fun _ _ => .inr rfl" if not last else """fun j _ => by
    have e := (h.rows j).c0
    simp only [Main.c0] at e
    show 1 - x.Main_main_last_segment = 0 ∨ 1 - x.Main_main_last_segment = 1
    rcases bit_or (s := x.Main_main_last_segment) (by grind) with h0 | h1
    · right; rw [h0]; grind
    · left; rw [h1]; grind""")
        out.append(f"""theorem {name} {{k : Nat}} {{x : Main.Ctx F}} {{t : Main.Trace F}} (h : MainSegOk k x t) :
    (Pair.mk ⟨⟨Main.{pre}, x, t⟩, Main.numRows⟩ ⟨⟨Main.{cur}, x, t⟩, Main.numRows⟩
      (fun _ => flushStep k {f})).Good where
  same := rfl
  side_pre := rfl
  side_cur := rfl
  bits_pre := fun _ _ => .inr rfl
  bits_cur := {bits_cur}
  shape := fun _ _ => rfl
  cur_reg := fun j _ _ _ => by
    refine ⟨rfl, ?_⟩
    have hs := flushStep_cast (F := F) k {f}
    simp only [tupleOf, Main.{cur}, Main.{pre}, el, List.map_cons, List.map_nil,
      List.getD_cons_zero, List.getD_cons_succ, List.cons.injEq, and_true, true_and]
    simp only [{unf([f'Main_flush_reg_mem_step_{f}'])}, h.init, hs]
    rw [← natCast_ofNat' {windows[f]}]
    grind
""")
    ranges = {(l, k): r for l, k, _, _, r in lanes}
    franges = {(f, r): rg for f, r, _, _, rg in flushes}
    bus_tuple0 = {m.group(1): m.group(2).rstrip(',') for m in re.finditer(
        r"^def (bus\d+) : BusArg F \(Ctx F\) \(Trace F\) where\n(?:  .*\n){6}  tuple := \[\n    fun x t i => (.*)\]\n", s, re.M)}
    gaps = []
    for l, k, pre, cur, rng in lanes:
        gname = f"gap_lane_{l}_{k}"
        gaps.append((gname, f".lane {l} {k}"))
        stepname = re.search(r"((?:Main|Zisk)_\w+) x t i", bus_tuple0[rng]).group(1)
        casts = []
        for v in (l, k):
            c = lit_cast(v)
            casts.append(c if c else f"← natCast_ofNat' {v}")
        cast_simp = [c for c in casts if not c.startswith('←')]
        cast_rw = list(dict.fromkeys(c for c in casts if c.startswith('←')))
        out.append(f"""theorem {gname} {{k : Nat}} {{x : Main.Ctx F}} {{t : Main.Trace F}} (h : MainSegOk k x t)
    (hR : RangeHolds Main.ranges Main.{rng} x t) : ((PairTag.lane {l} {k}).pair k x t).GapOk := by
  intro j hj hr
  obtain ⟨m, hm, rfl⟩ := mem_at_rows (s := ⟨⟨Main.{pre}, x, t⟩, Main.numRows⟩) rfl hj
  obtain ⟨n, hn, e⟩ := range_val hR rfl (j := m)
    (fun h0 => Lean.Grind.Field.zero_ne_one (h0.symm.trans hr)) (List.mem_singleton_self _)
  refine ⟨n, hn, ?_⟩
  have hs : Main.{stepname} x t m = ((laneStep k {l} {k} m : Nat) : F) := by
    rw [laneStep_cast]
    simp only [{unf([stepname])}, h.init, h.segStep m hm{''.join(', ' + c for c in cast_simp)}]
{'    all_goals rw [' + ', '.join(cast_rw) + ']' + chr(10) if cast_rw else ''}    all_goals grind
  show ((laneStep k {l} {k} m : Nat) : F) - el 2 (tupleOf ⟨Main.{pre}, x, t⟩ m) - 1 = _
  rw [← hs]
  simp only [tupleOf, el, Main.{pre}, List.map_cons, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero]
  exact e
""")
    for f, r, pre, cur, rng in flushes:
        gname = f"gap_flush_{f}_{r}"
        gaps.append((gname, f".flush {f} {r}"))
        out.append(f"""theorem {gname} {{k : Nat}} {{x : Main.Ctx F}} {{t : Main.Trace F}} (h : MainSegOk k x t)
    (hR : RangeHolds Main.ranges Main.{rng} x t) : ((PairTag.flush {f} {r}).pair k x t).GapOk := by
  intro j hj _
  obtain ⟨n, hn, e⟩ := range_val hR rfl (j := j) (fun h0 => Lean.Grind.Field.zero_ne_one h0.symm)
    (List.mem_singleton_self _)
  refine ⟨n, hn, ?_⟩
  have hs : Main.Main_flush_reg_mem_step_{f} x t j = ((flushStep k {f} : Nat) : F) := by
    rw [flushStep_cast]
    simp only [{unf([f'Main_flush_reg_mem_step_{f}'])}, h.init]
    rw [← natCast_ofNat' {windows[f]}]
    all_goals grind
  show ((flushStep k {f} : Nat) : F) - el 2 (tupleOf ⟨Main.{pre}, x, t⟩ j) - 1 = _
  rw [← hs]
  simp only [tupleOf, el, Main.{pre}, List.map_cons, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero]
  exact e
""")
    gterms = ',\n    '.join(f"{g} h (hR ({tag}))" for g, tag in gaps)
    out.append(f"""set_option maxRecDepth 100000 in
/-- **Every pair's gap is range-checked**, given each pair's range check sound. -/
theorem mainTags_gap {{k : Nat}} {{x : Main.Ctx F}} {{t : Main.Trace F}} (h : MainSegOk k x t)
    (hR : ∀ g : PairTag, RangeHolds Main.ranges g.rangeArg x t) :
    AllOf (fun g : PairTag => (g.pair k x t).GapOk) mainTags :=
  ⟨{gterms},
    trivial⟩
""")
    aterms = []
    for l, k, *_ in lanes:
        aterms.append("fun _ _ e => PairTag.noConfusion e")
    for f, r, *_ in flushes:
        n = r + 1
        lem = "Lean.Grind.Semiring.natCast_one.symm" if n == 1 else f"natCast_ofNat' {n}"
        aterms.append(f"fun _ _ e => by cases e; exact fun _ _ _ => {lem}")
    oterms = ',\n    '.join(["rfl"] * (len(lanes) + len(flushes)))
    out.append(f"""set_option maxRecDepth 100000 in
/-- **A lane pair is sent at every row, a flush once.** -/
theorem mainTags_once : AllOf (fun g : PairTag => g.OnceOk (F := F)) mainTags :=
  ⟨{oterms},
    trivial⟩

""")
    rterms = []
    reads = [(f"lane_{l}_{k}", p, c) for l, k, p, c, _ in lanes if k != 2] + \
            [(f"flush_{f}_{r}", p, c) for f, r, p, c, _ in flushes]
    for nm, p, c in reads:
        out.append(f"""omit [Lean.Grind.IsCharP F goldilocks] in
theorem read_{nm} (x : Main.Ctx F) (t : Main.Trace F) (j : Int) :
    el 4 (tupleOf ⟨Main.{c}, x, t⟩ j) = el 4 (tupleOf ⟨Main.{p}, x, t⟩ j) ∧
      el 5 (tupleOf ⟨Main.{c}, x, t⟩ j) = el 5 (tupleOf ⟨Main.{p}, x, t⟩ j) := ⟨rfl, rfl⟩
""")
    for l, k, *_ in lanes:
        rterms.append("fun h => absurd h (by decide)" if k == 2 else f"fun _ => read_lane_{l}_{k}")
    for f, r, *_ in flushes:
        rterms.append(f"fun _ => read_flush_{f}_{r}")
    rterms = ',\n    '.join(rterms)
    out.append(f"""set_option maxRecDepth 100000 in
/-- **Reads write back what they prove.** -/
theorem mainTags_read : AllOf (fun g : PairTag => g.ReadOk (F := F)) mainTags :=
  ⟨{rterms},
    trivial⟩

""")
    cterms = []
    for l, k, *_ in lanes:
        cterms.append(f"fun _ _ _ h _ hm => lane_step_{l}_{k} h hm")
    for f, r, p, c, _ in flushes:
        out.append(f"""omit [Lean.Grind.IsCharP F goldilocks] in
theorem cur_flush_{f}_{r} (x : Main.Ctx F) (t : Main.Trace F) (j : Int) :
    el 0 (tupleOf ⟨Main.{c}, x, t⟩ j) = 3 := rfl
""")
        cterms.append(f"cur_flush_{f}_{r}")
    out.append(f"""set_option maxRecDepth 100000 in
/-- **A lane access sends its memory step; a flush, op 3.** -/
theorem mainTags_cur : AllOf (fun g : PairTag => g.CurOk (F := F)) mainTags :=
  ⟨{(','+chr(10)+'    ').join(cterms)},
    trivial⟩

""")
    out.append(f"""set_option maxRecDepth 100000 in
/-- **A flush's address is its register's.** -/
theorem mainTags_addr : AllOf (fun g : PairTag => g.AddrOk (F := F)) mainTags :=
  ⟨{(','+chr(10)+'    ').join(aterms)},
    trivial⟩

""")
    terms = ',\n    '.join(f"{n} h" for n in names)
    out.append(f"""set_option maxRecDepth 100000 in
theorem mainTags_good {{k : Nat}} {{x : Main.Ctx F}} {{t : Main.Trace F}} (h : MainSegOk k x t) :
    AllOf (fun g : PairTag => (g.pair k x t).Good) mainTags :=
  ⟨{terms},
    trivial⟩

/-- **Every pair of Main's is good**, in a segment whose hypotheses hold. -/
theorem mainPairs_good {{k : Nat}} {{x : Main.Ctx F}} {{t : Main.Trace F}} (h : MainSegOk k x t) :
    ∀ P ∈ mainPairs k x t, P.Good := by
  intro P hP
  simp only [mainPairs, List.mem_map] at hP
  obtain ⟨g, hg, rfl⟩ := hP
  exact (mainTags_good h).mem g hg

end Correspondence
""")
    open(OUT_REGS, 'w').write('\n'.join(out))
    return len(names)


SYSTEM_HEAD = """/-! ### Every other AIR on the memory bus

The instances of the 39 AIRs, their senders on bus 10, and what follows for a
sound bus: a register tuple balances among Main's senders and the global
updates alone (`reg_balance`). -/

section System

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- A tuple's first element is the op its argument sends. -/
theorem headD_tupleOf (p : Prover F) (j : Int) : (tupleOf p j).headD 0 = opOf p.arg p.x p.t j := by
  unfold tupleOf opOf
  cases p.arg.tuple <;> rfl

theorem never_append {ss ss' : List (Sender F)} {k : F}
    (h : ∀ s ∈ ss, ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k)
    (h' : ∀ s ∈ ss', ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k) :
    ∀ s ∈ ss ++ ss', ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k := by
  intro s hs
  rcases List.mem_append.mp hs with hs | hs
  · exact h s hs
  · exact h' s hs

theorem never_flatMap {α : Type} {l : List α} {f : α → List (Sender F)} {k : F}
    (h : ∀ a ∈ l, ∀ s ∈ f a, ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k) :
    ∀ s ∈ l.flatMap f, ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k := by
  intro s hs
  obtain ⟨a, ha, hs⟩ := List.mem_flatMap.mp hs
  exact h a ha s hs

"""

SYSTEM_TAIL = """/-- Every instance of every AIR but Main on the memory bus. -/
structure MemOthers (F : Type) [Lean.Grind.Field F] where
{fields}

/-- Their senders on the memory bus. -/
def MemOthers.senders (S : MemOthers F) : List (Sender F) :=
{senders}

/-- **None of them sends a register tuple.** -/
theorem MemOthers.never (S : MemOthers F) :
    ∀ s ∈ S.senders, ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ ((3 : Int) : F) :=
{never}

/-- **A register tuple balances among Main's senders.** On a sound memory bus
over Main's senders `ms` and every other AIR's, every tuple with op 3 — a
register's — is proved as often as it is assumed by Main's senders and the
global updates alone. -/
theorem reg_balance (S : MemOthers F) {{ms : List (Sender F)}} {{gp ga : List (List F)}}
    (h : SumHolds 10 (ms ++ S.senders) gp ga) {{τ : List F}} (h3 : τ.headD 0 = ((3 : Int) : F)) :
    provedCount ms τ + times gp τ = assumedCount ms τ + times ga τ :=
  h.restrict S.never h3

/-- The memory step of lane `l`'s access in slot `κ` (0 for `a`, 1 for `b`, 2
for the store) at row `j` of segment `k`: `main_step_to_mem_step`, with the
segment's first step `k·{steps}` and the row's step `4·j + l`. -/
def laneStep (k l κ : Nat) (j : Int) : Nat := 1 + 4 * (k * {steps} + 4 * j.toNat + l) + κ

/-- The memory step of flush window `f` of segment `k`: the special step after
the window's last main step. -/
def flushStep (k f : Nat) : Nat := 1 + 4 * (k * {steps} + (f + 1) * {window} - 1) + 3

end System

/-- Which register-access pair of Main's: lane `l`'s access in slot `κ` (0 for
`a`, 1 for `b`, 2 for the store), or flush window `f`'s of register `ρ + 1`. -/
inductive PairTag where
  | lane (l κ : Nat)
  | flush (f ρ : Nat)
  deriving DecidableEq

/-- Whether the pair's access only reads: every lane access but a store, and
the flushes. -/
def PairTag.isRead : PairTag → Bool
  | .lane _ κ => κ != 2
  | .flush _ _ => true

/-- The pair's memory step at row `j` of segment `k`. -/
def PairTag.step : PairTag → Nat → Int → Nat
  | .lane l κ, k, j => laneStep k l κ j
  | .flush f _, k, _ => flushStep k f

/-- Main's {npairs} register-access pairs: per lane, `a`, `b` and the store
({nlane}); per flush window and register ({nflush}: {nwin} windows). -/
def mainTags : List PairTag := [{taglist}]

set_option maxRecDepth 100000 in
theorem mainTags_nodup : mainTags.Nodup := by decide

section System

variable {{F : Type}} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The argument proving the access before, per pair. -/
def PairTag.preArg : PairTag → BusArg F (Main.Ctx F) (Main.Trace F)
{prearms}

/-- The argument assuming the access, per pair. -/
def PairTag.curArg : PairTag → BusArg F (Main.Ctx F) (Main.Trace F)
{curarms}

/-- The range check of the pair's gap, `step - prev - 1`. -/
def PairTag.rangeArg : PairTag → BusArg F (Main.Ctx F) (Main.Trace F)
{rangearms}

/-- Pair `g` of segment `k`, instance `x`, `t`. -/
def PairTag.pair (g : PairTag) (k : Nat) (x : Main.Ctx F) (t : Main.Trace F) : Pair F :=
  ⟨⟨⟨g.preArg, x, t⟩, Main.numRows⟩, ⟨⟨g.curArg, x, t⟩, Main.numRows⟩, g.step k⟩

/-- Main's register-access pairs in segment `k`. -/
def mainPairs (k : Nat) (x : Main.Ctx F) (t : Main.Trace F) : List (Pair F) :=
  mainTags.map fun g => g.pair k x t

/-- Main's {nmain} arguments on the memory bus, for one instance: its pairs'. -/
def mainMemSenders (x : Main.Ctx F) (t : Main.Trace F) : List (Sender F) :=
  pairSenders (mainPairs 0 x t)

theorem pairSenders_mainPairs (k : Nat) (x : Main.Ctx F) (t : Main.Trace F) :
    pairSenders (mainPairs k x t) = mainMemSenders x t := by
  simp only [mainMemSenders, pairSenders, mainPairs, List.flatMap_map, PairTag.pair]

/-- The register `r`'s initial tuple, `(3, r, 0, 8, 0, 0)`: what the global
update `global_init_mem` assumes for each register in Main. -/
def regInit (r : Nat) : List F := [3, ((r : Nat) : F), 0, 8, 0, 0]

/-- The registers' initial tuples, `x1` to `x39`. -/
def regInits : List (List F) := (List.range 39).map fun r => regInit (r + 1)

/-- The global updates on the memory bus, as far as register tuples go: none
proves one, and the ones that assume one are the registers' initial tuples.
The global updates are not extracted, so this is written here, from
`main.pil` (`global_init_mem`). -/
def GlobalRegs (gp ga : List (List F)) : Prop :=
  ∀ τ : List F, τ.headD 0 = ((3 : Int) : F) → times gp τ = 0 ∧ times ga τ = times regInits τ

/-- **The register chain balances within Main.** On a sound memory bus over
every Main instance (`segs`) and every other AIR, with the global updates
`GlobalRegs` describes, every register tuple is proved by Main's arguments as
often as they and the registers' initial values assume it. -/
theorem reg_chain_balance (S : MemOthers F) (segs : List (Main.Ctx F × Main.Trace F))
    {{gp ga : List (List F)}} (hg : GlobalRegs gp ga)
    (h : SumHolds 10 (segs.flatMap (fun m => mainMemSenders m.1 m.2) ++ S.senders) gp ga)
    {{τ : List F}} (h3 : τ.headD 0 = ((3 : Int) : F)) :
    provedCount (segs.flatMap (fun m => mainMemSenders m.1 m.2)) τ =
      assumedCount (segs.flatMap (fun m => mainMemSenders m.1 m.2)) τ + times regInits τ := by
  have e := reg_balance S h h3
  obtain ⟨g1, g2⟩ := hg τ h3
  rw [g1, g2, Lean.Grind.Semiring.add_zero] at e
  exact e

end System
"""


REGS_HEAD = """/-
  Every pair of Main's is good.

  `pairs_chain` (`RegChain.lean`) turns good pairs into a register chain. This
  file proves Main's pairs good (`mainPairs_good`), from Main's constraints and
  two facts about the segment (`MainSegOk`): its first step is `k·2^24`, as
  the continuation chain makes it, and the fixed column `SEGMENT_STEP` is the
  row, as the setup fixes it.

  * A lane's register access in slot `κ` (`a`, `b`, the store): its count is a
    bit, the sum of its memory and register selectors is a bit, and when it
    assumes a register tuple, the register selector is 1 and the memory
    selectors 0, so its address is the register's (for `b`, through `addr1`,
    `main.pil:224`) and its memory step is `laneStep k l κ`.
  * A flush, window `f` and register `r`: sent once, its count 1 — or, for the
    last window, `1 - main_last_segment`, a bit — and its step `flushStep k f`.

  Generated by `sail/gen_membus.py` (`make membus`).
-/
import Proofs.RegChain

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/- The generated theorems share one signature; not all need the characteristic. -/
set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- Every element of a list satisfies `p`, as a nested conjunction: a proof of it
for a literal list is a tuple, one component per element. -/
def AllOf {α : Type _} (p : α → Prop) : List α → Prop
  | [] => True
  | a :: l => p a ∧ AllOf p l

theorem AllOf.mem {α : Type _} {p : α → Prop} : ∀ {l : List α}, AllOf p l → ∀ a ∈ l, p a
  | [], _, _, ha => nomatch ha
  | b :: l, ⟨hb, hl⟩, a, ha => by
    rcases List.mem_cons.mp ha with rfl | ha
    · exact hb
    · exact AllOf.mem hl a ha

/-- What a segment's register pairs need besides Main's constraints: the
segment's first step, `k·2^24` (the continuation chain's), and the row's step
in the fixed column `SEGMENT_STEP` (the setup's). -/
structure MainSegOk (k : Nat) (x : Main.Ctx F) (t : Main.Trace F) : Prop where
  rows : ∀ i, Main.everyRow x t i
  init : x.Main_segment_initial_step = ((k * 16777216 : Nat) : F)
  segStep : ∀ m : Nat, m < Main.numRows → (t m).Main_SEGMENT_STEP = ((m : Nat) : F)

omit [Lean.Grind.IsCharP F goldilocks] in
theorem bit_or {s : F} (h : s * (1 - s) = 0) : s = 0 ∨ s = 1 := by
  obtain ⟨b, e⟩ := bool_of_idem h
  cases b <;> simp [e, bit]

/-- Two bits that sum to 0 are both 0. -/
theorem bits_zero {u v : F} (hu : u * (1 - u) = 0) (hv : v * (1 - v) = 0) (h : u + v = 0) :
    u = 0 ∧ v = 0 := by
  rcases bit_or hu with rfl | rfl <;> rcases bit_or hv with rfl | rfl
  · exact ⟨rfl, rfl⟩
  · exfalso; have : (1 : F) = 0 := by grind
    exact Lean.Grind.Field.zero_ne_one this.symm
  · exfalso; have : (1 : F) = 0 := by grind
    exact Lean.Grind.Field.zero_ne_one this.symm
  · exfalso
    have : ((2 : Nat) : F) = ((0 : Nat) : F) := by
      rw [natCast_two, Lean.Grind.Semiring.natCast_zero]; grind
    have := natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega) this
    omega

omit [Lean.Grind.IsCharP F goldilocks] in
theorem mem_at_rows {s : Sender F} (hs : s.p.arg.once = false) {j : Int}
    (hj : j ∈ s.at) : ∃ m : Nat, m < s.rows ∧ j = (m : Int) := by
  simp only [Sender.at, hs, Bool.false_eq_true, if_false, List.mem_map, List.mem_range] at hj
  obtain ⟨m, hm, rfl⟩ := hj
  exact ⟨m, hm, rfl⟩

omit [Lean.Grind.IsCharP F goldilocks] in
theorem laneStep_cast (k l κ m : Nat) :
    ((laneStep k l κ (m : Int) : Nat) : F) =
      1 + 4 * (((k * 16777216 : Nat) : F) + ((m : Nat) : F) * 4 + ((l : Nat) : F)) + ((κ : Nat) : F) := by
  simp only [laneStep, Int.toNat_natCast, Lean.Grind.Semiring.natCast_add,
    Lean.Grind.Semiring.natCast_mul, Lean.Grind.Semiring.natCast_one]
  rw [← natCast_ofNat' 4, ← natCast_ofNat' 16777216]
  grind

omit [Lean.Grind.IsCharP F goldilocks] in
theorem flushStep_cast (k f : Nat) :
    ((flushStep k f : Nat) : F) =
      1 + 4 * (((k * 16777216 : Nat) : F) + (((f + 1) * 4194304 : Nat) : F) - 1) + 4 - 1 := by
  have e : flushStep k f = 4 * (k * 16777216) + 4 * ((f + 1) * 4194304) := by
    unfold flushStep; have : 1 ≤ (f + 1) * 4194304 := by omega
    omega
  rw [e]
  simp only [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul,
    Lean.Grind.Semiring.natCast_one]
  rw [← natCast_ofNat' 4, ← natCast_ofNat' 16777216, ← natCast_ofNat' 4194304]
  grind

/-- At every row a pair proves at, its gap `step - prev - 1` is a range-checked
24-bit number. -/
def Pair.GapOk (P : Pair F) : Prop :=
  ∀ j ∈ P.pre.at, P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 1 →
    ∃ m : Nat, m < 16777216 ∧ ((P.step j : Nat) : F) - el 2 (tupleOf P.pre.p j) - 1 = ((m : Nat) : F)

set_option maxRecDepth 100000 in
omit [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks] in
theorem main_range_max : ∀ d ∈ (Main.ranges : List RangeDef), d.opid = 102 → d.max = 16777215 := by
  decide

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A value a sound range check on bus 102 sends is a 24-bit number. -/
theorem range_val {x : Main.Ctx F} {t : Main.Trace F} {R : BusArg F (Main.Ctx F) (Main.Trace F)}
    (hR : RangeHolds Main.ranges R x t) (hop : R.opids = [102]) {j : Int} (hr : R.reps x t j ≠ 0)
    {f : Main.Ctx F → Main.Trace F → Int → F} (hf : f ∈ R.tuple) :
    ∃ n : Nat, n < 16777216 ∧ f x t j = ((n : Nat) : F) := by
  obtain ⟨-, -, b, d, hb, hd, hdo, -, -, hall⟩ := hR
  rw [hop] at hb
  have hb' : b = 102 := by simpa using hb.symm
  have hmax := main_range_max d hd (by rw [hdo, hb'])
  obtain ⟨n, -, hn, e⟩ := hall j hr f hf
  exact ⟨n, by omega, e⟩

/-- A lane pair is sent at every row of its instance, a flush once. -/
def PairTag.OnceOk : PairTag → Prop
  | .lane l κ => ((PairTag.lane l κ).preArg (F := F)).once = false
  | .flush f ρ => ((PairTag.flush f ρ).preArg (F := F)).once = true

/-- A read writes back what it proves: the assumed tuple's value is the proved
one's, for a lane's `a` and `b` accesses and a flush (not a store). -/
def PairTag.ReadOk (g : PairTag) : Prop :=
  g.isRead = true → ∀ (x : Main.Ctx F) (t : Main.Trace F) (j : Int),
    el 4 (tupleOf ⟨g.curArg, x, t⟩ j) = el 4 (tupleOf ⟨g.preArg, x, t⟩ j) ∧
      el 5 (tupleOf ⟨g.curArg, x, t⟩ j) = el 5 (tupleOf ⟨g.preArg, x, t⟩ j)

/-- What a pair's assuming side sends: a lane access, its lane's memory step,
whatever its op; a flush, op 3. -/
def PairTag.CurOk : PairTag → Prop
  | .lane l κ => ∀ (k : Nat) (x : Main.Ctx F) (t : Main.Trace F), MainSegOk k x t →
      ∀ m : Nat, m < Main.numRows →
        el 2 (tupleOf ⟨(PairTag.lane l κ).curArg, x, t⟩ m) = ((laneStep k l κ m : Nat) : F)
  | .flush f ρ => ∀ (x : Main.Ctx F) (t : Main.Trace F) (j : Int),
      el 0 (tupleOf ⟨(PairTag.flush f ρ).curArg, x, t⟩ j) = 3

/-- A flush pair's address is its register's. -/
def PairTag.AddrOk (g : PairTag) : Prop :=
  ∀ f ρ, g = .flush f ρ → ∀ (x : Main.Ctx F) (t : Main.Trace F) (j : Int),
    el 1 (tupleOf ⟨g.preArg, x, t⟩ j) = ((ρ + 1 : Nat) : F)

"""

HEAD = """/-
  No AIR but Main sends a register tuple.

  Main keeps the registers as a chain of tuples `(3, reg, step, 8, value)` on
  the memory bus (10); the chain argument (`MemCheck.lean`) needs the tuples
  with op 3 to be Main's alone. This file proves it for every other argument on
  the bus — {n} arguments in {nairs} AIRs ({kinds}): at a row where it sends,
  its op, the tuple's first element, is not 3.

  * `bits`: the op is a constant plus selectors times coefficients, each
    selector a bit by the AIR's booleanity constraint; `op_ne` enumerates the
    values it can take.
  * `sel`: some of those values are 3, but only if two selectors are set at
    once, and the AIR constrains their sum (`<Air>_sel`) to a bit; `op_ne_sel`
    enumerates the values with that sum a bit.
  * `fixed`: the op is a sum of the precompile's clock columns plus 1, fixed
    data; that it is 1 or 2 at every row is `FixedOp`, which `check_tables.py`
    checks against the setup's fixed files.
  * `romdata`: the Rom's data rows send their `flags`, 2 (RAM) or 4 (ROM
    initialization), the program's data: `RomDataFlags`.

  Then the system: an instance structure per AIR (its trace, constraints and
  the data its ops rest on), `MemOthers` listing any number of instances of
  each, and Main's 336 arguments on the bus (`mainMemSenders`). On a sound
  memory bus (`SumHolds`, `SumBus.lean`) over all of them, with the global
  updates as `GlobalRegs` writes them, every register tuple balances among
  Main's arguments and the registers' initial tuples alone
  (`reg_chain_balance`).

  Generated by `sail/gen_membus.py` (`make membus`).
-/
import Proofs.OpBusCmp
import Proofs.Adders
import Proofs.SumBus
{imports}

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/- The theorems are generated with one signature; not all use every hypothesis. -/
set_option linter.unusedVariables false
set_option linter.unusedSectionVars false

section Lemma

variable {{F : Type}} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- **An op that is `k` only if a bit is not a bit.** As `op_ne2`, with the
second form a sum of the same selectors that the AIR constrains to a bit: no
combination with that sum 0 or 1 has the op `k`. -/
theorem op_ne_sel (c₀ d₀ : Int) (cds : List (Int × Int)) (f₀ e₀ : F) (fs es ss : List F) (k : Int)
    (op sel : F) (hc₀ : ((c₀ : Int) : F) = f₀) (hd₀ : ((d₀ : Int) : F) = e₀)
    (hcf : cds.map (fun p => (p.1 : F)) = fs) (hdf : cds.map (fun p => (p.2 : F)) = es)
    (hl : cds.length = ss.length) (hb : ∀ s ∈ ss, s * (1 - s) = 0)
    (heq : op = f₀ + linF fs ss) (hseq : sel = e₀ + linF es ss) (hs : sel * (1 - sel) = 0)
    (hvals : ∀ zw ∈ subsetSums2 cds, ((d₀ + zw.2 = 0 ∨ d₀ + zw.2 = 1) → c₀ + zw.1 ≠ k) ∧
      -2 ^ 62 < c₀ + zw.1 ∧ c₀ + zw.1 < 2 ^ 62 ∧ -2 ^ 62 < d₀ + zw.2 ∧ d₀ + zw.2 < 2 ^ 62)
    (hk : -2 ^ 62 < k ∧ k < 2 ^ 62) :
    op ≠ (k : F) := by
  obtain ⟨zw, hz, e1, e2⟩ := linF_bits2 cds ss hl hb
  obtain ⟨himp, l1, h1, l2, h2⟩ := hvals zw hz
  have hsel : ((d₀ + zw.2 : Int) : F) = sel := by
    rw [hseq, ← hdf, e2, ← hd₀, ← Lean.Grind.Ring.intCast_add]
  obtain ⟨b, hb'⟩ := bool_of_idem hs
  have h01 : d₀ + zw.2 = 0 ∨ d₀ + zw.2 = 1 := by
    rw [← hsel] at hb'
    cases b
    · have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks) (x := d₀ + zw.2) (y := 0)).mp
        (by rw [hb', Lean.Grind.Ring.intCast_zero]; simp [bit])
      unfold goldilocks at this; omega
    · have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks) (x := d₀ + zw.2) (y := 1)).mp
        (by rw [hb', Lean.Grind.Ring.intCast_one]; simp [bit])
      unfold goldilocks at this; omega
  intro h
  rw [heq, ← hcf, e1, ← hc₀, ← Lean.Grind.Ring.intCast_add] at h
  have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp h
  unfold goldilocks at this
  exact himp h01 (by omega)

/-- The fixed-data fact about a precompile's op: 1 or 2 at the row, a sum of
its clock columns plus 1. `check_tables.py` checks it at every row of the
setup's fixed file. -/
def FixedOp {{C T : Type}} (a : BusArg F C T) (x : C) (t : T) (i : Int) : Prop :=
  opOf a x t i = 1 ∨ opOf a x t i = 2

theorem fixedOp_ne {{C T : Type}} {{a : BusArg F C T}} {{x : C}} {{t : T}} {{i : Int}}
    (h : FixedOp a x t i) : opOf a x t i ≠ ((3 : Int) : F) := by
  intro h3
  rw [intCast_lit] at h3
  rcases h with h | h <;> rw [h3] at h
  · have := natCast_inj (F := F) (x := 3) (y := 1) (by unfold goldilocks; omega)
      (by unfold goldilocks; omega) (by rw [Lean.Grind.Semiring.natCast_one, ← natCast_ofNat' 3]; exact h)
    omega
  · have := natCast_inj (F := F) (x := 3) (y := 2) (by unfold goldilocks; omega)
      (by unfold goldilocks; omega) (by rw [← natCast_ofNat' 3, ← natCast_ofNat' 2]; exact h)
    omega

/-- The program's data rows initialize memory with `flags` 2 (RAM) or 4 (ROM):
the program's data, as `custom_rom.rs` writes it. -/
def RomDataFlags (t : Rom.Trace F) : Prop :=
  ∀ j, (t j).is_data ≠ 0 → (t j).flags = 2 ∨ (t j).flags = 4

theorem flags_ne_three {{f : F}} (h : f = 2 ∨ f = 4) : f ≠ ((3 : Int) : F) := by
  intro h3
  rw [intCast_lit] at h3
  rcases h with h | h <;> rw [h3] at h
  · have := natCast_inj (F := F) (x := 3) (y := 2) (by unfold goldilocks; omega)
      (by unfold goldilocks; omega) (by rw [← natCast_ofNat' 3, ← natCast_ofNat' 2]; exact h)
    omega
  · have := natCast_inj (F := F) (x := 3) (y := 4) (by unfold goldilocks; omega)
      (by unfold goldilocks; omega) (by rw [← natCast_ofNat' 3, ← natCast_ofNat' 4]; exact h)
    omega

end Lemma

section Provers

variable {{F : Type}} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

"""

if __name__ == '__main__':
    main()
