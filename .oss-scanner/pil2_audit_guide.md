# PIL2 Constraint Audit Guide (for OSS Scanner)

How the ZisK team audits PIL2 constraint systems for soundness. Use it when reviewing the PIL files
of ZisK (`pil/`, `state-machines/*/pil`, `precompiles/*/pil`) and of the pil2-proofman standard
library (`/pil2-proofman/pil2-components/lib/std/pil/`). It complements
[threat_model.md](threat_model.md), which takes precedence: severities follow its section 7 and
reports follow its section 9. Where this guide says a check is High or Critical, that is the impact
if the issue is reachable, not a severity to assign without an exploit path.

## 1. Adversarial prover model

### Witness, completeness and soundness

A PIL2 airtemplate defines constraints, not how the witness is computed. The witness (every trace
cell and every scalar) is filled by external code (Rust in ZisK: the state machines and the
executor). From the proof system's point of view that code is a generic prover, so the constraints
and the bus balance are the only thing protecting the verifier.

| Witness builder | Outcome |
| --- | --- |
| Correct | The witness satisfies the constraints and the proof generates. |
| Buggy | The proof fails to generate: a **completeness** problem. |
| Malicious | Either rejected (desired) or accepted through a constraint that is too permissive: a **soundness** problem, the main target of the audit. |

Audit the constraints, not the builder. Read the builder only when (a) the intent of a constraint
is ambiguous from the PIL alone, (b) a finding depends on whether a specific witness shape can be
produced, or (c) a `@hint` block is only readable in the builder. Builder bugs break completeness,
not soundness, and are reported as such.

### What the prover controls

| Construct | Set freely by the prover | Constrained only by |
| --- | --- | --- |
| `col witness x` | Per row, per instance | The `===` constraints that use `x` |
| `airval v` | Per instance | The `===` constraints that use `v` |
| `airgroupval v` | Per-instance contribution, aggregated by sum or product | The aggregation rule and any `===` |
| `proofval v` | Once per proof | The `===` constraints that use `v` |

Any of these with no `===` mentioning it is free, and the freedom must be analysed. The prover
cannot override `col fixed` columns (preprocessed, part of the verifier's view), committed `public`
inputs, a `===` constraint, or a std-library argument (the bus must vanish).

The verifier sees exactly: the fixed columns, the public inputs, the per-stage trace commitments,
the challenges (re-derived by Fiat–Shamir), the `===` constraints evaluating to 0 on every row, and
the bus sums vanishing per opid. **Any property that does not reduce to one of these is not
enforced.**

### Bus balance as a soundness mechanism

Every `*_proves`, `*_assumes` and `direct_update_*` call adds a contribution to the bus of its
opid, and every opid's bus must sum to zero (LogUp or grand product). So a scalar that is free at
the constraint level can still be forced by the bus:

```pil
airval k;
direct_update_assumes(opid, [const_tuple], sel: k);   // adds k copies of const_tuple
```

`k` has no `===`, but only one value balances the bus, so this is **not** an unconstrained-witness
bug. The reasoning fails, and it is a finding, when the scalar is used in anything beyond a degree-0
bus selector without its own constraint: inside a polynomial identity, or as a value inside a
`direct_update` tuple rather than as its selector.

### Lookup tables

The prover filling a `lookup_proves` air chooses which tuples appear and how many times, subject
only to that air's per-row constraints. Those constraints and range checks are therefore the
soundness boundary of the table: if they admit an invalid tuple (for example operands wider than
declared because no range check was emitted), the table can publish it, and whether that is
exploitable depends on the assumers.

## 2. PIL2 semantics that matter for soundness

- **`bits(N)` is metadata only.** `col witness bits(8) x` does not constrain `x`; an explicit
  `range_check` (or a lookup against a bounded table) is required.
- **Row references wrap.** `a'` on the last row reads row 0 and `'a` on the first row reads row
  N-1. Transition constraints need a fixed selector that zeroes the wrap-around row, e.g.
  `(1 - LAST) * (a' - a - 1) === 0`.
- **`c <== expr`** is a constraint `c === expr` plus a witness hint. A plain `@hint` is **not** a
  constraint: a hint without a matching constraint is a soundness bug.
- **Boolean columns** need `x * (1 - x) === 0`, explicitly or implicitly through a lookup against a
  `{0,1}` fixed column. Selectors must be boolean and must mask padding rows.
- **Field arithmetic** wraps modulo `PRIME` (Goldilocks, `2^64 - 2^32 + 1`). Negative values are
  `PRIME - |v|`. Watch sums of products and large constants that can wrap, and sign-related
  off-by-ones in carry chains.
- **Degree**: constraints above `MAX_CONSTRAINT_DEGREE` are decomposed by the compiler with
  intermediate witness columns. LogUp adds about one degree to compressed lookup expressions.
- **Stages and challenges**: a `stage(S)` column or value may only depend on challenges and data of
  earlier stages; challenges may only be used after their declared stage.
- **`airgroupval`** must declare a correct `default(...)` (0 for sum, 1 for product) when not every
  air of the airgroup contributes to it.
- **Public inputs** must be bound to trace values (e.g. `L1 * (a - in1) === 0`), not just declared.
  Commit bindings such as `commit stage(0) public(rom_root) rom` must be present where expected.
- **Virtual airs** (`virtual X(...)`) are not in the pilout and may not contain `===` (the compiler
  rejects it); their N does not have to be a power of 2. Check the instantiation site before
  flagging a non-power-of-2 N.
- **Fixed-column sequences**: `[a..b]` includes both ends (`[0..255]` is 256 values); `[...]...`
  pads with the last value; `[...]:n` repeats.
- **Precedence**: `-2**8` is `-256`; `a & 0x80 ? 1 : 0` parses as `a & (0x80 ? 1 : 0)`.

## 3. Standard library internals

Source: `/pil2-proofman/pil2-components/lib/std/pil/` (`std_constants.pil`, `std_lookup.pil`,
`std_permutation.pil`, `std_range_check.pil`, `std_connection.pil`, `std_direct.pil`,
`std_virtual_table.pil`, `std_sum.pil`, `std_prod.pil`), with Rust helpers in `../rs/`.

- **Buses**: `PIOP_BUS_SUM` (LogUp) for lookups, range checks and default permutations;
  `PIOP_BUS_PROD` (grand product) only as an alternative for permutations. A lookup on a product bus
  is a misuse.
- **Lookups**: `lookup_assumes(opid, exprs, sel)` and `lookup_proves(opid, exprs, mul)`, plus
  `lookup(...)` (sign of `mul` chooses the side) and the `_dynamic` variants (opid chosen at run
  time; every opid in the array needs a matching prover).
  - `mul = 1` (the default) means one table row matches one query: if a tuple is queried twice it
    must appear in two rows, and N must fit the total query count of all assumers.
  - `mul` as a witness column is the shared-table / frequent-operations pattern. LogUp does not
    enforce `mul >= 0`; a wrong sign only unbalances the bus, so soundness does not depend on it.
  - `mul` as a fixed column is almost always wrong.
  - Inside a virtual instantiation `lookup_proves` does not emit a sum constraint: it collects the
    table, which is later packed into a `VirtualTable` air with a `UID` column.
- **Permutations** use a boolean `sel` on both sides, and both sides must select the same number of
  rows.
- **Range checks** all reduce to lookups. Opids `100` (U8) and `101` (U16) are the predefined tables
  (`predefined: 1`); every other `(min, max)` pair gets a dynamically allocated opid from `102`
  upwards, shared by identical ranges. Ranges need `min <= max`, at least 8 values (smaller ones use
  a direct polynomial check) and at most `(PRIME - 1) / 2` values; negative ranges are allowed.
  There is no compile-time check that user opids do not collide with the dynamic range opids. ZisK's
  opids are in `pil/opids.pil`, and several are in the low hundreds, so check them against the
  ranges allocated in practice.
- **Connections** reduce to a permutation check over cosets; `connection_connect(opid)` must be
  called to finalize an online connection.
- **Direct updates** (`direct_update*`, `direct_global_update`) contribute once per air (or per
  proof for the global variant), not per row, and accept only degree-0 expressions.

## 4. Audit procedure

The phases are dimensions of analysis, not a strict sequence; skip a phase when the template has
no relevant constructs.

**Phase 1: structure.** Read the instantiation (`pil/zisk.pil`): airgroups, airtemplates, sizes,
`virtual`. Map public inputs and confirm they are constrained. List every lookup, permutation and
direct-update opid the template touches; check that every `_assumes` has a matching `_proves` with
the same tuple width and bus type, and that opids do not collide with the std reserved ranges.

**Phase 2: constraint soundness, per airtemplate.**
1. Parameter validation: if the body hard-codes assumptions about a constructor parameter, an
   explicit `error(...)`/`assert` must reject other values.
2. Boundary constraints on the first and last rows, with selectors.
3. Transition constraints guarded against the wrap-around row.
4. Selectors boolean and masking inactive or padding rows.
5. Row offsets correct on the first and last rows.
6. Every 0/1 column boolean-constrained (explicitly or via a lookup).
7. Constraint degree (selector × witness × witness products, the LogUp contribution).
8. Field overflow in sums of products and large constants.
9. Negative values and carry chains.

**Phase 3: argument integrity.**
1. The proves side covers every `(op, args)` tuple an assumer can query.
2. Multiplicity semantics as in section 3.
3. `sel = 0` removes the row's bus contribution.
4. Tuple widths match per opid; permutation row counts match.
5. Every column in a lookup or permutation tuple has its range or booleanity enforced somewhere.
6. Dynamic lookups: every opid has a prover. Bus types match on both sides.

**Phase 4: scalars and stages.** Stage ordering and challenge use, `airgroupval` defaults and
contributions, public inputs and commit bindings (section 2). Scalars used as `direct_update`
selectors or multiplicities: confirm their only uses are degree-0 bus contributions (section 1).

**Phase 5: adversarial pass** over the previous findings:
1. For every witness column, scalar and hint: what if the prover sets an arbitrary value? It must be
   fixed by `===`, gated by a selector locked to 0, or forced by bus balance; otherwise it is a
   finding.
2. Every `<==` has a constraint that uniquely determines the witness.
3. For every implicit constraint through a lookup against a fixed table: would removing the lookup
   silently remove the constraint?
4. Cross-check with the Rust witness builder where the intent is unclear (section 1).

### Table airs

A table air declares only `col fixed` columns filled by a constructor loop, a single
`col witness multiplicity`, no `===`, and one trailing `lookup_proves` (or `lookup_proves_dynamic`).
For these, soundness means: **is every published tuple a correct and complete encoding of the
operation it claims?** Phase 2 reduces to parameter validation, Phase 4 usually does not apply, and
Phase 3 applies plus:

1. **Constructor exhaustiveness**: the dispatch over `OP` covers every value the `OP` column takes,
   with `default: error(...)`. A missing case produces silent zero-output rows.
2. **Per-op output correctness**: hand-compute interesting combinations (boundaries, sign-bit flips,
   carry chains) and compare with the reference semantics. A wrong row that any querier can reach
   forges operation results.
3. **Context-column coverage**: the tiling of the fixed columns enumerates the full product of
   contexts each op needs; off-by-one repetition counts in `[a:N1, b:N2]:R` are the typical bug.
4. **Table size**: `N >= TABLE_SIZE`; if larger, the padding tail (`[...]...` repeats the last
   value) must not be queryable.
5. **Tuple shape vs queriers**: width and slot meaning match every `lookup_assumes` consumer;
   co-audit at least one querier.
6. **Multiplicity** is `col witness`, not `col fixed`.
7. **Reference cross-check**: for tables encoding known semantics (RISC-V ALU, curve operations, hash
   rounds), the constructor can be instrumented with `#pragma transpile logfile:...` (see
   `state-machines/binary/pil/binary_table.pil`) and diffed against a Rust or Python reference.

### Edge-case final pass

Division by zero in any inverse; inclusive ranges (`[0..255]` is 256 values); empty or tiny traces
(N = 0 or 1, mostly relevant for virtual airs); literals close to or above `PRIME`.

## 5. Recurring patterns

### Padding absorption (`airval` + `direct_update_assumes`)

Instead of a per-row enable column, the template lets padding rows (all inputs 0) produce a fixed
tuple on the proves side, and cancels them with one `direct_update_assumes(opid, T_padding,
sel: padding_size)` (in ZisK, `assumes_padding_operation(...)` from `pil/operations.pil`, used by
Binary, BinaryAdd, BinaryExtension, Arith and DMA). The prover must set `padding_size` to exactly the number of
padding rows or the bus does not balance. Verify:

1. The tuple an all-zero row produces equals `T_padding` exactly (width and values).
2. If a real operation can produce the same tuple (e.g. `ADD(0, 0) = 0`), the accounting is still
   correct. Usually it is, because both contribute identically, but confirm it.
3. `padding_size` is degree-0 (an `airval`) and `T_padding` is a constant expression.
4. No constraint limits `padding_size` in a way that breaks it (e.g. a boolean constraint).

### One row = one query (`lookup_proves` with `mul = 1`)

Each row is one operation consumed by one assumer. N must cover the summed query count of every
assumer of the opid. Range checks on the table's inputs are **not** automatic (next pattern).

### Lookup table without input range checks

`bits(32)` on the table's columns does not constrain them. If the table emits no `range_check` on
its inputs, either every assumer range-checks its own inputs (intentional, to save cost) or the
check was forgotten; the two look the same from the table alone. Find every assumer of the opid and
confirm each constrains its inputs to the chunk size the table assumes. A missing assumer-side check
is a soundness finding; an intentional contract that is not documented at the top of the table file
is only a recommendation.

### Several airtemplates proving the same opid

Two templates may publish to the same opid (e.g. the generic `Binary` and the specialized
`BinaryAdd` with a simpler tuple). Which one serves a query is the prover's choice and nothing
enforces routing. For every assumer, determine which template its tuple shape matches; if it can
match both in a way the design does not intend, the bus accounting can break (soundness or
completeness depending on how rows are filled).
