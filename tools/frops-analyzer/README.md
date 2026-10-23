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

and additionally emits the box data those predicates were generated from:

- `core/src/frops_regions.rs`

That module is the single source of truth: `zisk_core::frops_asm` generates the x86-64 code that
counts frequent operations in the ROM-histogram assembly from the very same boxes, so the assembly
and the state machines can never disagree about which triples are covered or which row each one
lands on. Rows there are numbered over the three family tables concatenated
(`FROPS_ARITH_BASE`, `FROPS_BINARY_BASIC_BASE`, `FROPS_BINARY_EXT_BASE` split them again).

The first time it overwrites a file it saves the original as `<file>.rs.bak` (later runs never clobber
that backup). Then:

```bash
# 1. Review the diffs.
git diff state-machines/*/src/*_frops.rs

# 2. Regenerate the fixed .bin tables consumed by the proving backend.
cargo run --release -p zisk-sm-arith  --bin zisk-arith-frops-fixed-gen
cargo run --release -p zisk-sm-binary --bin zisk-binary-basic-frops-fixed-gen
cargo run --release -p zisk-sm-binary --bin zisk-binary-extension-frops-fixed-gen

# 3. Run the generated consistency tests.
cargo test -p zisk-sm-arith -p zisk-sm-binary
```

Each generated file ships two tests: `test_table_offsets` (the per-op offset table matches what
`build_table` produces) and `test_all_accessible_values` (every materialised pair is found by
`get_row` / `is_frequent_op`).

## Regenerating everything (Rust + assembly + fixed tables)

To change the proposal (e.g. a different `--max-regions-per-op`) and refresh all artifacts:

```bash
# 1. Regenerate the Rust sources AND the box data the assembly is generated from (pick the cap).
frops-analyzer generate \
    --input <traces-dir> --max-table 16777216 --low-cap 4096 --table-cost 3 \
    --max-regions-per-op 8 --workspace .
#    -> state-machines/{arith,binary}/src/*_frops.rs   (the is_frequent_op / get_row / build_table)
#    -> core/src/frops_regions.rs                      (the boxes, shared with the asm generator)

# 2. Format them: the emitted `*_frops.rs` are not rustfmt-clean (`frops_regions.rs` is).
cargo fmt --all

# 3. Regenerate the fixed .bin multiplicity tables consumed by the proving backend.
cargo run --release -p zisk-sm-arith  --bin zisk-arith-frops-fixed-gen
cargo run --release -p zisk-sm-binary --bin zisk-binary-basic-frops-fixed-gen
cargo run --release -p zisk-sm-binary --bin zisk-binary-extension-frops-fixed-gen

# 4. Verify consistency (offsets + accessibility) on the regenerated tables.
cargo test -p zisk-sm-arith -p zisk-sm-binary

# 5. Check that the boxes, the generated predicates and the emitted assembly all agree.
cargo test -p zisk-core --lib frops
```

## Cross-checking the assembly against Rust

The FROPS multiplicity column has two independent producers: the ROM-histogram assembly
(`zisk_core::frops_asm`) and `zisk_core::frops::FropsMultiplicity`. The proof's lookup argument only
balances if whichever one is used agrees with what the state machines claim, so they can be compared
directly over a real execution:

```bash
# 1. Run the ROM-histogram assembly with -f: it dumps its whole output (control header, instruction
#    histogram and FROPS multiplicity) to /tmp/<shm_prefix>_RH_output.bin.
ziskemuasm -s -m -f --gen=2 --shm_prefix VERIF -p 23500 &
ziskemuasm -c -i <input> --gen=2 --shm_prefix VERIF -p 23500 --mt 1
ziskemuasm -c -i <input> --gen=2 --shm_prefix VERIF -p 23500 --shutdown

# 2. Take the operation trace of the same ELF and input (--stats is what enables the dump).
ziskemu -e <elf> -i <input> --stats --store-op-output ops.bin

# 3. Compare the two columns row by row.
frops-analyzer verify-asm --asm-dump /tmp/VERIF_RH_output.bin --trace ops.bin
```

It also checks that the instruction histogram sums to the step count. Exit status is non-zero on any
disagreement, so it can be dropped into a regression script.

## Choosing `--max-regions-per-op` (detection speed vs circuit area)

More regions per op = more FROPS coverage (smaller circuit area) but a longer membership test, and
that test runs on every executed arith / binary operation in the ROM-histogram assembly. It is the
**speed ↔ area knob**. Each counting thunk `zisk_core::frops_asm` emits is annotated with its
instruction count and its worst case to reject a non-FROP.

Measured over 103 mainnet blocks (8.3 G steps, 4.33 G candidate operations; `--max-table 25165824
--partition-bits 21 --low-cap 4096 --table-cost 3`), regenerating the tables for each cap and timing
the ROM-histogram assembly itself on a Ryzen 9 9950X3D:

| cap | table rows | coverage | area (no padding) | RH pass | over no FROPS |
|-----|-----------:|---------:|------------------:|--------:|--------------:|
| no FROPS at all       |         0 |      0% | 204.30 B |  20.42 s |      — |
| in-tree tables (other traces) | 24.26 M | 33.47% | 123.17 B | 24.77 s | +21.3% |
| **1**                 |   19.72 M |  37.60% | 117.71 B |  24.28 s | +18.9% |
| **2**                 |   21.98 M |  40.87% | 112.94 B |  25.26 s | +23.7% |
| **4**                 |   23.74 M |  44.30% | 108.78 B |  26.20 s | +28.3% |
| 8                     |   24.37 M |  44.66% | 108.17 B |  27.02 s | +32.3% |

Two things to take from it:

* **Cap 8 is not worth it**: +0.36 coverage points over cap 4 for +0.8 s of sequential pass. Cap 16
  (the old default) is worse still — `--max-table` caps the table before the extra regions pay off.
* **The knob is a poor lever on the assembly cost.** Dropping from cap 4 to cap 1 removes only a
  third of the counting time (−1.9 s of 5.8 s) and costs 6.7 coverage points and 8.2% more area,
  because most of the cost is the first box of every op, not the later ones. If the ROM-histogram
  pass is on the critical path, this knob will not get it off — see the note below.

Which cap to use depends on what is scarce. When the sequential ROM-histogram pass gates the
pipeline, a lower cap buys wall clock; when it does not — a 100-200 ms delay is nothing against a
whole proof — take the coverage and the area. The measurements above are for one workload and one
machine: rerun them for yours rather than trusting the ranking.

### Whether the ROM-histogram pass is on the critical path

The three sequential assemblies (MT, MO, RH) run in parallel and the pipeline only waits for RH long
after MT, so counting FROPS there is free while RH has slack. On the workload above it has almost
none: over the same 103 blocks MT totals 21.3 s and MO 20.7 s against RH's 20.42 s, a median margin
of about 9 ms per block. Any of the caps above spends more than that, so RH becomes the critical path
and the pass' extra time turns into wall clock. Check the `WAIT_ASM_RH` timer of a real run before
assuming otherwise.

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
