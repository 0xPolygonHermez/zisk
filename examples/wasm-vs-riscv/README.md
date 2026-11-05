# wasm vs RISC-V in ZisK

The same Rust code, built as a `wasm32-wasip1` guest and as a ZisK RISC-V guest, measured on the
same inputs. Everything here is deterministic emulator output (steps, cost, instances), so the
numbers do not depend on the machine.

```text
wasm-vs-riscv/
├── workloads/     # the code being measured, shared by both guests
├── wasm-guest/    # plain wasm32-wasip1 program (stdin → stdout)
├── riscv-guest/   # ZisK RISC-V program (ziskos)
└── compare.py     # runs both with `ziskemu -X` and prints the tables below
```

Each workload does `n` units of one kind of work and returns a checksum. Both guests read the
workload id and `n` from the input, and publish the same text, which `compare.py` checks on
every run. The precompile workload calls the Keccak-f precompile through `ziskos` on RISC-V and
through the `env::keccak_f` import on wasm; both become the same ZisK instruction.

## Running it

```bash
(cd wasm-guest && cargo build --release --target wasm32-wasip1)
(cd riscv-guest && cargo-zisk build --release)
./compare.py --ziskemu <ziskemu built with the float feature>
```

The wasm guest needs a `float` build of ZisK (the `float` and `json` workloads use `f64`, and
they are in the same binary), see [the float limitation](../../transpilers/wasm/README.md#current-limitations).
The RISC-V numbers are the same with either build (steps identical, cost within 0.01%).

## Results

ZisK 1.4.0-alpha, with the transpiler changes listed in
[Transpiler changes](#transpiler-changes) (which also gives the numbers before them); wasm guest built with Rust 1.97.1, RISC-V guest with the
ZisK toolchain (Rust 1.99-dev, the only one with the RISC-V target; the ZisK toolchain has no
wasm target), both `opt-level = 3`, `panic = "abort"`.

**Fixed overhead** (`n = 0`: startup, reading the input, writing the output):

| | wasm | RISC-V | ratio |
|---|---:|---:|---:|
| Steps | 10,689 | 2,631 | 4.1× |
| Variable cost | 1,077,291 | 269,966 | 4.0× |
| ROM size (instructions) | 106,889 | 47,881 | 2.2× |

**Per unit of work** (the slope between `n = 0` and `n = N`):

| Workload | Unit | wasm steps | RISC-V steps | ratio | wasm cost | RISC-V cost | ratio |
|---|---|---:|---:|---:|---:|---:|---:|
| fib | loop iteration | 4 | 5 | 0.7× | 284 | 397 | 0.7× |
| calls | 1,023 calls | 19,698 | 14,845 | 1.3× | 1,621,626 | 1,211,724 | 1.3× |
| sort | element sorted | 495 | 126 | 3.9× | 50,739 | 18,410 | 2.8× |
| btree | insert + lookup | 1,508 | 489 | 3.1× | 130,869 | 39,845 | 3.3× |
| json | JSON record | 24,676 | 6,352 | 3.9× | 2,339,364 | 523,855 | 4.5× |
| float | `f64` step | 5,548 | 506 | 11.0× | 517,443 | 48,998 | 10.6× |
| keccak_sw | Keccak-f in Rust | 29,249 | 4,056 | 7.2× | 2,791,573 | 438,078 | 6.4× |
| bigint | 256 × 256-bit product | 1,022 | 118 | 8.7× | 90,399 | 13,338 | 6.8× |
| memcpy | 4 KiB copy | 67 | 19 | 3.5× | 41,151 | 35,722 | 1.2× |
| keccak_pc | Keccak-f precompile | 33 | 7 | 5.0× | 44,845 | 42,774 | 1.0× |

**Proving** (the `json` workload, emulator backend, measured before the transpiler changes,
which take 51% of its steps away, so these ratios are now lower; instance counts from the prover's plan, proving time on
one 128-core machine, which is noisy):

| `n = 2,000` | wasm | RISC-V | ratio |
|---|---:|---:|---:|
| Steps | 104,843,055 | 12,750,990 | 8.2× |
| Main instances | 7 | 1 | 7× |
| Total instances | 20 | 14 | 1.4× |
| Proof generation | 480 s | 237 s | 2.0× |
| Peak memory | 47 GB | 46 GB | 1.0× |

| `n = 6,000` (plan only) | wasm | RISC-V | ratio |
|---|---:|---:|---:|
| Steps | 314,429,524 | 38,121,735 | 8.2× |
| Main instances | 19 | 3 | 6.3× |
| Total instances | 42 | 16 | 2.6× |

Proving time grows much slower than steps at this size: every program pays for a base set of
instances (ROM, tables, one of each state machine it touches) and for the final recursion,
whatever its length. RISC-V even uses more kinds of state machine here (the four DMA ones,
for its `memcpy`). The ratio rises towards the cost ratio as programs grow and the main and
memory instances dominate.

## Reading the results

Cost is what proving pays for, so it is the column to compare; steps alone understate the gap
where wasm moves more memory per step and overstate it where one RISC-V step is a precompile.

- **About 0.7–1.3× for register-bound code** (`fib`, `calls`). Wasm locals live in memory
  slots of the frame, not registers, so operations load their operands from memory, and every
  call writes its arguments to memory and checks the stack. A short chain of operations on a few locals, like `fib`'s loop, is as
  cheap as RISC-V's.
- **About 3–4.5× for typical code** (`sort`, `btree`, `json`): on top of the slots, every
  linear-memory access is bounds-checked (one compare, less when the same pointer local was
  checked just before), which shows as extra `leu`/`and` in the opcode breakdown
  (`ziskemu -X`) and as memory cost.
- **6–11× for arithmetic that keeps many values live, and for floating point:**
  - `keccak_sw`: 25 lanes and their temporaries all live in slots and linear memory, so nearly
    every XOR, AND-NOT and rotation loads its operands and stores its result, with bounds
    checks; RISC-V keeps them in registers and has `andn`.
  - `bigint`: each 64 × 64 → 128-bit product is a call to `__multi3` (wasm32 has no such
    instruction), with its frame and its result written to memory; RISC-V uses `mul` + `mulhu`
    inline.
  - `float`: RISC-V uses the compiler's software floating point inline; wasm traps each `f64`
    operation into the float handler, which saves and restores the registers and decodes a
    RISC-V instruction before running the soft-float routine.
- **About 1× when a precompile does the work** (`keccak_pc`, and `memcpy` on the DMA): the
  precompile cost is identical and dominates.
- **Fixed overhead about 4×**: Rust's `std` startup and WASI I/O on wasm, against `ziskos`
  reading the input in place.

## Transpiler changes

This comparison found three places where the transpiler did not use a ZisK operation RISC-V
guests get, and then showed that most steps were moves between frame slots. The changes made
since changed the per-unit cost (wasm / RISC-V) as follows, each row measured on top of the ones
above it:

| Change | Workload | Before | After |
|---|---|---:|---:|
| `i32`/`i64` rotations on ZisK's `rol`/`ror`, not two shifts and an `or` | keccak_sw | 15.1× | 14.3× |
| `memory.copy` / `memory.fill` on the DMA `memcpy` / `memset` precompiles, not word loops | memcpy | 10.7× | 1.3× |
| `__multi3` replaced by a body on 64-bit products (`i64.mul_wide_u`) | bigint | 23.1× | 12.5× |
| Operands read from wherever they are (a local, a constant, the last result in `x11`), compares fused with `br_if`/`if` | json | 8.4× | 5.6× |
| Bounds checks in one compare, and addresses reused across accesses through the same local | keccak_sw | 8.6× | 6.4× |
| `br_table` through a jump table, not a chain of compares | json | 4.9× | 4.8× |
| Arguments written straight to the callee, one-instruction returns, `x1` kept by leaf functions, results left in the return register | calls | 1.7× | 1.3× |

`sort`, `btree` and `json` gained 1–2% from the DMA copies. The operand change applies to all
code; in steps per unit it saved 50% on `fib`, 41–49% on `calls`, `sort`, `btree` and
`keccak_sw`, 36% on `json`, 27% on `bigint` and 37% of the fixed overhead, nothing on `float`
(the float handler dominates it), and it made the ROM 29% smaller. The bounds-check change
saved 28% on `keccak_sw` and `memcpy`, 12–13% on `json`, `sort` and `bigint`, and 7% on
`btree`. The call change saved 28% on `keccak_pc` (a call to the precompile's import), 22% on
`calls`, 19% on `bigint` (its `__multi3` calls), 9% on `json` and 11% of the fixed overhead.

Support for the wide-arithmetic proposal came with the third change. A guest built with
`RUSTFLAGS="-C target-feature=+wide-arithmetic"` (an unstable flag; stable Rust accepts it with a
warning) multiplies inline instead of calling `__multi3`, which brings `bigint` to 441 steps and
38,645 cost per product: 2.9× RISC-V, against 6.8× through `__multi3`.
