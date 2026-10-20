#!/usr/bin/env python3
"""Generate lean/Proofs/Provers.lean: every prover on the operation bus, collected.

`run_pilot` (lean/Proofs/Run.lean) assumes that every prover on the operation
bus (5000) computes the four modeled external ops. The prover files prove it
one argument at a time; this script writes the file that collects them:

1. **The census**, from the generated AIRs (build/pil/Pil/*.lean): every bus
   argument, in every AIR but Main and the FROPS table, whose opids include
   5000 and whose side is `proves`.
2. **The lemmas**, from the prover files: for each argument, a theorem
   `<Air>_<bus>_adds` or `_not_add` for `add`, and `_cmp`, `_not_cmp` or
   `_not_ltu` for `ltu`, `eq` and `and`. The script stops if one is missing.
3. **The arguments** each lemma takes, read from its signature: the AIR's
   constraints, its range checks and lookups (which become fields of the AIR's
   instance structure), and the tables' contents (fields of `Tables`).

Usage: gen_provers.py   (from sail/; `make provers`)
"""

import glob
import os
import re
import sys
import textwrap

HERE = os.path.dirname(os.path.abspath(__file__))
PIL = os.path.join(HERE, 'build', 'pil', 'Pil')
PROOFS = os.path.join(HERE, 'lean', 'Proofs')
OUT = os.path.join(PROOFS, 'Provers.lean')

LEMMA_FILES = [
    ('OpBus.lean', r'[A-Z]\w*_bus\d+_not_add'),
    ('OpBusCmp.lean', r'[A-Z]\w*_bus\d+_not_(?:cmp|ltu)'),
    ('Adders.lean', r'[A-Z]\w*_bus\d+_adds'),
    ('Binary.lean', r'[A-Z]\w*_bus\d+_adds'),
    ('BinaryOps.lean', r'[A-Z]\w*_bus\d+_cmp'),
    ('OpBusPub.lean', r'[A-Z]\w*_bus\d+_not_pub'),
    ('BinaryPub.lean', r'[A-Z]\w*_bus\d+_pub'),
]


def signatures():
    """Theorem name -> its binders, as written, from the prover files."""
    out = {}
    for fname, pat in LEMMA_FILES:
        s = open(os.path.join(PROOFS, fname)).read()
        for m in re.finditer(r"^theorem (" + pat + r") (.*?) :=", s, re.M | re.S):
            name, rest = m.group(1), m.group(2)
            binders, depth, cur = [], 0, ''
            for ch in rest:
                if ch in '({[⦃':
                    if depth == 0:
                        cur = ''
                    depth += 1
                if depth > 0:
                    cur += ch
                if ch in ')}]⦄':
                    depth -= 1
                    if depth == 0:
                        binders.append(cur)
                if depth == 0 and ch == ':':
                    break
            out[name] = binders
    return out


def census():
    """AIR -> its arguments proving on the operation bus, in file order."""
    out = {}
    for f in sorted(glob.glob(os.path.join(PIL, '*.lean'))):
        air = os.path.basename(f)[:-5]
        if air in ('Main', 'VirtualTableZisk0', 'Prelude'):
            continue
        s = open(f).read()
        for m in re.finditer(r"^def (bus\d+) : BusArg F \(Ctx F\) \(Trace F\) where\n"
                             r"  opids := \[([^\]]*)\]\n(?:.*\n){2}  side := \.(\w+)", s, re.M):
            if '5000' in m.group(2).split(', ') and m.group(3) == 'proves':
                out.setdefault(air, []).append(m.group(1))
    return out


TABLE = {'bus16': 'arith', 'bus17': 'ext17', 'bus18': 'ext18'}


def arg_of(ty, fields):
    """The argument a lemma's explicit binder of type `ty` gets."""
    ty = ' '.join(ty.split())
    if re.match(r"∀ j, \w+\.everyRow x t j$", ty):
        return 'I.rows'
    if re.match(r"\w+\.everyRow x t i$", ty):
        return '(I.rows j)'
    if re.match(r"\w+\.bus\d+\.reps x t i ≠ 0$", ty):
        return 'hr'
    m = re.match(r"RangeHolds \w+\.ranges \w+\.(bus\d+) x t$", ty)
    if m:
        fields['rg_' + m.group(1)] = ty
        return 'I.rg_' + m.group(1)
    m = re.match(r"LookupHolds \w+\.(bus\d+) x t", ty)
    if m:
        fields['lk_' + m.group(1)] = ty
        return 'I.lk_' + m.group(1)
    if ty == '∀ p ∈ binaryTable xv tv, BinaryTableAdds p':
        return 'hT.binAdds'
    if ty == '∀ p ∈ binaryTable xv tv, BinaryTableOps p':
        return 'hT.binOps'
    if ty == '∀ p ∈ binaryTable xv tv, BinaryTableNoPub p':
        return 'hT.binNoPub'
    m = re.match(r"∀ k ∈ \(\[160\] : List Int\), TableNever ⟨VirtualTableZisk1\.(bus\d+), xv, tv⟩"
                 r" \d+ \(\(k : Int\) : F\)$", ty)
    if m:
        return f"hT.{TABLE[m.group(1)]}P"
    m = re.match(r"TableNever ⟨VirtualTableZisk1\.(bus\d+), xv, tv⟩ \d+ \(\(10 : Int\) : F\)$", ty)
    if m:
        return f"(hT.{TABLE[m.group(1)]} 10 (by simp))"
    m = re.match(r"∀ k ∈ \(\[6, 9, 14\] : List Int\), TableNever ⟨VirtualTableZisk1\.(bus\d+), xv, tv⟩"
                 r" \d+ \(\(k : Int\) : F\)$", ty)
    if m:
        return f"(fun k hk => hT.{TABLE[m.group(1)]} k (List.mem_cons_of_mem _ hk))"
    sys.exit(f"gen_provers: a binder of unknown type: {ty}")


def call(sigs, name, fields):
    args = [arg_of(b[1:-1].split(' : ', 1)[1], fields) for b in sigs[name] if not b.startswith('{')]
    return ' '.join([name] + args)


def not_fun(body):
    """`fun j hr => body`, or `fun j _` when the count is not needed."""
    name = 'hr' if re.search(r'\bhr\b', body) else '_'
    return f"fun j {name} => {body}"


def air_parts(sigs, air, buses):
    """An AIR's instance structure, its provers, and its `ok` theorem."""
    fields, oks = {}, []
    for bus in buses:
        def lemma(kind):
            n = f"{air}_{bus}_{kind}"
            return n if n in sigs else None
        adds, nadd = lemma('adds'), lemma('not_add')
        cmp_, ncmp, nltu = lemma('cmp'), lemma('not_cmp'), lemma('not_ltu')
        if adds:
            add = f"opProves_of_addsAt ({call(sigs, adds, fields)})"
        elif nadd:
            add = f"opProves_of_not ({not_fun(call(sigs, nadd, fields))})"
        else:
            sys.exit(f"gen_provers: {air}.{bus} has no lemma for add")
        parts = {}
        for code, key, idx in ((6, 'ltu', '1'), (9, 'eq', '2.1'), (14, 'and', '2.2')):
            if cmp_:
                parts[key] = f"fun j _ _ => ({call(sigs, cmp_, fields)} j).{idx}"
            elif key == 'ltu' and nltu:
                parts[key] = f"opProves_of_not ({not_fun(call(sigs, nltu, fields))})"
            elif ncmp and not (key == 'ltu' and nltu is None and air == 'Dma'):
                parts[key] = f"opProves_of_not ({not_fun(call(sigs, ncmp, fields) + f' {code} (by simp)')})"
            else:
                sys.exit(f"gen_provers: {air}.{bus} has no lemma for {key}")
        pub_, npub = lemma('pub'), lemma('not_pub')
        if pub_:
            parts['pub'] = f"fun j _ _ => ({call(sigs, pub_, fields)} j)"
        elif npub:
            parts['pub'] = f"opProves_of_not ({not_fun(call(sigs, npub, fields) + ' 160 (by simp)')})"
        else:
            sys.exit(f"gen_provers: {air}.{bus} has no lemma for pubout")
        oks.append((add, parts))
    inst = f"{air}Inst"
    extra = ''.join(f"\n  {f} : {ty}" for f, ty in fields.items())
    struct = f"""/-- An instance of `{air}`: its trace, the AIR's constraints at every row,
and the soundness of the buses it assumes. -/
structure {inst} (xv : VirtualTableZisk1.Ctx F) (tv : VirtualTableZisk1.Trace F) where
  x : {air}.Ctx F
  t : {air}.Trace F
  rows : ∀ j, {air}.everyRow x t j{extra}
"""
    plist = ', '.join(f"⟨{air}.{b}, I.x, I.t⟩" for b in buses)
    body = 'opsProved_nil'
    for add, parts in reversed(oks):
        body = f"""opsProved_cons
    {{ add := {add}
      ltu := {parts['ltu']}
      eq := {parts['eq']}
      and := {parts['and']} }}
    ({body})"""
    pbody = 'pubProved_nil'
    for add, parts in reversed(oks):
        pbody = f"""pubProved_cons
    ({parts['pub']})
    ({pbody})"""
    ok = f"""/-- `{air}`'s arguments on the operation bus. -/
def {inst}.provers {{xv : VirtualTableZisk1.Ctx F}} {{tv : VirtualTableZisk1.Trace F}} (I : {inst} xv tv) :
    List (Prover F) :=
  [{plist}]

theorem {inst}.ok {{xv : VirtualTableZisk1.Ctx F}} {{tv : VirtualTableZisk1.Trace F}} (hT : Tables xv tv)
    (I : {inst} xv tv) : OpsProved I.provers :=
  {body}

theorem {inst}.pub {{xv : VirtualTableZisk1.Ctx F}} {{tv : VirtualTableZisk1.Trace F}} (hT : Tables xv tv)
    (I : {inst} xv tv) : PubProved I.provers :=
  {pbody}
"""
    return struct, ok


def main():
    sigs = signatures()
    airs = census()
    structs, oks, fields, provers, okp, pubp = [], [], [], [], [], []
    for air, buses in airs.items():
        st, ok = air_parts(sigs, air, buses)
        structs.append(st)
        oks.append(ok)
        inst, fname = f"{air}Inst", air[0].lower() + air[1:]
        fields.append(f"  {fname} : List ({inst} xv tv)")
        provers.append(f"(S.{fname}.flatMap {inst}.provers)")
        okp.append(f"opsProved_flatMap (fun I _ => {inst}.ok hT I)")
        pubp.append(f"pubProved_flatMap (fun I _ => {inst}.pub hT I)")
    # `++` associates to the left; the list is nested to the right, as the
    # proof's `opsProved_append` chain is, so the two match without unfolding.
    parts = ['frops S.x0 S.t0'] + provers
    prov = '\n'.join(('  ' if i == 0 else '    ') + x + (' ++ (' if i < len(parts) - 1 else '')
                     for i, x in enumerate(parts)) + ')' * (len(parts) - 1)
    ok = ('  opsProved_append (frops_ok hF) <|\n    '
          + ' <|\n    '.join(f"opsProved_append ({o})" for o in okp[:-1]) + ' <|\n    ' + okp[-1])
    pub = ('  pubProved_append (frops_pub hF) <|\n    '
           + ' <|\n    '.join(f"pubProved_append ({o})" for o in pubp[:-1]) + ' <|\n    ' + pubp[-1])
    frops = ', '.join(f"⟨VirtualTableZisk0.bus{k}, x0, t0⟩" for k in range(1, 13))
    names = textwrap.fill(', '.join(airs), width=78, initial_indent='  ', subsequent_indent='  ')
    text = (HEAD.format(airs=names, frops=frops) + '/-! ### The instances -/\n\n' + '\n'.join(structs)
            + '\n/-! ### Each AIR\'s provers -/\n\n' + '\n'.join(oks) + '\n'
            + TAIL.format(fields='\n'.join(fields), provers=prov, ok=ok, pub=pub))
    open(OUT, 'w').write(text)
    print(f"wrote {os.path.relpath(OUT, HERE)}: {sum(map(len, airs.values()))} arguments in "
          f"{len(airs)} AIRs, and the FROPS table's 12")


HEAD = """/-
  Every prover on the operation bus, collected.

  `run_pilot` (`Run.lean`) takes the provers of the operation bus as a list
  `ps` and assumes `OpsProved ps`: every prover in it computes `add`, `ltu`,
  `eq` and `and`. The files before this one prove that one argument at a time.
  This file collects them, so the hypothesis is discharged for the actual
  provers ZisK has.

  * **The census** is generated: every argument, in every AIR but Main, that
    proves on bus 5000 — 61 arguments in 39 AIRs — plus the 12 arguments of
    the FROPS table, `VirtualTableZisk0`.
  * **An instance** of an AIR (`<Air>Inst`) is its trace, its constraints at
    every row, and the soundness of the buses it assumes (the range checks of
    the adders, Binary's byte lookups, Arith's and BinaryExtension's table
    lookups). An AIR can have any number of instances; a `System` lists them,
    with the two virtual tables.
  * **The tables' contents** (`Tables`, `FropsOk`) are what `check_tables.py`
    checks against the data the setup commits.

  `System.ok`: every prover of every instance computes the four ops.
  `System.pub`: and proves `pubout` correctly — none proves it at all.
  `run_system`: the run of `run_pilot`, over the system's provers, with
  `OpsProved` gone.

  Generated by `sail/gen_provers.py` (`make provers`) from the prover
  lemmas' signatures. The AIRs:\n\n{airs}.
-/
import Proofs.Run
import Proofs.BinaryOps
import Proofs.OpBusCmp
import Proofs.OpBusPub
import Proofs.BinaryPub
import Pil.VirtualTableZisk0

open Out.Functions

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/- The per-AIR theorems are generated with one signature, so most take the
tables (`hT`) and the characteristic without needing them. -/
set_option linter.unusedVariables false
set_option linter.unusedSectionVars false

variable {{F : Type}} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-! ### Lists of provers -/

theorem opsProved_nil : OpsProved ([] : List (Prover F)) := fun _ h => nomatch h

theorem opsProved_cons {{p : Prover F}} {{ps : List (Prover F)}} (h : OpsOk p) (hs : OpsProved ps) :
    OpsProved (p :: ps) := by
  intro q hq
  rcases List.mem_cons.mp hq with rfl | hq
  · exact h
  · exact hs q hq

theorem opsProved_append {{ps qs : List (Prover F)}} (h : OpsProved ps) (h' : OpsProved qs) :
    OpsProved (ps ++ qs) := by
  intro q hq
  rcases List.mem_append.mp hq with hq | hq
  · exact h q hq
  · exact h' q hq

theorem opsProved_flatMap {{α : Type}} {{l : List α}} {{f : α → List (Prover F)}}
    (h : ∀ a ∈ l, OpsProved (f a)) : OpsProved (l.flatMap f) := by
  intro q hq
  obtain ⟨a, ha, hq⟩ := List.mem_flatMap.mp hq
  exact h a ha q hq

theorem pubProved_nil : PubProved ([] : List (Prover F)) := fun _ h => nomatch h

theorem pubProved_cons {{p : Prover F}} {{ps : List (Prover F)}} (h : OpProves 0xA0 pubSem p)
    (hs : PubProved ps) : PubProved (p :: ps) := by
  intro q hq
  rcases List.mem_cons.mp hq with rfl | hq
  · exact h
  · exact hs q hq

theorem pubProved_append {{ps qs : List (Prover F)}} (h : PubProved ps) (h' : PubProved qs) :
    PubProved (ps ++ qs) := by
  intro q hq
  rcases List.mem_append.mp hq with hq | hq
  · exact h q hq
  · exact h' q hq

theorem pubProved_flatMap {{α : Type}} {{l : List α}} {{f : α → List (Prover F)}}
    (h : ∀ a ∈ l, PubProved (f a)) : PubProved (l.flatMap f) := by
  intro q hq
  obtain ⟨a, ha, hq⟩ := List.mem_flatMap.mp hq
  exact h a ha q hq

/-! ### The tables -/

/-- What `check_tables.py` checks of `VirtualTableZisk1`: the Binary table's
rows compute their byte, and the Arith (bus 331) and BinaryExtension (bus 124)
tables never serve a modeled external op. -/
structure Tables (xv : VirtualTableZisk1.Ctx F) (tv : VirtualTableZisk1.Trace F) : Prop where
  binAdds : ∀ p ∈ binaryTable xv tv, BinaryTableAdds p
  binOps : ∀ p ∈ binaryTable xv tv, BinaryTableOps p
  arith : ∀ k ∈ ([10, 6, 9, 14] : List Int),
    TableNever ⟨VirtualTableZisk1.bus16, xv, tv⟩ 331 ((k : Int) : F)
  ext17 : ∀ k ∈ ([10, 6, 9, 14] : List Int),
    TableNever ⟨VirtualTableZisk1.bus17, xv, tv⟩ 124 ((k : Int) : F)
  ext18 : ∀ k ∈ ([10, 6, 9, 14] : List Int),
    TableNever ⟨VirtualTableZisk1.bus18, xv, tv⟩ 124 ((k : Int) : F)
  binNoPub : ∀ p ∈ binaryTable xv tv, BinaryTableNoPub p
  arithP : ∀ k ∈ ([160] : List Int), TableNever ⟨VirtualTableZisk1.bus16, xv, tv⟩ 331 ((k : Int) : F)
  ext17P : ∀ k ∈ ([160] : List Int), TableNever ⟨VirtualTableZisk1.bus17, xv, tv⟩ 124 ((k : Int) : F)
  ext18P : ∀ k ∈ ([160] : List Int), TableNever ⟨VirtualTableZisk1.bus18, xv, tv⟩ 124 ((k : Int) : F)

/-- The FROPS table's arguments on the operation bus. -/
def frops (x0 : VirtualTableZisk0.Ctx F) (t0 : VirtualTableZisk0.Trace F) : List (Prover F) :=
  [{frops}]

/-- What `check_tables.py` checks of a FROPS argument: each modeled op's rows
carry its result. -/
structure FropsOk (p : Prover F) : Prop where
  add : FropsProves 10 addN p
  ltu : FropsProves 6 ltuN p
  eq : FropsProves 9 eqN p
  and : FropsProves 14 andN p
  pub : FropsProves 0xA0 pubN p

theorem frops_ok {{x0 : VirtualTableZisk0.Ctx F}} {{t0 : VirtualTableZisk0.Trace F}}
    (h : ∀ p ∈ frops x0 t0, FropsOk p) : OpsProved (frops x0 t0) := fun p hp =>
  {{ add := opProves_of_frops addN_sem (h p hp).add
    ltu := opProves_of_frops ltuN_sem (h p hp).ltu
    eq := opProves_of_frops eqN_sem (h p hp).eq
    and := opProves_of_frops andN_sem (h p hp).and }}

theorem frops_pub {{x0 : VirtualTableZisk0.Ctx F}} {{t0 : VirtualTableZisk0.Trace F}}
    (h : ∀ p ∈ frops x0 t0, FropsOk p) : PubProved (frops x0 t0) := fun p hp =>
  opProves_of_frops pubN_sem (h p hp).pub

"""

TAIL = """/-! ### The system -/

/-- Every instance of every AIR that proves on the operation bus, and the two
virtual tables. -/
structure System (F : Type) [Lean.Grind.Field F] where
  xv : VirtualTableZisk1.Ctx F
  tv : VirtualTableZisk1.Trace F
  x0 : VirtualTableZisk0.Ctx F
  t0 : VirtualTableZisk0.Trace F
{fields}

/-- The operation bus's provers: every argument of every instance. -/
def System.provers (S : System F) : List (Prover F) :=
{provers}

/-- **Every prover on the operation bus computes the four ops**, given the
tables' contents. -/
theorem System.ok (S : System F) (hT : Tables S.xv S.tv) (hF : ∀ p ∈ frops S.x0 S.t0, FropsOk p) :
    OpsProved S.provers :=
{ok}

/-- **Every prover on the operation bus proves `pubout` correctly**, given the
tables' contents: none of them proves it at all. -/
theorem System.pub (S : System F) (hT : Tables S.xv S.tv) (hF : ∀ p ∈ frops S.x0 S.t0, FropsOk p) :
    PubProved S.provers :=
{pub}

/-- **A run of the pilot's programs, over ZisK's provers.** `run_pilot` with
the operation bus answered by the system's provers: `OpsProved` is no longer
assumed, and in its place are the tables' contents, which `check_tables.py`
checks.

What remains assumed: the soundness of every bus argument involved (Main's
ROM and operation-bus lookups at every lane, and each instance's own lookups
and range checks), the Rom's contents (`RomHolds`), the tables' contents, the
pilot's scope, and the start. -/
theorem run_system (S : System F) (hT : Tables S.xv S.tv)
    (hF : ∀ p ∈ frops S.x0 S.t0, FropsOk p)
    (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProg prog)
    (xm : Main.Ctx F) (tm : Main.Trace F) (hm : ∀ i, Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hbus : ∀ l, l < 4 → LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩])
    (hrom : RomHolds prog tr)
    (hops : ∀ l, l < 4 → LookupHolds (opArg l) xm tm S.provers) (n : Nat) :
    ∀ (s : Pos) (σ : State) (pc zc : BitVec 64), Ready σ pc zc → Encodes xm tm s pc zc →
      (∀ k, k < n → InSegment tm (s.after k)) →
      ∃ σ' pc' zc', Runs prog n σ σ' ∧ Ready σ' pc' zc' ∧ Encodes xm tm (s.after n) pc' zc' :=
  run_pilot prog hprog xm tm hm xr tr hr hbus hrom S.provers hops (S.ok hT hF) n

end Correspondence
"""

if __name__ == '__main__':
    main()
