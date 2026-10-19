#!/usr/bin/env python3
"""Generate lean/Proofs/MemBus.lean: no AIR but Main sends a register tuple.

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
    theorems, census, airs = [], [], []
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
                             r"  bus := .*\n  piop := \.(\w+)\n  side := \.(\w+)\n  reps := (.*)\n  tuple := \[\n"
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
        '\n'.join(theorems) + "\nend Provers\n\nend Correspondence\n"
    open(OUT, 'w').write(text)
    print(f"wrote {os.path.relpath(OUT, HERE)}: {len(census)} arguments in {len(airs)} AIRs "
          f"({', '.join(f'{v} {k}' for k, v in sorted(kinds.items()))})")


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

  Generated by `sail/gen_membus.py` (`make membus`).
-/
import Proofs.OpBusCmp
import Proofs.Adders
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
