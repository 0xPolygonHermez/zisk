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

- **The expression DAG**, with sharing intact. Main is 146 constraints over
  1778 expression nodes, about half of which are shared.
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
  [3] every row  main/pil/main.pil:373
       pil: a_src_step*(a[0]-(STEP))
       0 = a_src_step * (a[0] - Main.STEP)
```

The first two lines are the pilout's; the third is this tool rendering the DAG.
They should say the same thing, which is the cheapest available check that the
walker is faithful.

## What the Lean looks like

```lean
/-- `main/pil/main.pil:191` — `addr1-(b_offset_imm0+(b_src_ind*a[0]))` -/
def c1 (x : Ctx F) (t : Trace F) (i : Int) : Prop :=
  (t i).addr1 - ((t i).b_offset_imm0 + (t i).b_src_ind * (t i).a_0) = 0
```

Five decisions shape that output.

**The field is abstract.** A pil2 constraint is a polynomial identity, so the
generated `Pil/Prelude.lean` declares a `PilField` class with addition,
subtraction, multiplication, negation and numerals, and nothing else. This
avoids a Mathlib dependency — the Lean that Sail generates into
`sail/build/lean/out` has no dependencies either — and it sidesteps a question
the pilout does not answer: stage-2 columns and the challenges are elements of
the cubic extension, while stage-1 columns and constants are base field
elements embedded in it. Left abstract, one definition covers both.

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

**Names are the PIL's, mangled only as far as Lean forces.**
`Main.last_reg_value[3][1]` becomes `Main_last_reg_value_3_1`, `__L1__` stays
`__L1__`, and `next_pc'` keeps its apostrophe because Lean allows it and it is
the PIL's own notation. The compiler can emit two intermediates with the same
name — Main has two `Main.previous_c`, one per limb of `c` — so a duplicated
name gets its pilout expression index appended (`Main_previous_c_e22`).

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
row-major. `Main.last_reg_value` is `[31][2]`, occupies air value ids 8 to 69,
and the next symbol starts at 70.

## Known limits

- **The generated Lean is not typechecked by CI, or by anything else yet.**
  There is no Lean toolchain in this repo — `sail/build/lean/out` is a lake
  project nobody builds either. Until one is wired up, "it compiles" is an
  assumption, not a fact.
- **`every_frame` is untested.** Every constraint in the ZisK pilout is
  `every_row`; boundary conditions are expressed with fixed selector columns
  such as `__L1__` and `Main.SEGMENT_L1` instead. The other three kinds are
  implemented and carry their frame bounds through, but nothing has exercised
  them.
- **Fixed column *values* are not extracted.** ZisK compiles with
  `--no-proto-fixed-data`, which keeps them out of the protobuf entirely (they
  go to `tmp/fixed/`), so only the columns' names and identities are available
  here.
- **Hints and global constraints are not extracted.** The lookup and
  permutation arguments the `std` library builds are visible only through the
  columns and `gsum` expressions they leave in the AIR.
