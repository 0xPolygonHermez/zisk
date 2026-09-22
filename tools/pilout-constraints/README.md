# pilout-constraints

Reads the AIR constraints out of a `.pilout` and writes them as a JSON IR, as
Lean 4 definitions, or as a listing.

```
                    ┌─→ Main.json   the IR: constraints, columns, expression DAG
  zisk.pilout ────→ ├─→ Pil/Main.lean   Lean definitions, one per constraint
                    └─→ Main.txt    a listing, to read and to diff
```

## Why the pilout and not the `.pil` sources

The `.pilout` is the pil2 compiler's output, and it is the only place the
constraints exist in fully resolved form. Re-parsing `state-machines/*/pil/*.pil`
would mean reimplementing the compiler: the templates, the `std` library's
lookup and permutation arguments, constant folding, and the intermediate
columns it introduces. The pilout has all of that already done, and it carries
three things that make it a better source than the text:

- **The expression DAG**, with sharing intact. Main is 609 constraints over a
  9488-node pool, 6764 of which the constraints reach.
- **Resolved names.** A witness operand is `(stage, colIdx, rowOffset)` in the
  protobuf, and `PilOut.symbols` maps that back to `a[0]` or `b_src_ind` — the
  same names the Sail model's `zisk_inst` record uses.
- **A `debugLine` per constraint**: the `.pil` file and line it came from, plus
  the constraint as the source spells it. Every generated constraint keeps it,
  so the output can always be checked against what a person wrote.

It is also the same artifact `pil-helpers` reads to generate
`pil/src/pil_helpers/traces.rs`, so the columns here and the trace layout the
witness computation uses cannot disagree.

## Getting a pilout

`*.pilout` is a build artifact and is gitignored. To produce `pil/zisk.pilout`:

```sh
tools/test-env/setup_build.sh --compile-pil
```

## Use

```sh
cargo run -p zisk-pilout-constraints -- list --pilout pil/zisk.pilout

# One AIR to stdout, or every AIR to a directory.
cargo run -p zisk-pilout-constraints -- text    --pilout pil/zisk.pilout --air Main
cargo run -p zisk-pilout-constraints -- extract --pilout pil/zisk.pilout --air Main --out build/pil
cargo run -p zisk-pilout-constraints -- lean    --pilout pil/zisk.pilout --all  --out sail/build/pil
```

`--air` defaults to `Main`; `--all` takes every AIR in the pilout. `--airgroup`
disambiguates if a pilout ever has more than one. From `sail/`, `make pil` runs
the `lean` step with the repo's paths already filled in.

## What the listing looks like

```
  [1] every row  main/pil/main.pil:224
       pil: addr1[0]-(b_offset_imm0[0]+(b_src_ind[0]*a[0][0]))
       0 = addr1[0] - (b_offset_imm0[0] + b_src_ind[0] * a[0][0])
```

The first two lines are the pilout's; the third is this tool rendering the DAG.
They should say the same thing, which is the cheapest available check that the
walker is faithful.

## What the Lean looks like

```lean
/-- `main/pil/main.pil:224` — `addr1[0]-(b_offset_imm0[0]+(b_src_ind[0]*a[0][0]))` -/
def c1 (x : Ctx F) (t : Trace F) (i : Int) : Prop :=
  (t i).addr1 0 - ((t i).b_offset_imm0 0 + (t i).b_src_ind 0 * (t i).a 0 0) = 0
```

Main packs four instructions into each row, which is why every column is
indexed: `addr1 0` is the first of the row's four slots, and the same `.pil`
line produces four constraints. Six decisions shape the output.

**The ring is abstract, and is Lean's own.** A pil2 constraint is a polynomial
identity, so what it needs is a commutative ring. Lean's core ships one —
`Lean.Grind.CommRing` — with the laws attached and the `grind` tactic able to
reason with it, so `Pil/Prelude.lean` is a one-line abbreviation over it and
**no Mathlib is needed**, for the AIRs or for proofs about them. (The Lean Sail
generates into `sail/build/lean/out` has no dependencies either, and its
toolchain is a PR release that Mathlib would not match anyway.)

Leaving the ring abstract also sidesteps a question the pilout does not answer:
stage-2 columns and the challenges are elements of the cubic extension, while
stage-1 columns and constants are base field elements embedded in it. Stated
over any commutative ring, one definition covers both.

**A row index is an `Int`, and the trace is asserted periodic.** Row offsets in
the ZisK pilout run from `-50400` to `+1008`, and PIL offsets wrap around the
trace. With `Fin numRows` every offset would carry a modular-arithmetic
obligation; with `Int` plus `cyclic t : ∀ i, t (i + numRows) = t i`, `t (i - 1)`
is just subtraction.

**`Row` and `Ctx` are separate.** `Row` holds what varies per row (committed,
fixed, periodic and custom-commit columns); `Ctx` holds what does not (air
values, challenges, publics, proof values, airgroup values). Splitting them
keeps `t` a function of the row index alone.

**Shared subexpressions get names, the rest are inlined.** One definition per
node would be unreadable, and inlining everything would duplicate the shared
half of the DAG. So a node gets its own definition when the PIL named it, when
more than one place uses it, or when its inlined form would exceed
`print::MAX_INLINE` nodes; everything else is folded into its user. That is why
`c1` above reads like the PIL line it came from.

**A PIL array is one field of function type, not one field per element.**
`Row` and `Ctx` declare a field per PIL *symbol*: `a : Nat → Nat → F` for
`a[4][2]`, so `a[2][0]` reads as `(t i).a 2 0`. The reason is not cosmetic.
Lean's structure elaboration is superlinear in the field count, and flattening
arrays gave Keccakf 599 fields across the two structures, which took **over ten
minutes** to elaborate before a single constraint was reached; grouped, that
AIR declares 32 fields and the file elaborates in 42 seconds. The index type is
`Nat` rather than `Fin n` because only in-range indices are ever generated, and
`Nat` keeps them plain numerals instead of coercions — the tool's own check
verifies every emitted index against the declared extent.

**Names are the PIL's, mangled only as far as Lean forces.**
`Main.last_reg_value` keeps its name and takes its three indices as arguments,
`__L1__` stays `__L1__`, and `next_pc'` keeps its apostrophe because Lean
allows it and it is the PIL's own notation. The compiler can emit two
intermediates with the same name — an earlier pilout had two
`Main.previous_c`, one per limb of `c` — so a duplicated name gets its pilout
expression index appended (`Main_previous_c_e22`). The current pilout happens
to have no duplicates.

## Typechecking the output

The generated Lean is checked by the lake package in [`sail/lean`](../../sail/lean),
which points at `sail/build/pil` rather than holding sources of its own:

```sh
cd sail && make pil    # generate
cd sail && make pil-build   # typecheck
```

Needs a Lean toolchain — install [elan](https://lean-lang.org/install/). All 54
AIRs of the ZisK pilout elaborate: about 75s for the set, 42s of that being
Keccakf alone. That is the check that matters for this tool — an expression
printed with the wrong precedence, a reference to a column that was never
declared, or a name Lean will not accept all fail here.

The generated files raise `maxHeartbeats`, `maxRecDepth` and
`linter.unusedVariables`, the same three options sail's own Lean output sets.
The first is not optional: at the default limit the wider AIRs exhaust their
heartbeats while elaborating, and because the failure lands on `Row` itself,
every later field access fails with it — 2 real errors became 16520.

## What the output can prove

Elaborating is not the same as being usable, so [`sail/lean/Proofs/Main.lean`](../../sail/lean/Proofs/Main.lean)
proves five things from the generated Main and nothing else — `cd sail && make proofs`:

```lean
/-- `main.pil:224` determines `addr1`, so the column can be eliminated. -/
theorem addr1_eq (h : c1 x t i) :
    (t i).addr1 0 = (t i).b_offset_imm0 0 + (t i).b_src_ind 0 * (t i).a 0 0 := by
  unfold c1 at h; grind
```

They cover the three moves a correspondence proof needs constantly: turning a
vanishing polynomial into an equation, combining two constraints, and reaching
one constraint out of `holds` without unfolding the other 608. All five go
through by `grind` alone, in under a second.

Two limits surfaced while writing them, both worth knowing before planning a
larger proof:

- **Booleanity is not a constraint.** `a_src_mem` is `bits(1)` in the PIL, but
  that range check is a lookup argument — it lives in the hints, which this
  tool does not extract. So `a_src_mem * (1 - a_src_mem) = 0` cannot be had
  from the AIR, and a proof that needs it must take it as a hypothesis.
- **Ring laws are not field laws.** Mutual exclusion of two selectors follows
  from the constraints only up to a factor of two, because dividing by two
  needs `2 ≠ 0`. A proof needing that instantiates `F` at the concrete field;
  every generated AIR records its characteristic as `basePrime`.

## The IR

`extract` writes one JSON file per AIR:

| Field | What it holds |
|-------|---------------|
| `pilout` | Name, file, blake3, base field, stage count. The blake3 is the digest `pil-helpers` stamps as `PILOUT_HASH` in `pil/src/pil_helpers/traces.rs`, so an IR from a different compile is recognizable. |
| `air` | Airgroup and AIR name and id, row count, committed width per stage. |
| `columns` | Witness, fixed, periodic and custom-commit columns, one entry per scalar slot, plus the named intermediates. |
| `globals` | Air values, airgroup values, publics, proof values, challenges. |
| `expressions` | The expression pool. **Indices are the pilout's own**, including nodes no constraint reaches — those belong to hints and to the global constraint. `Ir::reachable` gives the subset the constraints use. |
| `constraints` | Kind (`every_row`, `first_row`, `last_row`, `every_frame`), the expression index asserted to vanish, and the `debugLine`. |

PIL arrays are flattened the way the pilout numbers them: consecutive ids,
row-major. `Main.last_reg_value` is `[4][31][2]`, so it covers 248 air value
slots, ids 9 to 256, and the next symbol (`Main.last_reg_mem_step`) starts at
257. The IR keeps one entry per scalar; the Lean backend regroups them.

## Known limits

- **CI does not typecheck the generated Lean.** `make pil-build` does, but it
  needs a Lean toolchain that the PR workflow does not install. Until it does,
  the check is one a person has to remember to run.
- **`every_frame` is untested.** All 4066 constraints in the ZisK pilout are
  `every_row`; boundary conditions are expressed with fixed selector columns
  such as `__L1__` instead. The other three kinds are
  implemented and carry their frame bounds through, but nothing has exercised
  them.
- **Fixed column *values* are not extracted.** ZisK compiles with
  `--no-proto-fixed-data`, which keeps them out of the protobuf entirely (they
  go to `tmp/fixed/`), so only the columns' names and identities are available
  here.
- **Hints and global constraints are not extracted.** The pilout carries 4211
  hints and 12 global constraints; the lookup and permutation arguments the
  `std` library builds are visible here only through the columns and `gsum`
  expressions they leave in the AIR.
