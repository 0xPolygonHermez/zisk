# frops-analyzer

Analyze real operation traces and propose **FROPS** (frequent operations) tables for ZisK.

FROPS are `(op, a, b)` triples that appear so often that precomputing them in a fixed table is cheaper
than proving them as ordinary instance rows. The membership test must be **CPU-bound** — a handful of
integer comparisons, no hashing and no memory access — so the proposed predicates are always simple
half-open *boxes* (`a in [lo, hi)`, `b in [lo, hi)`). See [FROPS.md](FROPS.md) for the full model.

The tool does two things:

1. **`analyze`** — read a directory of trace files and emit a compact, AI-reviewable proposal
   (`proposal.json` + `report.md`). No source files are touched. *(Approach 1)*
2. **`generate`** — additionally regenerate the three `*_frops.rs` source files in place for the
   proposed tables. *(Approach 2)*

## 1. Produce trace files with ziskemu

`ziskemu` dumps every Arith / Binary / BinaryExtension operation it executes as a flat binary of
17-byte records (`1B op` + `8B a` little-endian + `8B b` little-endian):

```bash
ziskemu --elf program.elf --inputs input.bin --store-op-output ops.bin
```

`--store-op-output` works on its own (it implicitly enables the stats execution path). Run it over as
many programs / inputs as you want and collect the resulting `*.bin` files into a directory.

> Tip: name files distinctively (`ops_progA.bin`, `ops_progB.bin`, …). The analyzer reads every
> `*.bin` in the directory (non-recursive).

## 2. Analyze (proposal only)

```bash
frops-analyzer analyze \
    --input ./traces \
    --max-table 4000000 \
    --nodes 1 \
    --table-cost 3 \
    --report-dir ./frops-report
```

Outputs `build/frops-report/proposal.json` (machine-readable; feed it to an AI to refine predicates)
and `build/frops-report/report.md` (human-readable summary: per-op coverage, region predicates, table
usage and area before/after). The default report directory is `build/frops-report`.

Both `analyze` and `generate` also **score the proposal against the FROPS implementation currently in
the tree** over the same data — see the `comparison_vs_current` block in `proposal.json`, the
"Proposed vs current FROPS" section in `report.md`, and the `vs current FROPS:` line printed to the
console (table rows, coverage, area, and which one wins).

## 3. Generate (write the source)

```bash
frops-analyzer generate \
    --input ./traces \
    --max-table 4000000 \
    --workspace . \
    --report-dir ./frops-report
```

This rewrites:

- `state-machines/arith/src/arith_frops.rs`
- `state-machines/binary/src/binary_basic_frops.rs`
- `state-machines/binary/src/binary_extension_frops.rs`

and additionally emits x86-64 assembly macros for counting FROPS multiplicity:

- `emulator-asm/src/frops/frops.s`

One `.macro FROP_<OP> a, b, t0, t1` per operation (GAS `.intel_syntax noprefix`). `a`/`b` are the
operands (register or immediate, read-only); `t0`/`t1` are the only registers the macro clobbers
freely (plus FLAGS; the overflow path also push/pops `rax`/`rcx`/`rdx`). If `(op, a, b)` is a FROP it
does `mult[row] += 1` on the per-family multiplicity table (`.extern frops_<family>_mult`); on a u32
wrap it appends the row offset to `frops_<family>_overflow` at `frops_<family>_overflow_index`. If the
op is not a FROP (or has no FROPS), the macro does nothing. The three multiplicity tables and overflow
vectors are referenced as external symbols (the runtime owns the memory).

The first time it overwrites a file it saves the original as `<file>.rs.bak` (later runs never clobber
that backup). Then:

```bash
# 1. Review the diffs.
git diff state-machines/*/src/*_frops.rs

# 2. Regenerate the fixed .bin tables consumed by the proving backend.
cargo run -p sm-arith  --bin arith_frops_fixed_gen
cargo run -p sm-binary --bin binary_basic_frops_fixed_gen
cargo run -p sm-binary --bin binary_extension_frops_fixed_gen

# 3. Run the generated consistency tests.
cargo test -p sm-arith -p sm-binary
```

Each generated file ships two tests: `test_table_offsets` (the per-op offset table matches what
`build_table` produces) and `test_all_accessible_values` (every materialised pair is found by
`get_row` / `is_frequent_op`).

## Regenerating everything (Rust + assembly + fixed tables)

To change the proposal (e.g. a different `--max-regions-per-op`) and refresh all artifacts:

```bash
# 1. Regenerate the Rust sources AND the x86-64 assembly macros (pick the cap).
frops-analyzer generate \
    --input <traces-dir> --max-table 16777216 --low-cap 4096 --table-cost 3 \
    --max-regions-per-op 8 --workspace .
#    -> state-machines/{arith,binary}/src/*_frops.rs   (the is_frequent_op / get_row / build_table)
#    -> emulator-asm/src/frops/frops.s                 (the FROP_<OP> counting macros)

# 2. Regenerate the fixed .bin multiplicity tables consumed by the proving backend.
cargo run -p sm-arith  --bin arith_frops_fixed_gen
cargo run -p sm-binary --bin binary_basic_frops_fixed_gen
cargo run -p sm-binary --bin binary_extension_frops_fixed_gen

# 3. Verify consistency (offsets + accessibility) on the regenerated tables.
cargo test -p sm-arith -p sm-binary

# 4. (optional) Emit the ORIGINAL hand-tuned FROPS as assembly to compare detection cycles.
frops-analyzer asm-original   # -> emulator-asm/src/frops/frops_original.s
```

Each macro in `frops.s` / `frops_original.s` is annotated with a cost comment (cycles, `imul`=3 else
1): worst case to reject a non-FROP, and best/worst case to count a FROP (no overflow).

## Choosing `--max-regions-per-op` (detection speed vs circuit area)

More regions per op = more FROPS coverage (smaller circuit area) but a longer membership test (more
cycles to reject a non-FROP in the assembly path). It is the **speed ↔ area knob**.

Measured on the 12 mainnet blocks (1.28 B ops; `--max-table 2^24 --low-cap 4096 --table-cost 3`):

| cap | area (no padding) | coverage | total regions | reject worst-case (srl/eq/or, cyc) |
|-----|-------------------|----------|---------------|------------------------------------|
| hand-tuned (original) | 37.749 B          | 39.56%   | ~1–7 / op     | 9–93   |
| **4**                 | 35.967 B (−4.7%)  | 41.41%   | 51            | ~50–68 |
| **8**                 | 35.540 B (−5.9%)  | 42.12%   | 80            | ~99–136 |
| 16                    | 35.197 B (−6.8%)  | 42.62%   | 127           | ~200–289 |

**Recommended cap: between 4 and 8.** Going from 8 to 16 buys only ~0.5 coverage points / 0.34 B area
while roughly doubling the worst-case reject cost — not worth it. Use **4** when emulation/detection
speed matters most, **8** to squeeze a bit more area at ~2× the reject cost. Several ops (`add`, `mul`,
`sub`, `and`) already saturate at cap 8 (no extra regions beyond that).

## Options

| flag | default | meaning |
|------|---------|---------|
| `--input <dir>` | — | directory of `*.bin` trace files |
| `--max-table <N>` | — | maximum **total** FROPS rows across the three tables |
| `--nodes <n>` | `1` | number of distributed nodes (the FROPS table is recomputed per node) |
| `--padding` | off | account for instance padding to each trace's `NUM_ROWS` in the area model |
| `--table-cost <c>` | `3` | per-row area cost of the FROPS table |
| `--low-cap <n>` | `1024` | exclusive bound of the tracked "low value" region for `a` and `b` |
| `--max-regions-per-op <n>` | `16` | cap on FROPS regions per opcode; bounds the cost of `is_frequent_op` / `get_row` (the speed vs coverage knob) |
| `--partition-bits <k>` | `21` | each family's table is padded to a multiple of `2^k` rows (the recursion partition size). `--max-table` then bounds the total **paid** (padded) rows: `Σ_family ceil(rows/2^k)·2^k ≤ max-table`. The optimizer fills the already-paid padding with extra coverage by *growing* selected boxes (same predicate, larger constants → no extra `is_frequent_op` comparisons); a new region is only opened when it pays for its rows at full cost |
| `--report-dir <dir>` | `build/frops-report` | where to write `proposal.json` / `report.md` |
| `--workspace <dir>` | `.` | (`generate` only) workspace root containing `state-machines/...` |

## The two functions FROPS need

The generated code exposes exactly the surface the state machines and the emulator already use:

- `is_frequent_op(op, a, b) -> bool` — the fast membership test (a `match` on the opcode plus the box
  comparisons).
- `get_row(op, a, b) -> usize` — the row of a frequent operation in its table, so its multiplicity can
  be incremented (or `NO_FROPS` if not frequent).

## How the proposal is chosen

For every candidate box, coverage `hits` (observed occurrences inside it) is weighed against its `rows`
(the table area it costs). The optimizer greedily takes the most efficient boxes until the
`--max-table` budget is spent, minimizing total **area = instance area + FROPS-table area**. It reports
this area both without padding (linear) and with padding (`NUM_ROWS` step costs). The result is a
proposal *below* the maximum, not a proven optimum — review it before committing. See
[FROPS.md](FROPS.md) for the area model and predicate templates.
