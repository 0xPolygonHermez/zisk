# Plan: a `dot_bf16` precompile

Status: steps 1–7 are implemented (emulator only), and step 8 has started: the llama2 example runs with BF16 weights and activations (`--weights bf16`), with its results in the [llama2 README](../../examples/llama2/README.md#bf16-with-dot_bf16). The reference, `ziskemu` and the assembly emulator pass the guest test program with the same step count, and the assembly emulator's C kernel matches the reference on 300,000 random cases.

## Goal

Run models whose weights and activations are BF16 (SmolLM2, Qwen3-0.6B) with their matrix multiplications as precompile calls, as `dot_f8` does for int8, FP8 and FP4. The operation must be deterministic and fully specified, so that `ziskemu`, the assembly emulator, a native reference and, later, a state machine all agree bit for bit.

Like `dot_f8`, the first version is **emulator only**: it runs in `ziskemu` and the assembly emulator with an estimated cost, and a state machine comes later.

## Semantics

### Inputs

The guest passes the address of a parameter block of six `u64`, laid out like `dot_f8`'s so that the guest code and the emulators can share patterns:

| Word | Meaning |
|---|---|
| `x` | address of the first vector, 8-byte aligned |
| `y` | address of the second vector, 8-byte aligned |
| `n` | number of values in each vector: a multiple of 4 (4 BF16 per 64-bit word), at most 2^20 |
| `flags` | 0 for two BF16 vectors; 1 and 2 select MXFP4 or NVFP4 weights in `x` (see [`fp4.md`](fp4.md)) |
| `block` | values per block, a multiple of 4 dividing `n`; 0 for a single block of `n` values |
| `out` | when not 0, receives the result of each block as `f32` bits in a `u32` (`n / block` of them), 8-byte aligned |

With `n` = 0 nothing is read or written, and any addresses are accepted: Rust's empty slices have dangling, unaligned pointers.

Each value is a BF16, little-endian: 1 sign bit, 8 exponent bits (bias 127), 7 mantissa bits.

### Result

`c` = the `f32` bits (zero-extended to 64 bits) of **the exact sum `Σ x[i]·y[i]`, rounded once to `f32`, to nearest, ties to even**.

- **Products and sums are exact.** A BF16 product has a 16-bit significand and an exponent from 2^-266 to 2^256, so the sum fits in a fixed-point accumulator of about 550 bits (576 in practice). There is no intermediate rounding and no intermediate overflow. The result therefore doesn't depend on the order of the additions, which is what makes it reproducible and checkable.
- **Subnormal inputs are kept** (no flush to zero). An exact accumulator handles them at no extra cost.
- **Single rounding at the end:** standard round-to-nearest-even to `f32`, including `f32` subnormals, overflow to ±∞, and underflow to 0.
- **Zero:** an exact sum of 0 gives +0.0, whatever the signs of the products. This is a simplification of IEEE's signed-zero rules.
- **Infinities and NaN:** if any input value is ±∞ or NaN, the result is the canonical quiet NaN `0x7FC00000`. Models never contain them, and this keeps the state machine simple. It departs from IEEE, which would for example give ∞ for ∞·1.
- **Blocks:** each block's result is its own exact sum rounded once. The total `c` is the exact sum of all `n` products rounded once, which is not the sum of the rounded blocks.

Why this rather than an `f32` accumulator, like GPUs and CPUs use:

| | Exact sum, rounded once (proposed) | `f32` sum in a fixed order |
|---|---|---|
| Result | the correctly rounded dot product | depends on the order; we would have to fix one |
| Agreement with PyTorch/GPU results | equal or closer to the exact value; may differ in the last bit | may differ in the last bits (their order is not ours either) |
| Emulator code | a small multi-word accumulator | simple |
| State machine | a wide accumulator (~18 32-bit limbs) and a shifted add per product | one `f32` addition with rounding per product, which is heavy too |
| Specification | short: "exact, then round" | long: the order, every rounding, subnormals |

### What the guest does with it

For BF16 models there are no scales, so each output of a matrix multiplication is one call: `out[r] = dot_bf16(W[r], x)` with `block = 0`. That gives one call per row, as with `dot_f8`. Blocks are there for rows longer than a model needs, or for later variants.

## Opcode, syscall and cost

- **Opcode `0xc3`,** the next free one after `dot_f8` (`0xc3` to `0xcf` are free).
- **Syscall `0x823`.** Syscall IDs must stay contiguous and in the same order as the list in `riscv2zisk_context.rs`.
- **Minimal-trace header (`input_size`):** 7 words, the 6-word parameter block plus the result, followed by both vectors (`data_ext_len`), as for `dot_f8`.
- **Cost estimate, to replace once a state machine exists:**
  - one row per call, like DMA (`DOT_BF16_COST = DMA_COST`);
  - one row per 64-bit word of each vector, which is 4 multiply-adds;
  - the row cost is a placeholder of about 120 columns (`DOT_BF16_ROW_COST`). Each row holds 4 significand products of 8 by 8 bits, their exponents, the alignment of each product into the accumulator, and the accumulator limbs with their carries. That is about three times a `dot_f8` row for half the multiply-adds;
  - the vectors are charged as aligned 64-bit reads, and the block results as writes.

## Implementation steps

The same files as `dot_f8` (commit `df3c5a400`), in this order:

1. **Reference semantics, in `definitions/src/dot_bf16.rs`**, `no_std`:
   - BF16 decoding into sign, significand and exponent;
   - `bf16_from_f32` (round to nearest even, with NaN handled first) and `bf16_to_f32`;
   - an exact accumulator of 9 × 64-bit words, with one product added at its bit position;
   - rounding of the accumulator to `f32`;
   - `dot_bf16_reference(x, y, n, block, out) -> u32`.

   This is the specification, and every other implementation is tested against it.
2. **Syscall ID** `SYSCALL_DOT_BF16_ID = 0x823` in `definitions/src/syscall.rs`.
3. **Core:**
   - `core/src/operations/dot_bf16.rs`: parameter checks (alignment, `n`, `block`, `flags` = 0), the operation for `ziskemu`, and the cost model;
   - an entry in `core/src/zisk_ops.rs` (`DotBf16`, `0xc3`, `input_size` 56);
   - `ZiskOperationType::DotBf16` in `zisk_inst.rs`;
   - constants in `zisk_ops_costs.rs`;
   - the translation in `zisk_rom_2_asm.rs`.
4. **Transpiler:** the syscall name in `transpilers/riscv/src/riscv2zisk_context.rs`.
5. **`ziskos`:** `syscalls/dot_bf16.rs` with `SyscallDotBf16Params` and `syscall_dot_bf16`. Off the ZisK target it calls the reference, so native code gets the same results.
6. **Assembly emulator,** `emulator-asm/src/dot_bf16/`:
   - a C kernel with the same accumulator (64-bit words, carries deferred while they cannot overflow), and the final rounding;
   - assembly wrappers `dot_bf16_fast` and `dot_bf16_mtrace`, which save the registers like `dot_f8`'s;
   - the Makefile rules.
7. **Tests:** the basic tests below, then a guest test program, run on both emulators.
8. **Later, not in this plan:** the llama2 example with BF16 weights, SmolLM2-135M, and the state machine.

## Basic tests

### Unit tests of the reference (`definitions`)

In these vectors, `n` = 4 unless stated, and `y = [1, 1, 1, 0]` means four BF16 values. Every value listed is exactly representable in BF16.

| # | Case | `x` | `y` | Expected `c` (`f32` bits) | What it checks |
|---|---|---|---|---|---|
| 1 | Simple sum | 1, 2, 3, 4 | 1, 1, 1, 1 | 10.0 (`0x41200000`) | basic multiply-add |
| 2 | Exact cancellation | 2^100, 1, −2^100, 0 | 1, 1, 1, 0 | 1.0 (`0x3F800000`) | no intermediate rounding; an `f32` sum in order would give 0 |
| 3 | Final tie, to even | 1, 2^-24, 0, 0 | 1, 1, 0, 0 | 1.0 (`0x3F800000`) | 1 + 2^-24 is halfway between 1 and 1 + 2^-23; ties go to the even 1.0 |
| 4 | Final tie, to even, upward | 1, 2^-24, 2^-23, 0 | 1, 1, 1, 0 | 1 + 2^-22 (`0x3F800002`) | 1 + 3·2^-24 is halfway between `…01` and `…10`; the even one is `…10` |
| 5 | Sticky bit | 1, 2^-24, 2^-60, 0 | 1, 1, 1, 0 | 1 + 2^-23 (`0x3F800001`) | a tiny term breaks the tie upward; only an exact sum sees it |
| 6 | Underflow to zero | 2^-133 (smallest subnormal), 0, 0, 0 | 2^-133, 0, 0, 0 | +0.0 (`0x00000000`) | 2^-266 rounds to 0 |
| 7 | Smallest `f32` subnormal | 2^-75, 0, 0, 0 | 2^-74, 0, 0, 0 | 2^-149 (`0x00000001`) | rounding into `f32` subnormals |
| 8 | Overflow | BF16 max (`0x7F7F`), 0, 0, 0 | 2, 0, 0, 0 | +∞ (`0x7F800000`) | the final rounding overflows |
| 9 | No intermediate overflow | max, max, −max, 0 | 2, 1, 2, 0 | max as `f32` (`0x7F7F0000`) | 2·max + max − 2·max is exact |
| 10 | Signed zeros | −1, 0, 0, 0 | +0, 0, 0, 0 | +0.0 (`0x00000000`) | a zero result is always +0 |
| 11 | Infinity input | +∞, 1, 1, 1 | 0, 1, 1, 1 | NaN (`0x7FC00000`) | any ∞ or NaN input gives the canonical NaN |
| 12 | NaN input | NaN (`0x7FC1`), 0, 0, 0 | 1, 0, 0, 0 | NaN (`0x7FC00000`) | as above |
| 13 | Blocks | n = 8, block = 4: 2^100, 0, 0, 0, −2^100, 1, 0, 0 | all 1 | `out` = [2^100, −2^100], `c` = 1.0 | each block is rounded alone; the total is rounded from the exact total, not from the rounded blocks |
| 14 | Empty | n = 0 | | +0.0 | no values |
| 15 | Random, against an independent oracle | 10,000 vectors, n up to 4,096, exponents in [−8, 8], random signs and mantissas | | equal | see below |

**The oracle for test 15** doesn't use the reference's accumulator:
- with exponents in [−8, 8], every product times 2^30 is an integer below 2^62, so the sum is exact in `i128`;
- Rust's `i128 as f32` rounds to nearest, ties to even;
- scaling by 2^-30 is exact, since the results aren't subnormal in this range.

**Conversion tests:**
- `bf16_from_f32`:
  - round to nearest even, at ties and just past them;
  - values that round up to the next exponent;
  - `f32::MAX` and values that round to ∞;
  - NaNs with payloads only in the low 16 bits (they must stay NaN);
  - subnormals.
- `bf16_to_f32` round-trips every BF16 code exactly. That's an exhaustive test of 65,536 codes.

### Operation tests (`core`)

- **Parameter checks panic** with a clear message for each rule:
  - an unaligned `x`, `y`, block or `out`;
  - `n` not a multiple of 4, or above 2^20;
  - `block` not a multiple of 4 or not dividing `n`;
  - `flags` not 0.
- **The cost model** charges `n / 4` rows, the vector reads, and the block writes.

### Guest test program (`test-artifacts/programs/syscalls/dot_bf16`)

A guest in the style of the existing syscall tests (`syscalls/arith256`): it runs cases 1–14 through `syscall_dot_bf16` and `assert_eq!`s each result.
- It also runs a few longer vectors (n = 1,024 and 4,096, with blocks of 64) whose expected results it computes with the reference in the guest, so `ziskemu`'s and the assembly emulator's code are checked against it.
- It runs on `ziskemu`, and on the assembly emulator in the fast and minimal-trace modes: both must pass, with the same step count.

### Cross-checks between implementations

- **The assembly emulator's C kernel against the reference:** a small native test links the kernel and compares it with `dot_bf16_reference` on the random vectors of test 15, plus vectors with the full exponent range (the reference against itself on permuted inputs, since the exact sum doesn't depend on the order).
- **Order independence:** the result must not change when the products are permuted. A property test with random permutations.

## Risks and open questions

1. **The state machine is the expensive part.** An exact accumulator of about 550 bits, with a variable shift per product, may cost more columns than estimated. Once the semantics are fixed, it's worth sketching the AIR to check that the estimate is in the right range before building on it. If it's too expensive, the fallback is an `f32` accumulator in a fixed order, which would change the semantics above.
2. **Agreement with PyTorch.** Reference implementations sum in `f32` in an order of their own, so our logits will differ from theirs in the last bits. That's expected; accuracy checks should compare top tokens and logit differences, as the llama2 README does, not bits.
3. **The ∞/NaN and zero-sign simplifications** depart from IEEE. They are fine for models, but they must be documented in `ziskos`, so a guest that needs IEEE behaviour checks its inputs.
4. **The largest `n`.** 2^20 values keeps a single step's minimal-trace data at 4 MB at most. Rows of the target models are at most a few thousand values, so it could be lower.
5. **Mixed inputs.** BF16 weights times `f32` activations is a common combination. `flags` is reserved for it, so a later version can add it without changing the interface.
