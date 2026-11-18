# Parallel Execution

> **Status: experimental.** Fork-join regions run in parallel in the assembly emulator's minimal-trace (MT) and memory-operations (MO) services. Everything else runs them one task after another, and the proof is the same either way.

Emulating the guest used to be the one serial stage of a ZisK proof: proving is split into chunks and instances, but the emulator ran the program on a single core.  Fork-join regions let a guest say which parts of its work are independent, so that the emulator can run them at the same time.

## Overview

### Sequential semantics

A region is a list of tasks `T1 … Tn`.  By definition they run in order, one after another, and that sequential execution is what gets proven.  The emulator may run them at the same time because, when they are independent, the result is the same.  This page says that the emulator runs a region *apart* when it runs each task in a process of its own, at the same time as the others, and *sequentially* when it runs them one after another.  Run apart, the tasks' traces are stitched in task order, with the global steps they would have had in the sequential execution:

```text
emulator      pre ─┬─ T1 ───────┐
                   ├─ T2 ────┐  │
                   └─ T3 ────┴──┴──────── post        all tasks start from the state at the fork

proof         pre ░│ T1      ░│ T2   ░│ T3         ░│ post
                   ↑ every pass starts on a chunk boundary (2^18 steps); ░ = padding
```

The first pass starts at the chunk boundary that follows the fork, and task `Tk`'s pass at that step plus the lengths of the passes of `T1 … Tk-1`, which are known once those tasks finish.  After the last task, one more pass runs no task and ends the region.

### What does not change

- **The proof.**  There is one trace, with global steps, and every task is aligned to the chunk grid.  The padding costs about 1% of the steps of a large block.  The region markers are free calls (fcalls) that only the emulators interpret; to the proof they are ordinary steps.  The PIL, the AIRs and proofman know nothing about regions.
- **Correctness when a region cannot run apart.**  The emulator then runs the region sequentially and produces the same proof (see [Fallbacks](#fallbacks)).  The Rust emulator always runs regions sequentially.

### Who guarantees independence

ZisK cannot tell which parts of a program are independent: the program decides, with whatever guarantee it has.  An Ethereum block can rely on its block-level access list (EIP-7928), an aggregation guest on each proof being verified on its own, and a tree hash on its subtrees sharing nothing.

If a program breaks the contract, some read in the stitched trace does not match the last write in sequential order.  The memory state machine then fails, and there is no proof.  A false proof is never produced.

## Writing a guest

### Rust: `ziskos::par::run`

`par::run` takes a list of closures and returns their results in the same order:

```rust
use ziskos::par;

// Built before the region: the tasks only read it
let shared = build_shared(&input);

// One task per part; each one writes only to its own arena
let jobs: Vec<_> = parts
    .iter()
    .map(|part| {
        let shared = &shared;
        move || process(part, shared)
    })
    .collect();

let config = par::Config { heap_bytes: 4 << 20, stack_bytes: 256 << 10 };
let results = par::run(jobs, &config); // in the order of `jobs`
```

Each task gets an **arena** of its own, carved out of the caller's heap before the region and page-aligned, so no two tasks share a page.  An arena holds, in order:

1. A slot with the task's closure and its result.
2. The task's heap.
3. A canary: 64 bytes of a known pattern.
4. The task's stack, which grows down towards the canary.

A task points the bump allocator at its own heap before it runs its closure; the caller's heap comes back after the region.  A result may own memory the task allocated (a `Vec`, a `Box`): arenas are never reused, so that memory stays valid after the region.

Natively, and on ZisK with an allocator other than the default bump allocator, the closures simply run one after another.

**Arena size.**  `Config::default()` gives each task a 16 MiB heap and a 256 KiB stack, and the guest RAM is 512 MiB, so a region with many tasks needs a smaller `Config`.  For example, 48 tasks with 16 MiB heaps do not fit, but with 4 MiB heaps they do.  Arenas are never reused, so the arenas of every region a program opens add up.  A task that allocates more than its heap panics, which fails the execution.  Nothing stops a task that runs deeper than its stack: it writes over the canary and then the top of its heap.  When the task returns, the runtime checks the canary and panics if it changed, so an overflow fails the execution instead of corrupting the task's data; but a single stack frame larger than the canary can step over it unnoticed.  Size the stack for the task's deepest call chain.

For two closures of different types, `par::join(a, b, &config)` runs them as a region of two tasks and returns both results.

Complete examples:
- `examples/parallel`: three guests, each showing one pattern.  `pi-guest` splits a computation into ranges (`par::run`, results in task order); `histogram-guest` is a reduction: each task reduces its slice to a histogram of its own instead of updating a shared counter; `sort-guest` runs two regions in sequence, the second reading the results of the first, sizes the arenas from the data, and ends with `par::join`.  Its host checks every output against the same functions run natively.
- `test-artifacts/programs/examples/fork_join`: tasks of different lengths in two consecutive regions, then a `par::join` of two closures with results of different types that read the regions' results, all checked in the guest against a sequential run.

### What a task may do

- Read any data built before the region.
- Write the memory it owns: its stack, its slot and its arena's heap.
- Write other memory only if nobody reads it afterwards without writing it first.  The runtime relies on this for the bump allocator's state: every task, and the caller after the region, sets it before using it.
- Call precompiles and stateless fcalls.
- Call `par::run` or `par::join`: inside a task they run their jobs inline, one after another, in that task.  Regions do not nest.
- Use floating point.  Every task starts with the floating-point control and status register (FCSR) the region started with, and its rounding-mode changes and flags end with it: after the region, the FCSR is what the caller left.

### What a task must not do

- Read what another task of the same region writes.
- Change in place data built before the region: a slice of a shared buffer (`chunks_mut`), or a `Vec` moved into its closure, whose memory is still the caller's.  Run apart, a task keeps only its writes to its own arena, so those writes are lost and the proof fails.  Return what the task computes instead.
- Read the input or commit outputs (`ziskos::io::read*`, `ziskos::io::commit*`), which panics inside a task, or use host randomness or the Keccak-f cache.  Each keeps state, in the guest's memory or in the host, that the tasks before it would have advanced.
- Open a region of its own with the raw ABI (`zkvm_parallel`): the run fails.  `par::run` and `par::join` run inline inside a task instead.
- End the program.  A guest that ends (it exits or panics) or faults inside a task fails the execution with an error that says so, even where its sequential execution would end normally.  A panic or a fault fails a sequential execution too; an exit does not.  `ZISK_ASM_PAR_WORKERS=1` runs the regions one task after another, which gives the sequential result.
- Depend on the step counter.  A task run apart counts its steps from the region's first step, as if it ran first.

Tasks cannot share mutable state, and atomics are no exception: a task sees memory as it was at the fork, not the writes of the tasks before it.  `par::run` and `par::join` require `Send` closures and results, so sharing through `Cell`, `RefCell` or `Rc` does not compile; slices of a shared buffer and moved data do, and the rules above are what keeps them out.  Accumulate per task instead, and combine the results after the region, in task order.  For example, give each task its own histogram and merge them after the region.

### C and other languages: the raw ABI

Below `par::run` there is one call that any language can use:

| Symbol | zkvmcall | Role |
|---|---|---|
| `zkvm_parallel(tasks, n, ret)` | `0x8B5` | Runs `n` tasks as one region |
| `zkvm_parallel_ret` | `0x8B6` | Where every task returns; pass it as `ret` and never call it |

C guests include `zkvm_parallel.h` (`ziskasm/lang/c/include`) and link `zisklib_c`, whose `src/zkvm_calls.s` holds the zkvmcall thunks.  The header declares the task and both calls:

```c
typedef struct zkvm_task {
    void (*entry)(const void *arg, void *out); // runs the task
    const void *arg;                           // its argument
    void *out;                                 // where it writes its result
    uint64_t stack_top;                        // 16-byte aligned, within mem_start..mem_end
    uint64_t mem_start;                        // the memory the task owns: page-aligned,
    uint64_t mem_end;                          // in the RAM, holding its stack and its out
} zkvm_task_t;                                 // six 64-bit words, 48 bytes

void zkvm_parallel(const zkvm_task_t *tasks, uint64_t n, void (*ret)(void));
void zkvm_parallel_ret(void);
static inline void zkvm_parallel_run(const zkvm_task_t *tasks, uint64_t n); // ret = zkvm_parallel_ret
```

Tasks return to `zkvm_parallel_ret` rather than into the library routine, because the assembly emulator resolves the dynamic jumps of guest code (a task's `ret`) only within the guest.  The raw ABI does not set up a heap per task: laying out each task's memory is the caller's job.  `ziskasm/lang/c/example/zkvm_parallel_guest.c` runs SAXPY over four tasks, each owning a page-aligned arena with its slice of the output and its stack.

## How it works

### Layers

| Layer | What it does | Where |
|---|---|---|
| Markers and driver | Five fcalls and the routine `ziskasm_zkvm_parallel`, which starts every pass in a canonical state and pads it to a chunk boundary | `ziskos/entrypoint/src/zisklib/fcalls/mod.rs`, `ziskasm/zisklib/zkvm/parallel.zisk`, `definitions/src/zkvmcall.rs` |
| Guest runtime | `par::run`, with one arena per task | `ziskos/entrypoint/src/par.rs`, `ziskos/entrypoint/src/zisklib/lib/zkvm_parallel.rs` |
| Rust emulator | Runs regions sequentially and checks the driver's invariant; the reference for the oracle | `core/src/fcall.rs` |
| Assembly emulator | One process per task in the MT and MO services, a trace object per task, one event per chunk.  The RH service runs regions sequentially | `emulator-asm/src/par.c`, `par_events.hpp`, `core/src/zisk_rom_2_asm.rs` |
| Readers | Follow the events and hand the chunks on in sequential order, with their final steps | `emulator-asm/asm-runner/src/par_events.rs`, `asm_mt_runner.rs`, `asm_mo_runner.rs` |
| Executor, memory planners, PIL, AIRs, proofman | Nothing: they get the chunks of a sequential execution | — |

### Markers

| fcall | Id | Where the driver issues it | Meaning |
|---|---|---|---|
| `FCALL_PAR_FORK` | 28 | Once, before the first pass; parameters `tasks`, `n` | A region starts |
| `FCALL_PAR_BOUNDARY` | 29 | First step of every pass, always on a chunk boundary | A pass starts |
| `FCALL_PAR_TASK_END` | 30 | After a task returns | A task ends |
| `FCALL_PAR_PAD` | 31 | Before the padding | Returns the steps left in the current chunk |
| `FCALL_PAR_JOIN` | 32 | In the last pass, which runs no task | The region ends |

`FCALL_PAR_PAD` is a free input that only decides how many steps the padding takes, never what the program computes.  Its value is `(CHUNK_SIZE - 1) - step % CHUNK_SIZE` (`par_pad_steps` in `core/src/fcall.rs`).  It is counted on the `2^18` grid from step 0, whatever chunk size an execution uses, so that every emulator returns the same value at the same step.

### The driver: every pass in the same state

The driver (`ziskasm/zisklib/zkvm/parallel.zisk`) is an ordinary sequential loop, with one pass per task and a last pass that leaves the loop.  Every pass starts at `FCALL_PAR_BOUNDARY`, on the first step of a chunk, in a canonical state: the same as the first pass except for `s11` (`x27`), the task index.  To get there:

- On entry, it pushes a 64-byte frame on the caller's stack with `ra`, `s7`–`s11` and the FCSR.  The loop then keeps its own state in callee-saved registers, which every task preserves: `s7` = `ret`, `s8` = the stack pointer below that frame, `s9` = `n`, `s10` = `tasks`, `s11` = the task index.
- Before every pass, the first included, it restores the FCSR the region started with.  The floating-point library reads the FCSR at every operation (its rounding mode) and rewrites its flags, and the FCSR lives in memory that no task owns: a task run apart would read it as the region started, where run in order it would read what the task before it left.
- Before the first pass and after every task, it zeroes what a task may leave anything in: `ra`, the temporaries, the arguments and the virtual registers.  The other callee-saved registers, `gp` and `tp` keep the caller's values, and `c` is the loop's address, left by the jump every pass arrives through.
- It pads to the next chunk boundary after the fork and after every task.  `FCALL_PAR_PAD` returns `r`, the steps left to the boundary.  The padding takes 9 fixed steps, one more step when the rest is odd, and a two-step loop for the remainder, so exactly `r` steps run.  When `r` is less than 11 (`ZPAR_PAD_MIN`), too few for the fixed steps and the loop, it runs `r + CHUNK_SIZE` steps instead and the pass starts one chunk later.

So the chunk that the first pass opens is, except for its step and `s11`, the first chunk of any task.  An emulator can start any task from that state.

### Rust emulator

The Rust emulator runs regions one task after another and, at every marker, checks the invariant that the parallel emulation relies on (`par_marker` in `core/src/fcall.rs`).  It checks it when it runs the guest itself, not when it replays the chunks of the assembly emulator to count them:

- Regions are not nested, and the fork has two parameters.
- Every pass starts on a chunk boundary, and there are no more passes than tasks plus one.
- The first pass starts with task index 0.  Every later pass starts with the same registers, `c` and `sp` as the first one, with `s11` equal to the pass number.
- The join comes in the last pass, after one pass per task.

If the guest breaks any of these, it panics, saying which register differed and at which step.

### Assembly emulator: one process per task

Fork-join runs apart in the services that write chunked traces: MT, and MO in both its heavy and light forms.  The RH service runs regions sequentially.  The generated assembly calls into the C runtime at `FCALL_PAR_FORK` and `FCALL_PAR_BOUNDARY` (`par_marker_call` in `core/src/zisk_rom_2_asm.rs`).

At a region's first pass, the service's process (the parent):

1. Copies the task list to its own memory and validates the copy.  The list must lie in the RAM, outside the memory of every task, and give each task a page-aligned range of the RAM that holds its stack, no two of them overlapping.  Every task's process is set up from that copy, so a task that runs while others wait for a worker cannot change what they run.  It also checks the run's budget: a service runs at most 512 tasks apart in one run, counting the tasks of all its regions, however many run at a time.  The reader keeps each task's trace object mapped for the rest of the run, over the whole trace range, 32 GiB of address space each.  A region that would take the run past 512 tasks runs sequentially.
2. Saves the prefix of the chunk the first pass opened: its header, and the reads (MT) or records (MO) the `BOUNDARY` step made.
3. Creates one sparse trace object per task, `<program>_<MT|MO>_r<region>_t<task>`, and publishes the region (`REGION`).  `<program>` is the prefix of the program's semaphores, which carries its hash: every program set up on a rank shares the shared memory prefix, and a service that removes its own objects must never remove those of another program's run.
4. Forks one process per task, keeping at most `ZISK_ASM_PAR_WORKERS` of them running at a time.  Unset, that is the number of CPUs the service may run on: those in its CPU affinity mask (`taskset`, a container's CPU set), or the online CPUs if the mask cannot be read, but no more than its cgroup v2 CPU quota allows, such as a container's CPU limit (the `cpu.max` of its cgroup and of the cgroup's ancestors, quota over period rounded up).  The service reads the variable, or counts the CPUs, at its first region, so the variable must be set before the services start.

Each task's process:

- Sees the guest RAM as a private copy, except for the range its task owns, which it shares with the parent.  Its writes anywhere else are lost.
- Maps its trace object over the trace address range, at the same addresses, so its chunks start where the first pass's chunk is in the main trace.  It writes the saved prefix first, then runs the task.
- Publishes every chunk as it closes it (`TASK_CHUNK`).  In MT, the step in a chunk's header counts as if the task ran first, from the region's first step; the reader adds the steps of the tasks before it.
- At its next pass, checks that the pass starts in the first pass's state.  In MO it checks only the `BOUNDARY` step's records, because the MT service runs the same program and checks the rest.  It then publishes how long the task ran (`TASK_DONE`) and exits.

Once every task's process has exited, the parent opens the region's last pass where the first pass's chunk was.  Its step is the region's first step plus the steps of every task and, in MT, `s11 = n`.  It publishes `REGION_END` and goes on emulating.  Nothing is copied: the region leaves nothing in the main trace.

Exit codes of a task's process (`emulator-asm/src/par.hpp`):

| Code | Meaning |
|---|---|
| 0 | Reached the next pass, with every chunk published |
| 2 | Could not set up its trace or memory |
| 3 | The guest ended the emulation inside the task |
| 4 | The guest faulted inside the task |
| 5 | Reached another pass than the next one, or in another state |
| 6 | Could not publish a chunk or its end |
| 7 | Writing its trace, an event or the guest RAM faulted (`SIGBUS`): `/dev/shm`, which backs them, is full |
| 8 | The emulation reached `max_steps` inside the task |
| 9 | A reset stopped the emulation inside the task: the run is being cancelled |

Any code other than 0 aborts the run.  Codes 3 and 4, the guest's, abort it with `PAR_ABORT_TASK_ENDED` and `PAR_ABORT_TASK_FAULTED`, which the readers report as `AsmRunError::ForkJoinTask`.  Every other code, the host's failures 7 to 9 included, aborts it with `PAR_ABORT_ERROR`, which the readers report as `AsmRunError::ForkJoinRegionFailed`.  Both errors name the region and the task, and `ForkJoinRegionFailed` what failed, as the service reports it in its events object (see [Chunk events](#chunk-events)).  A task's process handles `SIGBUS` itself, so that running out of `/dev/shm` is never reported as the guest faulting.  When a run aborts, its tasks' processes are killed, so none outlives the region.

### Fallbacks

The assembly emulator runs a region sequentially, publishing ordinary chunks, in two kinds of cases.  The first kind follows from how the execution is configured:

- the service is not MT or MO;
- the service has no chunk events object;
- the execution takes precompile results as hints, because a task would need its place in their stream;
- the chunk size is not the one the driver pads to;
- `ZISK_ASM_PAR_WORKERS` is 1 or less, or not a number, or it is unset and the service may run on one CPU only;
- the region has fewer than two tasks.

The second kind comes from the guest or from the run's budget:

- the task list is not in the RAM or lies in memory a task owns, a task does not own page-aligned RAM that holds its stack, or two tasks own overlapping memory;
- the region would take the run past its budget of 512 tasks run apart;
- the first pass does not start a chunk.

The service reports, in its events object, how many regions of the run it ran sequentially and why, and the MT reader logs it once per run: `fork-join: <n> region(s) ran one task after another: <reasons>`.  It logs at `info` when every reason is of the first kind, which is how the execution is set up, and at `warn` when one is of the second kind.  The execution's `AsmExecutionInfo` counts the regions run apart and those run sequentially (`fork_join_regions_apart`, `fork_join_regions_sequential`).  With `-v`, the services' own output goes to the log as well, each line prefixed with `[asm <service>]`: the reason for every region run sequentially, and the details of every warning.

### Chunk events

Every chunk reaches its reader as an event, and every region change is an event too (`emulator-asm/src/par_events.hpp` is the contract; `emulator-asm/asm-runner/src/par_events.rs` is its Rust side).  Each service has an events object, `<shm_prefix>_<MT|MO>_events`, with room for 2^18 + 2^11 events per run: one per chunk of the longest execution (2^36 steps), and the events of the 512 tasks a run runs apart at most.  The service posts its `chunk_done` semaphore once per event, and the reader handles one event per post, in event order.

| Event | Published by | `a`, `b`, `c` |
|---|---|---|
| `MAIN` | The process that owns the main trace | The next chunk of the main trace is complete, at the reader's cursor |
| `REGION` | Parent, before it forks | Region number, number of tasks, step of the first pass |
| `TASK_CHUNK` | Task's process | Task, chunk index within the task, byte offset in the task's trace object |
| `TASK_DONE` | Task's process, before it exits | Task, steps, chunks |
| `REGION_END` | Parent, once every task has exited | Region number |

A writer reserves its slot with an atomic fetch-add on `tail`, writes `a`, `b` and `c`, stores `kind` with release ordering, and then posts the semaphore.  A reader that finds `kind` still 0 waits for it.  The service zeroes the slots used by its last run before every run.

The object's header also holds the run's report, which the reader reads once the service has answered: the regions run sequentially and a bit per reason (`PAR_SEQUENTIAL_*`), and, if a region failed, how (`PAR_FAILURE_*`), the region and the task, and a detail such as the task's exit code, the signal that killed it or the system's error number.  Only the process that owns the main trace writes it, and only the first failure, which the others follow from.

The reader unlinks each task's trace object as soon as it maps it and keeps the mapping, not a file descriptor, so its memory goes away with the last mapping, even if a process dies before the service cleans up.  The service unlinks the objects of its last run that the reader never mapped at the start of its next run and at its teardown, and at its setup any object of its program and service that a service which died left behind; a program's teardown also unlinks the task objects such a service left.  A reader waiting for an event its writer reserved but never filled stops as soon as the service ends the run, and reports how the run failed.

### Readers

Both readers follow a region's events with the same `RegionReader` (`par_events.rs`), which maps the region's task traces and checks the events' order; each reader only says how it reads a chunk and what it does with it.  The chunks of a region arrive interleaved between tasks, in order within each task.  `RegionSequencer` hands them on in sequential order.  It releases task `i`'s chunks once every task before `i` is done, because only then are the steps they start at known.  The task at the frontier (the first one not yet done) is released as its chunks arrive.

- **MT.**  The reader adds the steps of the tasks before a chunk's task to its step, and hands it on with the next `ChunkId`.  The executor counts it and builds the Main instances as for any other chunk.
- **MO.**  The reader feeds each chunk's records to the memory planner, CPU or GPU, at the frontier.

So nothing past the readers knows about regions: the executor and both memory planners see the chunks of a sequential execution, in its order and with its steps.  The MT reader also counts the regions it reads run apart, which the execution reports in its `AsmExecutionInfo` and the tests use to check that regions did run apart.

## Guarantees and verification

### Checked on every run

- **Guest runtime:** every task's stack canary, when the task returns.  Reading the input or committing outputs inside a task panics.
- **Rust emulator:** the driver invariant above, at every marker, when it runs the guest itself.
- **Assembly emulator:** the task list before anything reads through it.  Each task's process also checks the canonical state at its next pass, and the parent checks what each task reports (steps, chunks, `max_steps`).
- **Failures:** a failing task process aborts the run, and no task process outlives its region.

### Checking a change

The minimal-trace oracle compares the assembly emulator, running regions apart, against the Rust emulator, which runs them sequentially.  With `ZISK_DUMP_MIN_TRACES=<path>`, every execution writes one line per chunk to `<path>`: its start state, its end and a hash of its memory reads.  Both runs must write the same file:

```bash
ZISK_DUMP_MIN_TRACES=mt-emu.txt cargo-zisk-dev execute -e guest.elf -i input.bin --standalone
ZISK_DUMP_MIN_TRACES=mt-asm.txt cargo-zisk-dev execute -e guest.elf -i input.bin --standalone -a
cmp mt-emu.txt mt-asm.txt
```

They use `cargo-zisk-dev`, the development CLI that `cargo build --release` builds with the workspace: `cargo-zisk` has neither a standalone execution nor `verify-constraints`.  Then run `cargo-zisk-dev verify-constraints -e guest.elf -i input.bin -a`, which runs the regions apart and checks the constraints of the whole trace.  The assembly emulator is cached per ROM: after changing `emulator-asm` or `ziskasm/zisklib`, delete the cached build so the change takes effect.

In CI, `integration-tests/tests/asm_fork_join.rs` runs this comparison on `fork_join` and on the C SAXPY guest, and checks with the execution's counts that their regions ran apart, and that a guest whose regions have one task each runs them sequentially.  `emulator/tests/zkvm_parallel.rs` checks the C guest on the Rust emulator.

The implementation has also been verified as follows:

- The MT oracle gives identical traces on six guests, from 6 to 2,271 chunks.
- The parallel MO trace has the same hash as the sequential one, in both heavy and light forms.
- The memory plans of parallel and sequential executions of the same block are identical, with the CPU and the GPU memory planners.
- `verify-constraints` passes with regions run apart, on the CPU and on the GPU.
- Full proofs, aggregated and verified, of `fork_join` and of blocks of 550 and 2,200 transactions, with regions run apart.
- Proofs in distributed mode, a coordinator and a GPU worker, of `fork_join` and of a block of 550 transactions, verified with `cargo-zisk verify`.
- The proofs of a guest without regions are those of the base branch: the same instances and the same proof time.  With regions, a 2,200-transaction block takes 242 instances instead of 240 and 0.6% more proof time, from the padding of every task to a chunk boundary; a 550-transaction block takes the same 84.

## Configuration

| Variable | Default | Effect |
|---|---|---|
| `ZISK_ASM_PAR_WORKERS` | The CPUs the service may run on: those in its affinity mask, capped by its cgroup v2 CPU quota | Task processes each service runs at a time; 1 or less, or a value that is not a number, runs regions sequentially.  Read once per service, at its first region |
| `ZISK_DUMP_MIN_TRACES` | Unset | Writes the minimal-trace dump used by the oracle |

## Troubleshooting

On the assembly emulator, a task that panics fails the execution with `AsmRunError::ForkJoinTask` ("the guest ended (it exited or panicked) inside a fork-join task (task 2 of region 0)"), without the panic's message.  The Rust emulator runs the tasks in order and prints it: run the same input with `cargo-zisk-dev execute --standalone`, without `-a`.

| What you see | Cause | What to do |
|---|---|---|
| `a fork-join task ran deeper than its N-byte stack` | The task's stack overflowed into the canary | Raise `par::Config::stack_bytes` |
| `OOM limit of heap with bump allocator`, inside a task | The task allocated more than its heap | Raise `par::Config::heap_bytes` |
| The same message before the region | The region's arenas do not fit in the caller's heap: they add up over every region, in 512 MiB of RAM | Fewer tasks per region, or smaller arenas |
| `reading the input inside a fork-join task` or `committing outputs inside a fork-join task` | The task calls `ziskos::io::read*` or `ziskos::io::commit*` | Read before the region and pass the data to the tasks; commit after it |
| `ForkJoinTask`, and the Rust emulator ends normally | A task exits the program | Return from the task instead; run `ZISK_ASM_PAR_WORKERS=1` to compare with the sequential result |
| `ForkJoinRegionFailed` | The region failed in the service, not in the guest: `/dev/shm` full, `max_steps` reached inside a task, a cancelled run, a task's process that could not set up or publish, or a service that could not set the region up | The error says which, and in which task; with `-v`, the service's log gives its details |
| The Rust emulator panics with `fork-join: …` | A raw-ABI task breaks the driver's invariant: it does not preserve a callee-saved register, `gp` or `tp`, or does not return to `zkvm_parallel_ret` | Fix the task; the message names the register and the step |
| The execution succeeds but the proof fails | A task broke the contract: it wrote memory it does not own, or read what another task writes | Run the oracle (see [Checking a change](#checking-a-change)): the first line that differs is the first chunk that read another value than in the sequential execution |
| No speed-up | The regions ran sequentially (see [Fallbacks](#fallbacks)), the tasks are short next to the padding of a chunk (2^18 steps) each, or the execution is bound by the stages after the emulation (see [Execution phase](#execution-phase)) | See below |

To tell whether the regions ran apart, look at the execution's log: the assembly execution's speed line ends with `<n> fork-join region(s) run apart`, and regions run sequentially are logged with their reasons (see [Fallbacks](#fallbacks)).  A program reads both counts from the execution's `AsmExecutionInfo`.  With `-v`, the services' output goes to the log too: for every region a service runs apart, a line `[asm mt] … fork-join region <n> (MT): <tasks> tasks on <workers> workers, …`, with the time it spent forking the tasks' processes and waiting for them, and what they used: CPU time, the largest peak resident memory and page faults.

## Performance

Measured in November 2026 with a synthetic guest that models the validation of a block with an access list, opening 16 tasks per region for 300 and 550 transactions and 48 for 2,200.  Sequential means the same block run by a guest without regions.

### Assembly emulator alone

What this mechanism speeds up is the emulator's execution.  The services run by hand, without counting or memory plan, as an execution runs them: MT, MO in one of its two forms, and RH at once.  RH runs regions sequentially and its result is awaited later, for the ROM instance, so the execution phase waits on the slower of MT and MO.  32 cores, sequential → parallel (medians of 5):

| Block | Steps | MO heavy | MO light |
|---|---|---|---|
| 300 tx | 86 M | 94 → 66 ms (1.4×) | 73 → 46 ms (1.6×) |
| 550 tx | 160 M | 175 → 118 ms (1.5×) | 134 → 84 ms (1.6×) |
| 2,200 tx | 595 M | 626 → 254 ms (2.5×) | 483 → 185 ms (2.6×) |

Each service alone, on the 2,200-transaction block: MT 464 → 143 ms (3.3×), MO heavy 3.0×, MO light 3.4×.  The more cores, the more a large region gains: on 256 hardware threads, MT alone runs it 5.2× faster, and MT with both MO forms at once 4.0×.  RH, sequential, now takes longer than MT and MO (about 700 ms on that block, against 650 ms with a guest without regions, as it shares the cores with the tasks).

Most of a region's time goes to writing its trace: a task writes into fresh shared memory, which the kernel allocates and clears page by page, and the MT and MO traces of the 2,200-transaction block take 1.4 and 3.0 GB.  Writing that much is bound by the memory's bandwidth, so smaller traces, or more bandwidth, are what would make regions faster still.

### Execution phase

Emulation, MT counting and memory plan, sequential → parallel:

| Block | 32 cores, GPU memory planner | 256 hardware threads, CPU memory planner |
|---|---|---|
| 300 tx | 128 → 113 ms (1.13×) | 341 → 527 ms (slower) |
| 550 tx | 214 → 190 ms (1.13×) | 660 → 806 ms (slower) |
| 2,200 tx | 760 → 689 ms (1.10×) | 2,196 → 2,550 ms (slower) |

The emulation gets faster, but the stages that consume its chunks set the floor.  They exist with or without regions, sized to keep pace with a sequential emulator:

- **MT counting.**  Every chunk is replayed with the Rust emulator to fill its counters.  That costs about 8 core-seconds per 600 M steps, against 0.7 for the assembly emulation itself, so on 32 cores it takes at least 250 ms whatever the emulator does.
- **Memory plan, GPU.**  Each stream of the planner is fed by a host thread of its own, at about 0.6 ms of CPU per chunk.  With four streams that keeps pace with a sequential emulator but not with a parallel one, and in a parallel run these threads also compete for the cores with the task processes and the MT counting.
- **Memory plan, CPU.**  The CPU planner is slower than the emulation even in a sequential run, so a faster emulation does not shorten the phase.  Its cost also grows with the span of addresses the guest touches, because it walks the gaps between the touched ones; the tasks' arenas spread a region's memory, which adds 150 to 350 ms on these blocks.  Sizing arenas for what the tasks allocate keeps the span small.

Removing those floors is work on the consumers, not on the emulation: counting in the assembly emulator, a cheaper host side and more streams for the GPU planner, and a CPU planner that skips untouched memory.

## Limitations and future work

- **Hints per task.**  Today, a guest that takes hints runs its regions sequentially.  This is what production guests need.
- **The stages after the emulation** (see [Execution phase](#execution-phase)): counting in the assembly emulator, and the memory planners.
- **The Rust emulator** runs regions sequentially, so hosts without the assembly emulator gain nothing.
- **WebAssembly guests** (`wasm2rom`) have no `zkvm_parallel` import: a task is a code address in the ROM, and a wasm function reference is a table index.
- **RH service** running regions apart.
- Streamed input, host randomness and the Keccak-f cache inside tasks.
- A guest that exits inside a task fails, where its sequential execution ends normally.
- A conflict detector for development runs, cancellation in the middle of a region, and restarting a service while its readers are open.
- Native execution with threads, nested regions that run apart (today they run inline in their task), automatic arena sizing and a versioned ABI.
