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
| Steps | 9,533 | 2,631 | 3.6× |
| Variable cost | 979,735 | 269,966 | 3.6× |
| ROM size (instructions) | 100,521 | 47,881 | 2.1× |

**Per unit of work** (the slope between `n = 0` and `n = N`):

| Workload | Unit | wasm steps | RISC-V steps | ratio | wasm cost | RISC-V cost | ratio |
|---|---|---:|---:|---:|---:|---:|---:|
| fib | loop iteration | 3 | 5 | 0.5× | 208 | 397 | 0.5× |
| calls | 1,023 calls | 18,431 | 14,845 | 1.2× | 1,465,490 | 1,211,724 | 1.2× |
| sort | element sorted | 317 | 126 | 2.5× | 36,394 | 18,410 | 2.0× |
| btree | insert + lookup | 1,011 | 489 | 2.1× | 92,430 | 39,845 | 2.3× |
| json | JSON record | 21,138 | 6,352 | 3.3× | 2,049,873 | 523,855 | 3.9× |
| float | `f64` step | 5,546 | 506 | 11.0× | 517,276 | 48,998 | 10.6× |
| keccak_sw | Keccak-f in Rust | 18,754 | 4,056 | 4.6× | 1,953,656 | 438,078 | 4.5× |
| bigint | 256 × 256-bit product | 399 | 118 | 3.4× | 36,487 | 13,338 | 2.7× |
| memcpy | 4 KiB copy | 56 | 19 | 2.9× | 40,186 | 35,722 | 1.1× |
| keccak_pc | Keccak-f precompile | 33 | 7 | 5.1× | 44,884 | 42,774 | 1.0× |

**Proving** (the `json` workload, emulator backend, measured before the transpiler changes,
which take 58% of its steps away, so these ratios are now lower; instance counts from the prover's plan, proving time on
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

- **About 0.5–1.2× for register-bound code** (`fib`, `calls`). A function's most used locals
  live in registers, so a short chain of operations on a few of them, like `fib`'s loop, beats
  the RISC-V build; every call still writes its arguments to memory, saves those registers and
  checks the stack. A short chain of operations on a few locals, like `fib`'s loop, is as
  cheap as RISC-V's.
- **About 2–4× for typical code** (`sort`, `btree`, `json`): on top of the slots, every
  linear-memory access is bounds-checked (one compare, less when the same pointer local was
  checked just before), which shows as extra `leu`/`and` in the opcode breakdown
  (`ziskemu -X`) and as memory cost.
- **3–11× for arithmetic that keeps many values live, and for floating point:**
  - `keccak_sw`: 25 lanes and their temporaries are more than the 17 registers locals get, and
    the state lives in linear memory, so many XOR, AND-NOT and rotation operations load their
    operands or store their result, with bounds checks; RISC-V keeps more in registers and has
    `andn`.
  - `bigint` (now 2.7×): wasm32 has no 64 × 64 → 128-bit multiply, so each product is a
    `__multi3` call, which the transpiler lowers inline to `mul` + `muluh`; but its result still
    goes through memory (the call's `ret` pointer) and is loaded back, with bounds checks, where
    RISC-V keeps both halves in registers.
  - `float`: RISC-V uses the compiler's software floating point inline; wasm traps each `f64`
    operation into the float handler, which saves and restores the registers and decodes a
    RISC-V instruction before running the soft-float routine.
- **About 1× when a precompile does the work** (`keccak_pc`, and `memcpy` on the DMA): the
  precompile cost is identical and dominates.
- **Fixed overhead about 3.5×**: Rust's `std` startup and WASI I/O on wasm, against `ziskos`
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
| Operands read from wherever they are (a local, a constant, the last result in the accumulator), compares fused with `br_if`/`if` | json | 8.4× | 5.6× |
| Bounds checks in one compare, and addresses reused across accesses through the same local | keccak_sw | 8.6× | 6.4× |
| `br_table` through a jump table, not a chain of compares | json | 4.9× | 4.8× |
| Arguments written straight to the callee, one-instruction returns, `x1` kept by leaf functions, results left in the return register | calls | 1.7× | 1.3× |
| The most used locals of each function in registers | keccak_sw | 6.4× | 4.5× |
| `__multi3` inlined at its direct calls, and the wide-arithmetic operators reading their operands in place | bigint | 6.6× | 2.9× |

`sort`, `btree` and `json` gained 1–2% from the DMA copies. The operand change applies to all
code; in steps per unit it saved 50% on `fib`, 41–49% on `calls`, `sort`, `btree` and
`keccak_sw`, 36% on `json`, 27% on `bigint` and 37% of the fixed overhead, nothing on `float`
(the float handler dominates it), and it made the ROM 29% smaller. The bounds-check change
saved 28% on `keccak_sw` and `memcpy`, 12–13% on `json`, `sort` and `bigint`, and 7% on
`btree`. The call change saved 28% on `keccak_pc` (a call to the precompile's import), 22% on
`calls`, 19% on `bigint` (its `__multi3` calls), 9% on `json` and 11% of the fixed overhead.
Locals in registers saved 33–36% on `sort`, `btree` and `keccak_sw`, 28% on `fib`, 17% on
`memcpy`, 14% on `json`, 6% on `calls`, 2% on `bigint` and 11% of the fixed overhead.
Inlining `__multi3` saved 58% on `bigint` and changed nothing else. Taking a comparison widened
to `i64` (the carries of `bigint`) as the 0 or 1 it already is, without masking it, saved another
6% there.

Support for the wide-arithmetic proposal came with the third change. A guest built with
`RUSTFLAGS="-C target-feature=+wide-arithmetic"` (an unstable flag; stable Rust accepts it with a
warning) multiplies inline instead of calling `__multi3`, which brings `bigint` to 241 steps and
22,037 cost per product: 1.7× RISC-V, against 2.7× through `__multi3`.
