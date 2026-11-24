# Allocator precompile: notes for implementation and verification

Read `README.md` first for what the precompile does. This file is for whoever (person or AI
agent) writes the executor and witness generation, wires the AIR into zisk, or verifies the
constraints formally. It records the exact design, the reasons behind each constraint, and what is
not done.

## State of the work

Done:
- `pil/allocator_flex.pil` (`AllocatorFlex`) and `pil/allocator.pil` (`Allocator`). Both compile
  standalone with the pinned pil2-compiler through `test/allocator_test.pil`.
- `model/allocator_flex_model.py` and `model/allocator_model.py`. Each has `verify()` (the
  constraints re-implemented by hand over Goldilocks plus the bus balance), an honest `Executor`
  that fills the witness, an `oracle()` for the guest-visible guarantees, and attacks.
  The `Allocator` model also has `reference()`, an independent best-fit allocator.

Not done:
- No wiring into `pil/zisk.pil`, `pil/operations.pil`, `pil/opids.pil`. Opcodes `0xDB`-`0xDF` and
  bus ids `8300`/`8301` are provisional and only live in `pil/allocator_common.pil`.
- Emulation only (ziskemu). The operations have no opcode: the guest reaches them through the
  allocation mark CSR `0x822` with the kinds `ALLOC_OP_*` (16-20) of
  `definitions/src/alloc_mark.rs`, transpiled to a `copyb` whose `func`
  (`core/src/operations/allocator.rs`) runs the operation on `core/src/alloc_heap.rs`; alloc and
  calloc return the address in rd as a free value. `ziskemu --alloc-policy` picks the variant and
  the stats charge the rows and memsets (`ops_alloc`). The ziskos allocator is the feature
  `zisk-precompile-alloc` (`ziskos/entrypoint/src/alloc/precompile_alloc.rs`).
  `ziskemu --alloc-stats` estimates the three variants for any guest by replaying its calls
  (`emulator/src/stats/alloc_estimate.rs`).
- No assembly emulator, no witness, no planner, no main-bus wiring (a provable version needs real
  opcodes again for the operation bus). `verify-constraints` has never run on these AIRs.
- No formal verification.

**The models are not the PIL.** `verify()` is a hand transcription. Any change to a `.pil` must be
mirrored in its model and vice versa. The `Allocator` model checks constraints per operation with Python
branches where the PIL uses selectors; the column names differ (see the mapping below).

## Commands

Run from the repo root.

```
python3 precompiles/allocator/model/allocator_flex_model.py
python3 precompiles/allocator/model/allocator_model.py
```

Both must end with `RESULT: OK`. "weak" lines show that an attack succeeds when the named
protection is removed; that is the expected outcome.

Compile the PIL standalone (well under a second) and get the column and memory numbers:

```
PIL2C=<pinned pil2-compiler>/node_modules/.bin/pil2com
STD=<pil2-proofman checkout matching Cargo.lock>/pil2-components/lib/std/pil
node $PIL2C precompiles/allocator/test/allocator_test.pil -I pil,$STD,state-machines,precompiles \
  -o tmp/allocator/allocator_test.pilout -u tmp/allocator/fixed -O fixed-to-file -O no-proto-fixed-data
cargo-zisk-dev proofman-setup stats -a tmp/allocator/allocator_test.pilout -o tmp/allocator/stats.txt
```

The harness instantiates both variants in one airgroup, which is why they have different bus ids
(the token tuples have 5 and 8 elements and one permutation id needs one arity).

## Main operation bus

Tuple `[op, a0, a1, b0, b1, c0, c1, flag, main_step, extended_arg, extra]`, proved with
`proves_operation`:

| op | a | b | c |
|---|---|---|---|
| `OP_ALLOC_CREATE_CLEAN`, `OP_ALLOC_CREATE_DIRTY` (= clean + 1) | `[addr, 0]` | `[size, 0]` | `[0, 0]` |
| `OP_ALLOC_ALLOC`, `OP_ALLOC_CALLOC` (= alloc + 1) | `[size, 0]` | `[0, 0]` | `[addr, 0]` |
| `OP_ALLOC_DEALLOC` | `[addr, 0]` | `[0, 0]` | `[0, 0]` |

The high chunks are the constant 0. This is a soundness requirement, not a simplification: a
64-bit `size` such as `p + 64` equals 64 as a field element, and would otherwise be accepted as an
allocation of 64 bytes.

`proves_operation` is a lookup with a free multiplicity `mul`. Nothing in the std forces it to be
0 or 1; the selector constraints of the AIR do.

A `calloc` over a dirty block assumes `OP_DMA_XMEMSET` with exactly the tuple that `Dma` proves to
the main: `a: [addr, 0], b: [size, 0], c: [addr, 0], main_step, extended_arg: 0` (fill byte 0).

## `AllocatorFlex` (prover chooses)

Bus `ALLOC_FLEX_BUS_ID`, tokens `[type, addr, size, step, dirty]`:

- `FREE(addr, size, step, dirty)`: free block available since `step`. `dirty = 0` means all bytes
  are zero.
- `USE(addr, size, step, 0)`: block handed out at `step`.

Rows and their bus effect (`-` assume, `+` prove):

| Row | Effect |
|---|---|
| create | `+FREE(addr, size, step, dirty)` if `will_be_use` |
| alloc/calloc | `-FREE(free_addr, free_size, free_step, free_dirty)` always; `+USE(free_addr, size, step)` if `will_be_freed`; `+FREE(free_addr + size, free_size - size, step, free_dirty)` if `will_be_use` |
| dealloc | `-USE(free_addr, free_size, free_step)`; `+FREE(free_addr, free_size, step, 1)` if `will_be_use` |
| join (internal, no main op) | `-FREE(addr, size, step, dirty)`, `-FREE(addr + size, free_size, free_step, free_dirty)`; `+FREE(addr, size + free_size, max(step, free_step), out_dirty)` |

Points that are easy to get wrong:

- `free_step` is not a column. It is `step - (1 - 2*free_is_newer) * (1 + step_diff)` with
  `step_diff` range checked (16 + 24 bits), so `free_step < step` holds by construction.
  `free_is_newer` may be 1 only in a join, where either token can be the newer one.
- A join carries the **newest** of the two steps. With the older one, an alloc that precedes the
  release of one of the pieces could take the joined block.
- In a join `(addr, size)` is always the lower block. The original spec had a flag for which
  address is lower; it was replaced by `free_is_newer` to keep degree 2.
- Alloc: `free_size = size + will_be_use + rest`, `rest` range checked. This gives
  `size <= free_size` and a non-empty rest when it is produced. There is no `use_all` flag.
- Alloc: `size = 8 * (size_chunks + 1)`. Non-zero is required for soundness: with `alloc(0)` two
  live `USE` tokens share an address and `dealloc(addr)`, whose size is a witness, can release the
  wrong one.
- `out_dirty` is only constrained from below (`>= dirty`, `>= free_dirty`, `= 1` after dealloc).
  A prover may mark a clean block dirty; that costs a memset and nothing else.
- The token of a join is always produced (`will_be_use = 1` forced). The executor must only emit
  joins whose result is consumed later, or the bus does not balance. The model joins lazily, when
  an alloc needs it.

## `Allocator` (deterministic)

Bus `ALLOC_BUS_ID`, tokens `[type, addr, size, prev, dirty, ver, next_size, next_addr]`:

- `FREE`: free block and list node. `(next_size, next_addr)` is the key of the next node in the
  list ordered by `(size, addr)`.
- `USE`: block handed out. `dirty` and `next_*` are 0.
- `GUARD`: end of a created area, at `addr + size` of the area, with `size = 0`.
- `prev`: size of the block immediately below, 0 for the first block of an area.
- `ver`: version of the update that produced the token.

Fixed tokens and keys:
- Head of the list: `FREE(addr 0, size 0, prev 0, dirty 0, ver 0, next = tail)`, proved once with
  `allocator_init()` (a global `direct_global_update_proves`).
- Tail: only a key, `(2**32, 0)`. No token.
- List key: `size * 2**30 + addr`. This equals lexicographic `(size, addr)` only because sizes are
  multiples of 8 and `addr < 2**33`. Keys are below `2**62`, the tail key is `2**62`, and a strict
  comparison is a 62-bit range check (16 + 22 + 24) of `b - a - 1`.

### Versions

The row of main step `s` performs up to three sequential updates with versions `4s+1`, `4s+2`,
`4s+3`. A consumed token's version is not a column: it is `update_version - 1 - ver_diff`, with
`ver_diff` range checked to 40 bits (16 + 24), so it is strictly older than the update.

There is deliberately **no clock token**. An earlier version chained the rows with a
`CLOCK(v, step)` token and a dense counter `v`. Tying the version to the step makes it redundant:
a row cannot consume anything produced at a later step, which is the only thing the chain
prevented. Rows are independent of each other.

Because versions are per update and not per row, the updates of one row can touch the same token
in sequence (for example, the predecessor of block A is also the predecessor of the joined block)
with no special case: update 3 consumes what update 1 produced.

### Updates per operation

| Update | create | alloc | dealloc |
|---|---|---|---|
| 1 (`4s+1`) | | remove A from the list (A = block taken, `a_addr` is the result); produce `USE(a_addr, size, a_prev)` | consume `USE(addr, size, prev)`; below block A: remove from the list if free (`join_below`), else consume and re-produce its `USE` |
| 2 (`4s+2`) | | | above block B: remove from the list if free (`join_above`) |
| 3 (`4s+3`) | insert M = the area; produce `GUARD` | if `has_rest`: insert M = rest; update `prev` of UP | insert M = joined block; update `prev` of UP |

Primitives, as bus effects:

- remove N with predecessor P: `-FREE(P -> N)`, `-FREE(N -> next)`, `+FREE(P -> next)`.
- insert M with predecessor Q: `-FREE(Q -> next)`, `+FREE(Q -> M)`, `+FREE(M -> next)`, with
  `key(Q) < key(M) < key(next)`.
- update UP: `-T(up_addr, up_size, prev = up_prev)`, `+T(up_addr, up_size, prev = m_size)`,
  `T` = `USE` or `GUARD`.

Why each check exists:

- **Best fit** (alloc): `pa_size < size` where PA is the list predecessor of A. Together with
  `size <= a_size` this makes A the first node whose size is at least `size`.
- **Mandatory rest** (alloc): `a_size = size + 8*has_rest + r32`, and `has_rest = 0` forces
  `a_size = size`. The prover cannot drop a rest to steer later allocations.
- **Forced join** (dealloc): a neighbour is either removed from the list (so it was free) or
  shown as `USE`/`GUARD` (so it was not). Both need a token that exists only in one case, so the
  join flags are not a choice. "Shown" means consumed and produced again with a new version;
  re-producing the identical token would cancel on the bus and prove nothing.
- **`has_below`**: `(sel_dealloc - has_below) * prev = 0`. Without it a prover could skip the
  check of the block below.
- **UP**: the first non-free block above the new free block. In dealloc it is what proves there is
  nothing else to join above, and its `prev` must be rewritten because the block below it changed
  size. In alloc with an exact fit nothing above changes, so UP is not touched.

### Adjacent areas

The `GUARD` of one area and the first block of an area created right after it have the same
address. They are told apart by `prev`: a `GUARD` or any non-first block has `prev != 0`, the first
block of an area has `prev = 0`. This is also why two areas never join. The model's executor got
this wrong twice during development and `verify()` rejected both traces; an executor in Rust must
look up the upper block by `(addr, prev != 0)`, not by address alone.

### Model to PIL column names

| Model | PIL |
|---|---|
| `n_addr`, `n_size`, `n_prev`, `n_dirty`, `nn_*` (alloc) | `a_addr`, `a_size`, `a_prev`, `a_dirty`, `a_next_*` |
| `below[...]` (dealloc) | `a_*` |
| `rm` / `rm_below` (`p_*`, `emit_p`) | `pa_*`, `emit_pa` |
| `above[...]`, `rm_above` | `b_*`, `pb_*`, `emit_pb` |
| `ins` (`q_*`, `qn_*`, `emit_q`, `emit_m`) | `q_*`, `qn_*`, `emit_q`, `emit_m` |
| `up` (`guard`, `size`, `ver`, `emit`) | `up_guard`, `up_size`, `up_ver`, `emit_up` |
| computed `m_addr`, `m_size`, `m_prev` | witness columns `m_*`, plus `below_size`, `up_addr`, `up_prev` |

The PIL has witness columns for values the model computes (`m_*`, `up_addr`, `up_prev`,
`below_size`) to keep every constraint and bus expression at degree 2 or less.

## Witness generation

Common to both variants:

- The `emit_*` / `will_be_*` flags depend on the future: a flag is 1 only if the token is consumed
  later. Generate rows with the flags at 0 and set a flag when its token is consumed. Both models
  do this with a registry from token to the row and flag that produced it.
- A token that is never consumed must not be produced, or the bus does not balance. This is how
  the heap left at the end of the execution is closed.
- Every executor (Rust emulator, assembly, GPU) must return the same addresses. For
  `Allocator` the address is unique, so any correct best-fit with eager coalescing agrees. For
  `AllocatorFlex` the policy is free but must be identical everywhere.
- Unused columns must be 0. Several constraints have no selector and rely on it (unconditional
  range checks, `dirty * (1 - out_dirty)`, `(1 - join_above) * b_size`).

`AllocatorFlex`: one row per main operation plus one per join. Join rows have no main step; their
`step` and `free_step` are those of the two tokens.

`Allocator`: one row per main operation. The executor needs the free list ordered by
`(size, addr)` with predecessor lookup, and the neighbours of each block by address.

## For formal verification

Target statements, for any accepted proof and under the assumptions in `README.md`:

1. Every `alloc` result lies in a created area.
2. Blocks live at the same main step are pairwise disjoint.
3. A `calloc` block is all zeros right after its row.
4. (`Allocator` only) the result of every `alloc` equals the result of the reference best-fit
   allocator with eager coalescing, given the same sequence of operations.

Proof outline used in the design:

- The bus balance gives a bijection between produced and consumed tokens.
- Order the rows by step (in `AllocatorFlex`, place each join at the step of its output; joins at the
  same step are ordered by output size, which strictly grows). The causality checks make this a
  topological order of the producer/consumer relation.
- Invariant, by induction along that order: the tokens produced and not yet consumed describe the
  true heap. For `AllocatorFlex`: they are pairwise disjoint intervals inside the areas. For
  `Allocator` additionally: each `FREE` token's `next` is the true next key among live free
  blocks, each `prev` is the true size of the block below, and no two adjacent blocks are free.
- A token the prover chose not to produce is missing from the set but does not make any existing
  token false. Missing tokens can only make later rows unprovable.

Things a verification must not skip:

- Booleanity of every selector and flag. A bus selector is a field element used as a
  multiplicity: `will_be_use = 2` duplicates a free block, `-1` on an assume is a prove. The
  permutation calls pass `sel_is_binary: 1`, so the std adds no constraint of its own; the AIR
  does. `dirty` and `calloc` also enter the opcode (`op + flag`), so a non-boolean value would
  select another operation.
- Field wraparound. Every subtraction that stands for an inequality has a range check sized so
  that no honest or dishonest value wraps `p = 2**64 - 2**32 + 1`. Token fields are bounded by
  induction from the main's 32-bit chunks at `create`.
- The list key relies on multiples of 8 and `addr < 2**33`. `create` does not enforce the
  multiple-of-8 part; it is an assumption on ziskos. Enforcing it costs a few range checks.

## Open items

- `OP_DMA_XMEMSET` is assumed at the same `main_step` as the alloc. Check against the memory
  timestamps of the instruction itself and make sure the memory planners count these operations.
- `create` arguments are trusted (alignment, non-overlap, zeroed). Decide which to enforce.
- Alignments above 8 and `realloc` are not covered. A `realloc` in place would be a new operation.
- `Allocator` column sharing: the alloc-only chunks (`r32_*`) and the dealloc-only update 2
  (`b_*`, `pb_*`) never appear in the same row and could share columns.
- Multiple areas share one free list in `Allocator`. One list per area would need an area id in
  the tokens.
- `MAIN_STEP_BITS + 2` must stay at or below 40 for the version range check; there is an `assert`.
