# Allocator precompile (prototype)

A precompile that validates the operations of a heap allocator, so the guest does not have to run
an allocator inside the zkVM. The guest asks for memory with one instruction; the executor decides
which address to return; the precompile proves that the answer is a valid one.

**Status: design prototype.** The PIL compiles standalone and the design is validated with Python
models. Nothing is wired into `zisk.pil`, there is no executor, no witness generation and no ziskos
support yet. Opcodes and bus ids are provisional.

## Operations

The main state machine sends these operations to the allocator through the operation bus:

| Operation | Arguments | Result | Meaning |
|---|---|---|---|
| `create_clean` | `addr`, `size` | | Registers an area of memory that is known to be zeroed. |
| `create_dirty` | `addr`, `size` | | Registers an area of memory that may contain anything. |
| `alloc` | `size` | `addr` | Returns a block of `size` bytes. |
| `calloc` | `size` | `addr` | Same, and the block is zeroed. |
| `dealloc` | `addr` | | Releases a block returned by `alloc`/`calloc`. |

`create_*` is called by ziskos when it sets up the heap, normally once. `size` in `alloc`/`calloc`
must be a non-zero multiple of 8.

There is no failure result. If no free block fits, the execution cannot be proved. This is on
purpose: with a failure result a prover could claim "out of memory" whenever it wanted.

## What is guaranteed

For any proof that verifies:

- every block returned lies inside a created area,
- two blocks that are live at the same time never overlap,
- `dealloc` only succeeds on the start address of a live block, once,
- a `calloc` block is zeroed when it is returned (a `memset` is launched only when the block may be
  dirty).

These hold against a malicious prover, under the assumptions below.

## Two variants

The two variants validate the same operations and give the guarantees above. They differ in how
much the prover is allowed to decide. Only one of them would be instantiated.

`Allocator` is the default. `AllocatorFlex` is the cheaper alternative, for when the guest is
trusted to have no memory bugs.

### `Allocator`: nothing to choose

The address returned by each `alloc` is fully determined by the previous operations:

- **Best fit.** The block taken is the smallest free block that fits, and the lowest address among
  equals. The free blocks form a list ordered by (size, address) and the precompile verifies that
  the block taken is the first one that fits.
- **Forced coalescing.** A released block is always joined with its free neighbours, in the same
  row as the `dealloc`. There are no separate joins.
- **Mandatory rest.** When the block taken is bigger than requested, the rest always goes back to
  the free list.

A prover cannot shape the heap, so a guest bug behaves the same for every prover.

### `AllocatorFlex`: the prover chooses

The prover picks which free block serves each `alloc` (always from the start of a free block) and
decides when two adjacent free blocks are joined, with internal `join` rows that do not come from
the main.

This is sound as long as the guest program is correct. If the guest has a memory bug (an overflow,
a use after free), a malicious prover can arrange the heap so that the bug hits the data it wants.
The result of a program that depends on pointer values is also not unique.

### Cost

Measured with `proofman-setup stats` on the standalone PIL, one instance of 2^22 rows:

| | `AllocatorFlex` | `Allocator` |
|---|---|---|
| Witness columns | 25 | 81 |
| Committed columns (base field, all stages) | 55 | 156 |
| Constraints | 42 | 92 |
| Max constraint degree | 3 | 3 |
| Prover memory | 4.93 GB | 11.24 GB |
| Rows | one per operation, plus one per join | one per operation |

Both only use range checks that other state machines already use (16, 22 and 24 bits).

## Assumptions on the guest

The precompile does not verify these; ziskos must guarantee them:

- Created areas do not overlap each other or any other memory in use.
- An area registered with `create_clean` is really zeroed, and the program does not write to
  memory it has not allocated.
- `addr` and `size` of a created area are multiples of 8. Every returned address is then aligned
  to 8. Larger alignments are the job of the ziskos wrapper.

## How it works

The allocator state is not stored in a trace. It lives in a private multiset bus as a set of
tokens: one token per free block and one per block in use. Each row consumes the tokens it
modifies and produces the new ones, and the bus must balance at the end. Three rules make this
sound:

1. **Conservation.** A row only produces blocks that are inside the blocks it consumes. Only
   `create` produces without consuming.
2. **Causality.** Every token carries the time at which it was produced, and a row can only
   consume older tokens. Without this, the bus (which has no notion of time) would accept an
   `alloc` that consumes the free block produced by a later `dealloc` of that same `alloc`.
3. **Unique addresses.** Sizes are not zero, so two live blocks never share an address and
   `dealloc(addr)` identifies one block.

`Allocator` adds, to each free token, the key of the next free block in the ordered list, and
to every token the size of the block just below it. With the first the precompile proves that
nothing smaller would fit; with the second it finds both neighbours of a block and proves whether
each one is free.

The prover may leave out a token that nobody will consume again, which is how the bus is closed at
the end of the execution. Leaving out a token that is needed later makes that later operation
unprovable; it never changes its result.

## Files

- `pil/allocator_flex.pil`, `pil/allocator.pil`: the two variants.
- `pil/allocator_common.pil`: opcodes, bus ids and token types.
- `model/`: Python models with an honest executor, a re-implementation of the constraints and a
  set of attacks. `python3 model/allocator_flex_model.py`, `python3 model/allocator_model.py`.
- `test/allocator_test.pil`: harness to compile both variants standalone.
- `AGENTS.md`: implementation notes for the executor and for formal verification.
