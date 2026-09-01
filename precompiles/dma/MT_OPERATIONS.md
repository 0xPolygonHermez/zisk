# Memory-in-Time DMA operations (`mtcpy` / `mtcmp`)

> **Status: emulation only, and parked.** The whole feature lives in the Rust
> emulator; there is no circuit, no state machine and no assembly-emulator
> support behind it. It was built to measure whether the idea pays for itself
> (see [Measured impact](#measured-impact)) and the answer, for the workload we
> measured, was no. It is documented and kept because the mechanism is sound and
> other uses may well justify finishing it.

## The idea

The DMA family described in [`README.md`](./README.md) accelerates `memcpy` and
`memcmp` over memory *as it is now*. The `mt` ("memory in time") family is the
same pair of operations, except that the **source** is read as it was at an
earlier point of the execution:

| Operation    | Opcode | Meaning                                            |
|--------------|--------|----------------------------------------------------|
| `dma_mtcpy`  | `0xd3` | `memcpy(dst, src@t, count)`, count in a register    |
| `dma_mtcmp`  | `0xd4` | `memcmp(dst, src@t, count)`, count in a register    |
| `dma_xmtcpy` | `0xd5` | same as `dma_mtcpy`, count as an immediate          |
| `dma_xmtcmp` | `0xd8` | same as `dma_mtcmp`, count as an immediate          |

`t` is a *temporal reference*: an opaque handle the guest obtains at the moment
it wants to freeze, and passes back later. The destination side is always read
and written live; only the source travels in time.

### What it buys

It lets a guest read data it has already thrown away. The motivating shape is a
memo or cache, or any buffer that gets overwritten in place:

```rust
// Without mt: the guest must keep its own copy of the entry before clobbering
// it, and that copy is a real, proven memcpy plus the memory to hold it.
let saved = entry.to_vec();
refill(&mut entry);
if saved == other { ... }

// With mt: nothing is copied inside the proof. The guest only says "I may want
// to read this back", and pays for the read only on the paths that take it.
let t = ziskos_temporal_snapshot!(&entry, LEN);
refill(&mut entry);
if ziskos_mtcmp!(&other, &entry, LEN, t) == 0 { ... }
```

The saving is the eager copy that never gets used, on every path that does not
need it. The copy still happens — but outside the proof, in the emulator's
snapshot store, where it costs host memory instead of steps.

## How it works

### 1. Temporal references come from `step`

A temporal reference is just the `step` at which it was requested. The `flag`
operation, which used to leave `0` in `c`, now leaves the current `step` there
([`core/src/ops_core_context.rs`](../../core/src/ops_core_context.rs)). Every
existing user of `flag` — `nop`, hint `addi`, `jal` — either discards `c` or
stores the pc instead, so the change is transparent to them.

The guest requests one with a dedicated CSR read, `csrrs rd, 0x81F, x0`, which
the transpiler lowers to a single `flag` that stores `c` into `rd`. That `flag`
carries a distinctive tag in `b` (`TEMPORAL_REF_REQUEST_TAG`), which is how the
emulator tells a *request* apart from any other `flag`. The tag is out of reach
of the 12-bit immediate that feeds `b` on a hint `addi`, so nothing else can be
mistaken for a request.

### 2. The executor cannot keep a memory history, so the guest advises it

Retaining the full history of memory would be prohibitively expensive, so the
guest has to announce in advance which regions it may want to read back. That is
the `execute_advice` hint (opcode `0xc1`, `OpType::Internal`, zero cost): it
copies a memory region and tags the copy with the temporal reference most
recently requested.

It has no CSR of its own. A bare `addi x0, reg(address), count` is
indistinguishable from any other hint `addi`, so the pattern is delimited by two
marker `addi`s:

```text
 pc:    addi x0, x0, ID                ===>  execute_advice x0, reg(address), count ─┐
 pc+4:  addi x0, reg(address), count         addi x0, reg(address), count            │ jmp
 pc+8:  addi x0, x0, ID                      addi x0, x0, ID                         │ pc+12
 pc+12: ..........                           ..........   <──────────────────────────┘
```

A register count is accepted too (`add x0, reg(address), reg(count)` in the
middle), which is the only way past the 12-bit reach of an immediate.

There is also a **fused** form, `execute_advice_ref` (opcode `0xc2`, CSR
`0x820`), which opens a temporal reference *and* advises one region under it in a
single instruction, handing the reference back in `rd`. That is the common case —
one reference, one region — and it costs one step where the request-plus-advice
pair costs two:

```text
 i:  csrrs rd, 0x820, reg(addr)  ===>  execute_advice_ref rd, reg(addr), count ─┐
 n0: add[i] x0, reg(addr), count       add[i] x0, reg(addr), count              │ jmp
 n1: ..........                        ..........   <───────────────────────────┘
```

The fused form leaves the reference open, so plain `execute_advice`s can keep
adding regions to it; the two-step form remains the only way to put several
regions under one reference from the start.

### 3. The snapshot store

[`core/src/mem_snapshot.rs`](../../core/src/mem_snapshot.rs) holds the copies,
indexed by temporal reference in a hash map, with a `VecDeque` recording creation
order for eviction. Lookup is by hash rather than by scanning because the cap is
large and both `capture` and `read` run on the hot path.

`MEM_SNAPSHOT_GENERATIONS` is `1 << 20`. That is sized for a **long-range memo**
rather than for short-range buffer reuse: the guest's Keccak-f cache files one
reference for the input and one for the output of every distinct permutation, and
a hit can land on any of them however long ago it was created, so a small window
makes the memo useless. It is only a cap — a generation is created when something
is captured under it, so the store costs nothing until it is used. A block that
runs 218k permutations holds roughly 87 MB of snapshots, outside the proof, in
exchange for 2×200 bytes of *proven* copying per permutation.

Captured ranges are widened outwards to their 64-bit envelope, because the `mt`
operations read their source as whole words. A read that no live generation
covers — a missing advice, an evicted reference, or an advised range narrower
than the one being read — is always a guest bug, and aborts with a message naming
the regions that *are* available.

### 4. Calling convention of the `mt` operations

They take one parameter more than their `mem` counterparts. It travels in the
second extra-parameter slot, `EXTRA_PARAMS + 8`
(`EXTRA_PARAMS_TEMPORAL_REF_ADDR`), which is why the pattern has one instruction
more:

```text
 i:  csrrs rd, 0x81D/E, reg(src)  ===>  sd reg(count), [EXTRA_PARAMS]     ────┐
                                        sd reg(tref), [EXTRA_PARAMS + 8]  ────┤ internal
                                        mtcxx rd, reg(dst), reg(src)      ────┤
 n0: add  x0, reg(dst), reg(count)      add  x0, reg(dst), reg(count)         │ jmp
 n1: add  x0, reg(tref), x0             add  x0, reg(tref), x0                │ next[2]
 n2: ..........                         ..........   <────────────────────────┘
```

With an `addi` in `n0` the count is an immediate and travels in the extended
argument, which selects the `x` variants and drops the first store.

CSR assignments: `0x81D` `mtcpy`, `0x81E` `mtcmp`, `0x81F` temporal reference
request, `0x820` fused reference-and-advice.

### 5. Guest API

Unlike the `mem` family, the `mt` family is **not** transparent: there are no
standard C functions with these semantics, so guests call the macros in
[`ziskos/entrypoint/src/dma.rs`](../../ziskos/entrypoint/src/dma.rs) explicitly.

| Macro                        | Purpose                                                           |
|------------------------------|-------------------------------------------------------------------|
| `ziskos_temporal_snapshot!`  | Request a reference and capture a region, in one indivisible block |
| `ziskos_temporal_ref!`       | Request a reference on its own                                     |
| `ziskos_execute_advice!`     | Capture a region at the reference most recently requested           |
| `ziskos_mtcpy!`              | `memcpy` whose source is read at a reference                       |
| `ziskos_mtcmp!`              | `memcmp` whose second operand is read at a reference               |

`ziskos_temporal_snapshot!` is the form to reach for. A request and an advice
issued separately can be pulled apart by the compiler — any intervening call is a
`jal`, i.e. another `flag` — and a reference that has drifted away from the region
it was meant to bind is the one way of getting these operations wrong. The
single-block macro cannot drift. Advising several regions for one reference still
works: chain `ziskos_execute_advice!` calls right after the snapshot.

> `ziskos_temporal_snapshot!` currently emits the four-instruction two-step form
> (CSR `0x81F` plus the marker triple). The fused `0x820` path is implemented down
> to the transpiler but has no guest macro yet, so switching the macro over is the
> one-line change that would claim its saved step.

## Measured impact

The reason this stops at emulation. Applied to the Keccak cache of the
`zisk-eth-one` client — the case the design was aimed at — it cut **1.7% of the
steps**.

That is real but small, and the cost is not where one might expect:

- **Proving is not the problem.** The `mt` operations decompose into the same
  aligned/unaligned/head-tail work the existing DMA circuits already prove, and
  the temporal reference is one extra memory read. A circuit for them would look
  much like the `mem` one.
- **Execution and witness computation are.** The snapshot store has to be carried
  through emulation, kept bounded, and reproduced identically in every mode —
  including the assembly emulator, where `flag`'s `c` is currently constant-folded
  to `0` and folding it to `step` would give up real optimizations. In
  `ConsumeMemReads` the source words come back from the minimal trace rather than
  from memory, so the two paths have to be kept in agreement by construction. And
  the store trades proof steps for host memory: tens of MB per block for the
  Keccak memo, which is witness-computation budget, not proving budget.

1.7% does not pay for that. What keeps the idea alive is that the mechanism is
general: any guest pattern that eagerly copies data on a path that usually does
not need it can be rewritten this way, and a workload where such copies are a
larger share of the steps would justify finishing the implementation. The
groundwork — the ABI, the transpilation, the snapshot store, the emulator
semantics — is done and tested; what is missing is the proving side and the
assembly emulator.

## What exists, and what does not

| Piece                                                        | State                          |
|--------------------------------------------------------------|--------------------------------|
| ABI constants, CSRs, opcodes                                 | done                           |
| Transpilation of all four patterns                           | done                           |
| Guest macros                                                 | done, except the fused `0x820` |
| Rust emulator (`Mem`, `GenerateMemReads`, `ConsumeMemReads`)  | done                           |
| Cost/stats accounting (`ops_dma_mt*`)                        | done, mirrors the `mem` family |
| Assembly emulator                                            | **panics**, points at ziskemu  |
| DMA bus device / state machines / PIL                        | **panics**, emulator-only      |

Both refusals are explicit arms, not accidental fallthroughs:
[`core/src/zisk_rom_2_asm.rs`](../../core/src/zisk_rom_2_asm.rs) and
[`src/dma_bus_device.rs`](./src/dma_bus_device.rs).

## Code and tests

| Area              | Files                                                                    |
|-------------------|--------------------------------------------------------------------------|
| ABI constants     | [`definitions/src/syscall.rs`](../../definitions/src/syscall.rs)         |
| Snapshot store    | [`core/src/mem_snapshot.rs`](../../core/src/mem_snapshot.rs), plus the `*_snapshot` methods in [`core/src/mem.rs`](../../core/src/mem.rs) |
| Operations        | [`core/src/operations/dma_mtcpy.rs`](../../core/src/operations/dma_mtcpy.rs), [`dma_mtcmp.rs`](../../core/src/operations/dma_mtcmp.rs), [`execute_advice.rs`](../../core/src/operations/execute_advice.rs) |
| `flag` semantics  | [`core/src/ops_core_context.rs`](../../core/src/ops_core_context.rs)     |
| Transpilation     | [`transpilers/riscv/src/riscv2zisk_context.rs`](../../transpilers/riscv/src/riscv2zisk_context.rs) |
| Guest macros      | [`ziskos/entrypoint/src/dma.rs`](../../ziskos/entrypoint/src/dma.rs)     |

```bash
# operation semantics, through InstContext
cargo test -p zisk-core operations::tests::mt_tests

# RISC-V -> ZisK lowering of the patterns
cargo test -p zisk-riscv --test mt_transpile

# end-to-end: a guest that asserts every result itself, run under the emulator
cargo test -p zisk-precomp-dma dma_mt_tests
```

The guest lives in
[`test-artifacts/programs/syscalls/dma_mt/`](../../test-artifacts/programs/syscalls/dma_mt).
To see the opcodes actually firing:

```bash
ziskemu -e <dma_mt.elf> -X | grep mt
```
