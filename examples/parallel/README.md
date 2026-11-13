# Parallel Example

Three guest programs split their work into tasks that run as fork-join regions (`ziskos::par`). A region runs its tasks as if one after another, which is what the proof attests, while the assembly emulator runs them at the same time. Each guest shows one pattern.

## Overview

| Guest | What it computes | What it shows |
| --- | --- | --- |
| `pi-guest` | π from the cells of an `r × r` grid inside a quarter circle | `par::run`: one closure per task, results back in task order |
| `histogram-guest` | a histogram of pseudo-random values | a reduction, the pattern of CUDA's reduction samples without shared memory: each task reduces its slice to a histogram of its own and returns it, and the histograms are added up after the region. A commented-out wrong version writes into a shared buffer instead |
| `sort-guest` | a sort of pseudo-random values: each task sorts a slice, then each task gathers one range of values from every sorted slice and sorts it; then a checksum and a histogram of the result | two regions in sequence, both as wide as the tasks; arenas sized from the data before each region; and `par::join` for two closures whose results have different types |

Every guest reads the same input, `Params { len, tasks, seed }`, and the host checks what it commits against the same functions run natively.

## Structure

```text
parallel/
├── common/             # Params, the functions the tasks run, and the reference results
├── guest/
│   ├── pi.rs
│   ├── histogram.rs
│   └── sort.rs
└── host/
    └── main.rs         # Executes the three guests and checks their output
```

## Host

Run from the `host/` directory, with `--release`: the guests build with the host's profile, and the stacks of their tasks are sized for release builds (see *Arenas* below). The host only executes the guests, so it needs no proving key.

```bash
# Rust emulator: the tasks run one after another
cargo run --release

# Assembly emulator: the tasks of each region run at the same time
cargo run --release -- --asm

# The baseline: the same programs and the same steps, with each region's tasks run one after another
ZISK_ASM_PAR_WORKERS=1 cargo run --release -- --asm

# Tasks per region (8 by default) and executions per guest (3 by default)
cargo run --release -- --asm --tasks 64 --runs 5
```

For each guest the host prints the steps, the execution time (emulation, counting and memory plan) and, on the assembly emulator, the emulation time alone: the medians of its runs. It checks the output of every run.

### Comparing

Compare `--asm` with the baseline, `ZISK_ASM_PAR_WORKERS=1` and `--asm`: the programs, their steps and their output are the same, and only how the assembly emulator runs each region changes.

`--tasks 1` is not a baseline, because it changes what the guests do. With one task, `sort` sorts all the values in its first region, and its second region gathers that one sorted slice, which `sort_unstable` recognises as sorted in a single pass: a different sort from the one 8 tasks run, with other steps. Use `--tasks` to see what splitting costs instead: the driver pads every task to a chunk of its own (2^18 steps), so for the same work more tasks run more padding steps.

What to expect on the assembly emulator:

- **The emulation time drops, the execution time barely changes.** The stages that consume the emulator's chunks, counting each chunk with the Rust emulator and planning the memory, keep the pace they had with a sequential emulator: today they set the floor of the execution phase, so a faster emulation barely shortens it, and in a parallel run they share the cores with the tasks. See the *Performance* section of the book's *Parallel Execution* page.
- **Every region has a fixed cost.** The assembly emulator runs each task in a process of its own, and every task ends with padding to a chunk boundary. Prefer few regions, as wide as the cores, with tasks much longer than a chunk.

## Key concepts

- **Sequential semantics.** A region gives what running its tasks in order gives, on any emulator; the proof does not change.
- **A task returns what it computes.** A task reads anything built before its region, but not what another task of the region writes, and it writes only what it owns: its arena (its heap and its stack) and its result. It must never write into memory built before the region: not a shared counter, not its own disjoint `chunks_mut` slice of a shared buffer, not a `Vec` moved into its closure and changed in place. When the tasks run in parallel, each one writes into a private copy of the memory outside its arena, so those writes are lost: the guest goes on with what that memory held before the region, and the proof fails. Natively and on the Rust emulator, which run the tasks one after another, the same code gives the right result, so it can look correct. Instead, return what the task computes, and combine the results after the region, in task order, as `histogram.rs` does. That also replaces shared counters and atomics: a task sees memory as it was at the fork, not what the tasks before it wrote.
- **What a task may use.** Precompiles, stateless fcalls and floating point. Not host randomness or the Keccak-f cache, which carry state that the tasks before it would have advanced; not reading the input or committing outputs; and it must not end the program, since an exit or a panic inside a task fails the execution.
- **What the API checks.** `par::run` and `par::join` take closures and results that are `Send`, so a task that captures a `&Cell`, a `&RefCell` or an `Rc` does not compile. A `par::run` inside a task runs its jobs inline, one after another: regions do not nest, but they can follow one another, as in `sort`. Reading the input or committing outputs inside a task panics. Writes into memory built before the region are not caught: a `&mut` slice is `Send`.
- **Two closures of different types.** `par::run` takes closures of one type. `par::join` runs two closures as a region of two tasks and returns both results, which may have different types: `sort` joins a checksum (`u64`) with a histogram (`[u64; 16]`) of the sorted values.
- **Results own memory.** A result may own memory its task allocated (`sort`'s sorted slices): arenas are never reused, so it stays valid after the region, and a later region can read it.
- **Arenas.** Each task gets an arena of its own, sized by `par::Config` (16 MiB of heap and 256 KiB of stack by default): a heap for what the task allocates and a stack for its deepest call chain. Size them from the data before the region, as `sort` does:
  - The heap is a bump allocator: every allocation counts against it, and freeing gives nothing back. A `Vec` grown by `push` reallocates as it doubles and keeps its old buffers, several times its final size in all, so allocate once, with `Vec::with_capacity` or by collecting an iterator of known length. The stable `sort` allocates scratch space, up to as much again as the slice; `sort_unstable` allocates nothing. A heap too small fails the execution.
  - The stacks of these guests are sized for release builds. Guests build with the host's profile, so `cargo run` without `--release` builds debug guests, whose much larger stack frames can overflow them. A canary below each stack catches most overflows when the task returns, failing the execution; a single stack frame larger than the canary can step over it.
  - Arenas are never reused, so the arenas of every region a guest opens add up, within its 512 MiB of RAM. Arenas larger than needed also widen the memory that the CPU memory planner walks.
- **When a guest gets no parallelism.** The output and the proof stay the same, but the regions run one task after another, and nothing fails:
  - with an allocator other than the default bump allocator (one of the `zisk-embedded-*` or the `zisk-custom-alloc` features of `ziskos`), `par::run` runs the closures one after another, without a region;
  - an execution that takes hints, the precompile results the host computes ahead, runs every region one task after another, since a task would need its place in their stream (calling precompiles inside a task is fine);
  - a run runs at most 512 tasks apart, counted over all its regions: the regions past that budget run one task after another, with only a warning in the log;
  - `ZISK_ASM_PAR_WORKERS` set to 1 or less runs every region one task after another;
  - so do a region of fewer than two tasks, and the Rust emulator, always.

See the book's *Parallel Execution* page for how regions work and how to check a change.
