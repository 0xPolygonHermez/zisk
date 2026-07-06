# FROPS — Frequent Operations (model for AI review)

This document explains FROPS well enough for an AI (or engineer) to read `proposal.json` and propose
better predicates. It is intentionally self-contained and does not depend on any particular trace.

## What a FROP is

ZisK proves each executed operation `(op, a, b)` as a row in a state-machine instance. Some triples
recur enormously (small multiplications, shifts by a constant, comparisons against a fixed address,
…). For those it is cheaper to put the triple **once** into a fixed lookup table and, at runtime, just
increment the multiplicity of that table row instead of emitting a fresh instance row.

A triple is a FROP if a fast test says so. There are exactly two operations the rest of the codebase
needs:

- `is_frequent_op(op, a, b) -> bool` — must be as fast as possible.
- `get_row(op, a, b) -> usize` — the row index in the table (or `NO_FROPS = usize::MAX`).

The whole point is speed: the test runs on **every** executed operation. So it must be **CPU-bound** —
a `match` on the 1-byte opcode plus a few integer comparisons. **No hash maps, no memory loads.** That
constraint is what shapes everything below.

## Three tables, three source files

FROPS are split by operation family, one generated source file each:

| table | file | ops | `TABLE_ID` |
|-------|------|-----|-----------|
| arith | `state-machines/arith/src/arith_frops.rs` | `mul*`, `div*`, `rem*`, … (0xb0–0xbf) | 5010 |
| binary_basic | `state-machines/binary/src/binary_basic_frops.rs` | `add`, `sub`, `eq`, `lt*`, `and`, `or`, `xor`, … (0x02–0x1d) | 5011 |
| binary_extension | `state-machines/binary/src/binary_extension_frops.rs` | `sll`, `srl`, `sra`, `signextend*`, … (0x21–0x29) | 5012 |

`--max-table` bounds the **sum** of rows across all three.

The fixed `.bin` columns are produced from these sources by the existing
`*_frops_fixed_gen.rs` binaries via `sm_frequent_ops::FrequentOpsHelpers`.

## Predicate shape: half-open boxes

Every predicate is a box:

```
a in [a_lo, a_lo + a_count)   and   b in [b_lo, b_lo + b_count)
```

This single shape is enough to express the cheap patterns that actually occur, and its row offset is a
closed-form linear expression:

```
row_within_box = (a - a_lo) * b_count + (b - b_lo)      // row-major over b
```

Common specializations (all the same shape, all CPU-cheap):

| pattern | box | example predicate |
|---------|-----|-------------------|
| low rectangle | `a_lo=0, a_stride=1, b_lo=0` | `a < 386 && b < 386` |
| high mask | a span reaches `u64::MAX` | `a >= 0xFFFFFFFFFFFFF000 && b < 65` |
| address range | finite `a_lo > 0` | `a >= 0xA0100000 && a < 0xA0120000 && b < 8` |
| strided range | `a_stride > 1` (power of two) | `a >= 0xA0100000 && a < 0xA0120000 && (a & 7) == 0 && b < 8` |
| b-constant | `b_count == 1` | `a < 1024 && b == 0` |

A strided box covers `{a_lo + i*a_stride : 0 <= i < a_count}`. The analyzer detects the common low-bit
alignment of an op's mid-region addresses and, when they all share it (e.g. all 8-byte aligned), emits
the strided predicate — ~`a_stride`× fewer rows than the contiguous box for the same hits. The offset
becomes `((a - a_lo) / a_stride) * b_count + (b - b_lo)`.

A box predicate is at most two comparisons per coordinate. An op may have several **disjoint** boxes
(e.g. a low rectangle *and* a high mask), checked in order; `get_row` returns the first match's offset
plus that box's base within the op.

### b-splitting and the regions-per-op cap

Address regions often pair a wide span of `a` with only a few distinct `b` values (sizes, offsets,
constants). Instead of one bloated box `... && b < max_b`, the analyzer detects the `b` clusters in a
region and emits one tight box per cluster (the `b`-analog of the `a`-stride). This is the single
biggest coverage lever after stride.

Because each extra box adds a comparison to the (hot) membership test, `--max-regions-per-op`
(default 16) caps how many boxes an opcode may have, keeping the highest-coverage ones. It is the
**speed vs coverage knob**: raising it captures a longer tail of small boxes at the cost of a longer
predicate chain; lowering it keeps the test cheaper. Tiny boxes below a hit floor are pruned outright.

### Row layout invariant (why offsets line up)

Rows are grouped per opcode in **ascending opcode order**; within an opcode they follow box order, and
within a box they are **row-major over `b`** (the loop is `for a { for b { push([a,b]) } }`).
`OP_TABLE_OFFSETS[op - START]` is the cumulative row count of all lower opcodes — exactly what
`FrequentOpsHelpers::generate_table_offsets()` recomputes, so `test_table_offsets` passes. If you hand-
edit a predicate, keep this invariant or the offset table and the generated tests will disagree.

## Area model (how "optimal" is measured)

Total area of a proof block = **instance area** + **FROPS-table area**.

- **FROPS-table area** = `table_rows * table_cost * nodes`. `table_cost` defaults to 3. It is
  multiplied by `nodes` because every distributed node recomputes the whole table.
- **Instance area, no padding** = `Σ_op (occurrences - covered) * cost(op)`. Each executed, *non-
  covered* operation is one row; `cost(op)` is the per-row area from `core/src/zisk_ops_costs.rs`
  (Arith 95, Binary 60, BinaryAdd 25, BinaryExtension 53).
- **Instance area, with padding** (`--padding`) = `Σ_sm ceil(used_sm / NUM_ROWS_sm) * NUM_ROWS_sm *
  cost_sm`. Each state machine pads to its trace `NUM_ROWS` (`ArithTrace` = 2^21, the binary traces =
  2^22), so covering operations only saves area when it drops a whole instance.

A box is worth adding when `hits * cost(op) > table_cost * nodes * rows`, i.e. the instance rows it
removes outweigh the table rows it adds. The optimizer takes boxes by descending
`hits * cost / rows` until `--max-table` is exhausted.

Op → state machine mapping used by the padding model:

| state machine | ops | NUM_ROWS | per-row cost |
|---------------|-----|----------|--------------|
| Arith | all arith ops | 2,097,152 | 95 |
| BinaryAdd | `add` (0x0a) only | 4,194,304 | 25 |
| Binary | the other binary-basic ops | 4,194,304 | 60 |
| BinaryExtension | binary-extension ops | 4,194,304 | 53 |

## The trace file format

Input to the analyzer is one or more flat binaries of 17-byte little-endian records produced by
`ziskemu --store-op-output`:

```
offset 0      : op   (u8)
offset 1..9   : a    (u64 LE)
offset 9..17  : b    (u64 LE)
```

Only Arith / Binary / BinaryExtension ops are written; everything else (precompiles, control flow) is
absent, so the analyzer never sees non-candidate opcodes except as `skipped`.

## proposal.json schema

```jsonc
{
  "config":  { "max_table", "nodes", "padding", "table_cost", "low_cap" },
  "input":   { "files", "records", "frops_candidate_records", "skipped_non_frops", "trailing_bytes" },
  "tables":  { "arith", "binary_basic", "binary_extension", "total", "max_table" },
  "area": {
    "baseline_no_padding": { "instances", "table", "total" },   // no FROPS at all
    "proposed_no_padding": { "instances", "table", "total" },
    "baseline_padding":    { ... },
    "proposed_padding":    { ... },
    "savings_pct_no_padding", "savings_pct_padding"
  },
  "comparison_vs_current": {                 // proposal scored against the in-tree FROPS, same data
    "current":  { "table_rows", "covered_hits", "coverage_pct", "area_total_no_padding", "area_total_padding" },
    "proposed": { ... same keys ... },
    "delta":    { "table_rows", "coverage_pct_points", "area_no_padding", "area_padding",
                  "proposed_is_better_no_padding", "proposed_is_better_padding" }
  },
  "ops": [                                   // sorted by occurrences, descending
    {
      "code", "name", "table", "sm", "cost",
      "occurrences", "covered", "coverage_pct",
      "current_covered", "current_coverage_pct",   // what the in-tree FROPS cover for this op
      "regions": [
        { "kind",        // low_rect | mid_box | high_box
          "predicate",   // e.g. "a < 386 && b < 386"
          "a_lo", "a_count", "b_lo", "b_count",
          "rows",        // a_count * b_count (table rows this box costs)
          "hits" }       // observed occurrences this box covers
      ]
    }
  ],
  "dropped": [ { "code", "name", "kind", "predicate", "rows", "hits" } ]  // net-positive but over budget
}
```

## How an AI can improve a proposal

The auto-optimizer emits low rectangles, mid address-range boxes (contiguous or strided), and high
mask boxes, picking a single best box per template per op. Looking at `proposal.json` plus the per-op
stats you can often do better by:

- **Tightening a box.** If a low rectangle is `a < 1024 && b < 1024` but `coverage_pct` is high with
  most hits at small `b`, a `b`-constant or narrow-`b` box has far fewer rows for nearly the same hits.
- **Raising `--low-cap`.** The low region is only tracked up to `--low-cap` (default 1024). If a low
  rectangle is clamped exactly at 1024 (e.g. `a < 1024`), re-run with `--low-cap 4096` to see whether
  there is worthwhile mass beyond it.
- **Refining stride.** The analyzer detects stride *per cluster* (each contiguous address run from
  its own pages), so different clusters of one op can use different strides. If a single cluster still
  mixes aligned and unaligned addresses it falls back to a contiguous box; a tighter manual mask can
  shrink it further.
- **Splitting an op into disjoint boxes** when its hits cluster in two regions (e.g. tiny values *and*
  near-`-1` values) instead of one big bounding box.
- **Dropping a box** whose `rows` dwarf its `hits` once you account for `table_cost * nodes` — those
  rows are better spent on a denser op.

When proposing changes, keep every predicate a CPU-bound box (or box + cheap mask), preserve the
ascending-opcode / row-major layout invariant, and keep the total rows ≤ `--max-table`.
