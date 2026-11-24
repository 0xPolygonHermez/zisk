# Allocator precompile: status and value

**State:** design prototype, runnable in ziskemu only. It cannot be proven yet. Details:
[README.md](README.md) (design), [AGENTS.md](AGENTS.md) (implementation notes).

## What it brings

Today every guest runs a heap allocator (bump, dlmalloc, talc, tlsf) inside the zkVM. A real
allocator costs 60-80 steps per call, and a bump allocator never reuses memory.

With the precompile, the guest asks for memory with a single instruction. The executor picks the
address, and the precompile proves the answer is valid:

- blocks lie inside the heap and never overlap,
- `dealloc` is only accepted once, on a live block,
- `calloc` blocks are zeroed.

Each call costs the guest about 1 step plus one row of the precompile. Memory is reused like a
real allocator does.

## Two versions

| | `Allocator` | `AllocatorFlex` |
|---|---|---|
| Who picks the address | Nobody: best fit, unique result | The prover |
| Joining free blocks | Always, in the `dealloc` row | When the prover decides; one row per join |
| Columns (cost per row) | 156 | 55 |
| Prover memory, 2^22 rows | 11.2 GB | 4.9 GB |
| Guest with a memory bug | Same result for every prover | A malicious prover can shape the heap to exploit it |

`Allocator` is the safe default. `AllocatorFlex` is about 3x cheaper per row, but it is only
sound if the guest has no memory bugs (overflows, use after free).

## Two strategies for `AllocatorFlex`

Both use the same AIR and pick the best fit. They differ only in when free blocks are joined.
Every executor (emulator, assembly, GPU) has to apply the same strategy so that they return the
same addresses.

- **Eager:** joins on every `dealloc`. The heap is identical to `Allocator`'s, but each join
  adds a row: +26% to +44% rows on the guests measured.
- **Lazy:** joins only when an `alloc` finds no block that fits. On the guests measured no `alloc` ever
  needed it, so there were no join rows.
  - Clean memory stays apart from the freed (dirty) blocks, so most `calloc` calls need no memset
    (reth: 5.5k memsets instead of 46k).
  - The cost is a slightly larger heap (+3.5% on the dlmalloc guest), and a burst of join rows if the heap
    ever gets fragmented.

## Measured and estimated savings

These numbers come from ziskemu:

- reth uses a guest built with the precompile.
- A large guest using dlmalloc, with 2-3.4M allocator calls per run, is replayed from the calls
  of its dlmalloc build, so its numbers are estimates.

The precompile cost is modelled as rows × columns, plus one main step per operation, plus the
memsets and the realloc copies. On reth the estimate came within 8% of the measured cost.

| Guest | Baseline | Steps | Cost | Heap | Rows (instances of 2^20) |
|---|---|---:|---:|---|---:|
| reth, 15 mainnet blocks (measured) | bump | −0.4% | −1.3% | −30% (100 → 70 MB) | ≤ 0.83M per block (1) |
| reth, 15 mainnet blocks (measured) | dlmalloc | −17.9% | −13.3% | ≈ same | ≤ 0.83M per block (1) |
| Large dlmalloc guest, small run (estimated) | dlmalloc | −24.8% | −15.7% | ≈ same | 1.35M (2) |
| Large dlmalloc guest, large run (estimated) | dlmalloc | −14.5% | −11.3% | ≈ same | 3.61M (4) |

- On the dlmalloc guest, the precompile costs 5-8% of what dlmalloc costs today (1.9G vs 24G on
  the large run). The choice of variant only moves the total by about 0.2 points.
- Proving time roughly follows cost, so expect about 11-16% less proving time on guests that use
  a real allocator (dlmalloc, talc, tlsf) and about 1% less on guests that already use a bump
  allocator. Execution (emulation) time follows steps: 14-25% less on the dlmalloc guest.
- With N = 2^22, `Allocator` fits every run in one instance (the largest run fills 86% of it).
  `AllocatorFlex` eager needs two instances for the largest runs of the dlmalloc guest.

## Pending, to make it provable

1. **Choose the variant** (`Allocator` or `AllocatorFlex` + strategy) and the instance size N.
2. **Wiring:**
   - Add real opcodes and the operation bus in `zisk.pil`, `operations.pil` and `opids.pil`.
     Today the operations travel through the allocation mark CSR `0x822`, transpiled to a
     `copyb` that only ziskemu understands.
   - Make the transpiler emit those opcodes.
3. **Executors:**
   - Add the operations to the assembly emulator.
   - Implement the same heap policy in every executor, including GPU, with bit-identical
     addresses.
4. **Witness generation:**
   - Write the state machine and its planner (instance counting).
   - Witness the "next free block" values that `Allocator` needs.
   - Run `verify-constraints` on the AIRs; it has never run on them.
5. **`calloc` memset:**
   - Check how `OP_DMA_XMEMSET` at the same main step interacts with memory timestamps.
   - Make sure the memory planners count these memsets.
6. **ziskos:**
   - Settle `precompile_alloc.rs` as the guest allocator.
   - Decide whether to add an in-place `realloc` operation. It would save 28-52% of the realloc
     copy cost.
   - Alignments above 8 are handled by the wrapper today.
7. **Open design points:**
   - Which `create` arguments to enforce (alignment, no overlap, zeroed memory).
   - One free list per area.
   - Sharing columns in `Allocator`.
   - The step range limit.
   - See *Open items* in AGENTS.md.
8. **Formal verification** of the guarantees, following the outline in AGENTS.md.
