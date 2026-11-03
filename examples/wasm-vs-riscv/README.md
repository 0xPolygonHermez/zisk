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

ZisK 1.4.0-alpha (commit `14e1f6031`), wasm guest built with Rust 1.97.1, RISC-V guest with the
ZisK toolchain (Rust 1.99-dev, the only one with the RISC-V target; the ZisK toolchain has no
wasm target), both `opt-level = 3`, `panic = "abort"`.

**Fixed overhead** (`n = 0`: startup, reading the input, writing the output):

| | wasm | RISC-V | ratio |
|---|---:|---:|---:|
| Steps | 20,809 | 2,631 | 7.9× |
| Variable cost | 1,879,662 | 269,966 | 7.0× |
| ROM size (instructions) | 188,258 | 47,881 | 3.9× |

**Per unit of work** (the slope between `n = 0` and `n = N`):

| Workload | Unit | wasm steps | RISC-V steps | ratio | wasm cost | RISC-V cost | ratio |
|---|---|---:|---:|---:|---:|---:|---:|
| fib | loop iteration | 8 | 5 | 1.6× | 651 | 397 | 1.6× |
| calls | 1,023 calls | 42,723 | 14,845 | 2.9× | 3,456,827 | 1,211,724 | 2.9× |
| sort | element sorted | 987 | 126 | 7.8× | 90,630 | 18,410 | 4.9× |
| btree | insert + lookup | 3,328 | 489 | 6.8× | 277,000 | 39,845 | 7.0× |
| json | JSON record | 50,943 | 6,352 | 8.0× | 4,449,313 | 523,855 | 8.5× |
| float | `f64` step | 5,572 | 506 | 11.0× | 519,331 | 48,998 | 10.6× |
| keccak_sw | Keccak-f in Rust | 75,817 | 4,056 | 18.7× | 6,628,529 | 438,078 | 15.1× |
| bigint | 256 × 256-bit product | 3,654 | 118 | 30.9× | 308,363 | 13,338 | 23.1× |
| memcpy | 4 KiB copy | 3,732 | 19 | 195× | 382,294 | 35,722 | 10.7× |
| keccak_pc | Keccak-f precompile | 46 | 7 | 7.0× | 45,802 | 42,774 | 1.1× |

**Proving** (the `json` workload, emulator backend; instance counts from the prover's plan,
proving time on one 128-core machine, which is noisy):

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

- **About 1.5–3× for register-bound code** (`fib`, `calls`). A wasm value lives in a memory
  slot of the frame, not a register, so most operations are a load, the operation and a store,
  and every call sets up a frame.
- **About 5–8× for typical code** (`sort`, `btree`, `json`): on top of the slots, every
  linear-memory access is bounds-checked, which shows as extra `leu`/`and` in the opcode
  breakdown (`ziskemu -X`) and as memory cost.
- **10–30× where RISC-V uses an instruction wasm does not have:**
  - `keccak_sw`: the RISC-V target has bit-manipulation instructions (`ror`, `andn`) that map to
    single ZisK operations; the transpiler lowers a 64-bit rotation to `sll` + `srl` + `or`.
  - `bigint`: wasm has no 64 × 64 → 128-bit multiply, so LLVM calls `__multi3`, four 32-bit
    multiplies plus carry handling; RISC-V uses `mul` + `mulhu`.
  - `memcpy`: on RISC-V `copy_from_slice` becomes the DMA `memcpy` precompile (19 steps for
    4 KiB); the transpiler lowers `memory.copy` to a copy loop. The cost ratio (11×) is far below
    the step ratio (195×) because the precompile's own cost dominates the RISC-V side.
  - `float`: RISC-V uses the compiler's software floating point inline; wasm traps each `f64`
    operation into the float handler, which saves and restores the registers and decodes a
    RISC-V instruction before running the soft-float routine.
- **About 1× when a precompile does the work** (`keccak_pc`): the precompile cost is identical
  and dominates.
- **Fixed overhead about 7–8×**: Rust's `std` startup and WASI I/O on wasm, against `ziskos`
  reading the input in place.

The 10–30× cases point at transpiler work with a large payoff: lowering rotations to `ror`,
`memory.copy`/`memory.fill` to the DMA precompiles, and recognising `__multi3` (or supporting
the wide-arithmetic proposal's `i64.mul_wide_u`).
