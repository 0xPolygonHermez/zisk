# MLP Inference Example

A guest program runs one inference of a small integer-only neural network (a multilayer perceptron) and commits the predicted class and the logits. The host builds the same model deterministically, runs it natively, and checks that the proven result matches. It is meant for measuring how ZisK's proving cost grows with model size.

## Overview

```text
input: model blob (i8 weights, i32 biases) + input vector (i8)
   →  dense → ReLU → … → dense
   →  (class: u32, logits: Vec<i32>)   (committed public output)
```

- **Integers only.** Weights are `i8`, accumulators are `i32`, and hidden activations are `relu(acc >> shift)` saturated to `0..=127`. There is no floating point, so the guest and the host give bit-identical results.
- **Deterministic.** The weights and the input come from a fixed-seed PRNG (`DEFAULT_SEED`), so there are no data files and every run with the same widths is identical. The weights are random, so the predicted class means nothing. Only the cost does.
- **No copies.** The guest reads the model as one raw input slice and uses the weights in place. The step count measures the network, not deserialization.

## How it works (for non-ML readers)

### What a neural network is

A neural network is like a large calculator with fixed knobs:

- It takes a list of numbers as **input**. By default that is 784 numbers, the size of a 28×28 grayscale image with one number per pixel.
- It returns a short list of numbers as **output**: 10 scores, one per possible answer (for example "this is a 0", …, "this is a 9").
- The answer is the index of the **highest** score.

The knobs are called **weights**. In a real model, training sets them so the outputs make sense. Here they are random, because this example only measures how much work the computation takes, not whether the answer is meaningful.

### Layers

The network is a chain of **layers**. With the default widths `784 128 10`:

```text
784 input numbers  →  layer 1  →  128 numbers  →  layer 2  →  10 scores
```

For each of its outputs, a layer:

1. multiplies every input number by its own weight;
2. adds all those products together, plus a constant (the **bias**);
3. cleans up the result: negative values become 0 and large values are capped at 127. (This is the **ReLU** step, which lets the network do more than plain arithmetic.)

The last layer skips step 3 and returns its raw totals as the scores.

Layer 1 alone does 784 × 128 ≈ 100,000 "multiply two numbers and add to a running total" operations. This operation is called a **multiply-accumulate (MAC)**, and it is almost all the work. A bigger model simply does more MACs.

### Why whole numbers only

Most ML uses floating-point (decimal) numbers. This example uses whole numbers only: weights and inputs are small integers from −128 to 127. There are two reasons:

- **Determinism:** integer arithmetic gives exactly the same result on every machine, so the host and the ZisK run can be compared bit for bit.
- **Cost:** floating point is expensive inside a zkVM.

Real models are often converted to small integers in the same way (this is called **quantization**), so this choice is realistic.

### What each part does

- **`common`** is the shared recipe, used by both the guest and the host so that they compute exactly the same thing:
  - `generate_model` builds the random weights from a fixed seed, so every run gets the same "random" model;
  - `generate_input` builds a fake 784-number "image", also from the seed;
  - `infer` runs the layers and returns the 10 scores;
  - `argmax` picks the index of the highest score.
- **`guest`** runs inside ZisK and is what gets proven. It reads the weights and the input, runs `infer`, picks the winner, and **commits** the winner and the scores as the public output of the proof.
- **`host`** runs on the normal machine and drives everything. It:
  1. builds the model and the input;
  2. computes the expected answer natively;
  3. asks ZisK to **execute** the guest, which reports the number of steps (RISC-V instructions);
  4. asks ZisK to **prove** the run, and times it;
  5. **verifies** the proof;
  6. checks that the proven answer equals the native one.

### What the proof means

Anyone with the proof can check, in a few milliseconds and without rerunning anything, that *this program, given this model and this input, produced these scores*. For AI this is the useful part: it shows that an answer really came from a given model and was not faked or altered.

The measurements below show that each MAC costs about 7 steps inside ZisK at every model size. So the cost of a model can be estimated as its MAC count × 7, before it is ever run.

## Structure

```text
mlp/
├── common/   # Model blob layout, parser, infer(), argmax(), deterministic generator
├── guest/    # Reads model + input slices, runs infer, commits (class, logits)
└── host/     # Builds model, executes (steps), proves, verifies, compares with native
```

## Host

Run from the `host/` directory. The arguments are the layer widths, input first. The default is `784 128 10`, an MNIST-sized network.

```bash
# Default model, execute + prove + verify
cargo run --release

# Count steps only (fast), for scanning sizes
cargo run --release -- 784 512 512 10 --execute-only

# ASM backend, ASM + GPU
cargo run --release -- 784 512 512 10 --asm
cargo run --release -- 784 512 512 10 --asm --gpu

# Write the guest input file for cargo-zisk
cargo run --release -- 784 128 10 --save-input ../guest/samples/example-input.bin
```

The host prints the parameter count, multiply-accumulates (MACs) per inference, steps, steps per MAC, and the setup, prove and verify times.

## Guest

Run from the `guest/` directory. `samples/example-input.bin` holds the default `784 128 10` model.

```bash
cargo-zisk build --release
cargo-zisk run --release -i samples/example-input.bin
cargo-zisk prove --release -i samples/example-input.bin --verify-proof
```

## Measured execution cost

Emulator, `--execute-only`:

| Widths | Parameters | MACs | Steps | Steps/MAC |
|---|---|---|---|---|
| 784 128 10 | 101,770 | 101,632 | 717,314 | 7.1 |
| 784 512 512 10 | 669,706 | 668,672 | 4,708,183 | 7.0 |
| 1024 1024 1024 10 | 2,109,450 | 2,107,392 | 14,803,799 | 7.0 |

Execution is linear in MACs at about 7 steps each, so steps can be predicted from the model's shape before you run it.

### Where the 7 steps per MAC come from

The inner loop of `Layer::neuron` in `common/src/lib.rs`:

```rust
for (&w, &a) in row.iter().zip(x) {
    acc += (w as i8 as i32) * (a as i32);
}
```

compiles to exactly 7 RISC-V instructions (from disassembling the guest ELF at `target/elf/riscv64ima-zisk-zkvm-elf/release/mlp-guest`):

```asm
loop:
  lb    t1, 0(a1)       # 1. load one weight byte (signed)
  lb    t2, 0(a4)       # 2. load one activation byte (signed)
  mul   t1, t2, t1      # 3. multiply them
  addi  a4, a4, 1       # 4. advance the activation pointer
  add   a0, t1, a0      # 5. add the product to the running total
  addi  a1, a1, 1       # 6. advance the weight pointer
  bne   a4, a6, loop    # 7. not at the end of the row yet? repeat
```

The transpiler ([`riscv2zisk_context.rs`](../../transpilers/riscv/src/riscv2zisk_context.rs)) turns each of these into exactly one ZisK instruction:

| RISC-V | ZisK |
|---|---|
| `lb` | one `signextend_b` reading from memory |
| `mul` | `mul` |
| `add`, `addi` | one add |
| `bne` | one `eq` with an inverted jump |

So one MAC costs 7 ZisK steps. Only 4 of them do the actual work (two loads, the multiply, the add). The other 3 are loop bookkeeping: two pointer increments and the branch.

**Why 7.0–7.1 and not exactly 7.** The remainder is overhead outside the inner loop:

- per neuron, a few dozen instructions: slice bounds checks, loading the bias, the shift, the `0..=127` clamp, and storing the output;
- once per run: allocation, reading the input, committing the output.

In the smallest model this overhead is spread over fewer MACs, which gives 7.1. In the 2 M-MAC model it drops to 7.02.

**Not all steps cost the same to prove.** ZisK sends different operations to different state machines: memory accesses to the memory machine, `mul` to the arithmetic machine, add and compare to the binary machine. So proving time is not just steps × a constant; the mix of operations matters too. Each MAC here produces two memory reads and one `mul`, which are probably the most expensive parts.

**Possible optimization.** Unrolling the loop to do, say, 4 MACs per iteration, with one bump per pointer and one branch per iteration, would bring the cost down to about (4 × 4 + 3) / 4 ≈ 4.75 steps per MAC, roughly 30% fewer. The compiler does not unroll it by itself here; it could be done by hand with `chunks_exact(4)`. The two loads, the multiply and the add cannot be removed without changing how the data is represented.

## Measured proving cost

Default emulator backend, CPU only, 128 cores:

| Widths | Steps | Prove | Verify | Peak RAM |
|---|---|---|---|---|
| 784 128 10 | 717,340¹ | 226 s | 2.7 ms | 45 GB |

¹ Measured on an earlier base branch, whose runtime used 26 more steps than this one. The difference is negligible for the timing.

At this size the proof is likely dominated by fixed costs, so the larger models are needed to see the real slope.

## Making it faster

Three ways to lower the cost of the inner loop, from cheapest to most effective:

| Option | Steps / MAC | Changes needed |
|---|---|---|
| Today | 7 | — |
| Unroll the loop ×4 | ≈ 4.75 | Guest code only (`chunks_exact(4)`) |
| Wider operand modes in ZisK instructions | 5–6 | Main state machine, transpiler |
| Dot-product precompile | ≈ 0 in Main; 1 precompile row per 8 MACs | New precompile |

### Wider operand modes

Today an indirect memory operand is only possible for `b` (and for the store of `c`), and its address is always `a + offset`. The register that holds the pointer therefore also takes the `a` operand slot, so an indirect load can only feed ops that ignore `a`, such as `copyb` or `signextend_b`. That is why each `lb` is a separate instruction.

- Letting `b` be read from `mem[reg + offset]` with a register *other than* `a` would allow `mul(r_w, u8[a4])`: 6 steps per MAC.
- Also allowing `a` to be read from memory would allow `mul(s8[a1], s8[a4])`: 5 steps per MAC.

The gain is limited. The two memory reads, the `mul` and the loop bookkeeping all remain. Only 2 Main rows and the 2 `signextend_b` operations go away. The cost of the change is also high: a second address register and memory read per instruction make *every* Main row wider in *every* program, and the transpiler would have to fuse `lb; lb; mul` sequences, which needs liveness and branch-target analysis.

### Dot-product precompile

A precompile only pays off when one call does a lot of work, so its fixed cost is spread out. A precompile for a single multiply-add would not help: each call is still at least one ZisK instruction plus argument setup, and the loop already does the multiply-add in 2 instructions (`mul`, `add`). The natural unit is the **dot product**, the whole inner loop `Σ w[i]·x[i]` over one row:

```rust
// guest side, one call per neuron
acc = bias + syscall_dot_i8(row.as_ptr(), x.as_ptr(), n);
```

**Circuit sketch.**

- Each row of the precompile's state machine reads one 64-bit word from each vector (8 weights and 8 activations), so 8 MACs per row.
- It splits each word into 8 bytes, range-checks them as signed bytes, and checks `acc' = acc + Σ a_i·b_i`, which is a single degree-2 constraint.
- The arithmetic fits easily in the Goldilocks field: each product is below 2^14, so even a 2^40-element vector cannot overflow. The exact sum taken mod 2^32 equals Rust's wrapping `i32` sum, so the semantics match the current code.

**Cost per 8 MACs:**

| | Today (8 × the 7-instruction loop) | Dot-product precompile |
|---|---|---|
| Main rows | 56 | ≈ 0 (one step per neuron, not per MAC) |
| Memory accesses | 16 byte-sized reads | 2 aligned 64-bit reads |
| Arith (`mul`) | 8 | 0 |
| BinaryE (`signextend_b`) | 16 | 0 |
| BinaryAdd + Binary | 32 | 0 |
| Precompile rows | 0 | 1 |

That is a 10–50× reduction in trace work for the hot loop. The actual gain in proving time has to be measured, on models large enough that fixed costs do not dominate.

**Starting point: DMA `memcmp`.** `memcmp` has the same shape: two variable-length buffers streamed from memory and reduced to one number. The [DMA precompile](../../precompiles/dma/README.md) already provides:

- the guest-side call mechanism (a CSR write plus a fused opcode; see `dma_memcmp`, `0xd1`, in [`zisk_ops.rs`](../../core/src/zisk_ops.rs));
- an aligned 64-bit path that handles up to 8 words per row and continues across rows ([`dma_64_aligned.pil`](../../precompiles/dma/pil/dma_64_aligned.pil));
- separate paths for unaligned head and tail bytes.

A `dot_i8` precompile would reuse most of this, with a different per-row constraint (sum of products instead of compare) and a different result. A first version can stay small by requiring 8-byte-aligned pointers and lengths that are a multiple of 8. The model blob would then pad each layer width and weight row to a multiple of 8 on the host side, which avoids the unaligned path.

**Work involved:**

1. A new opcode and syscall ID, the `ziskos` wrapper with a native fallback for the host, and the emulator implementation (including `emulator-asm`).
2. The PIL AIR, witness generation, and generation of the memory inputs (modeled on `dma_gen_memcmp_mem_inputs.rs`).
3. Integration with the planner and the instance builder.

### Larger models

- **The precompile still applies.** In a transformer almost all the work is matrix multiplication, which is dot products, so the same precompile covers it as long as the model is quantized to int8. If the model uses int4 or int16 weights, the precompile could take a width parameter; that is easy to design in from the start and harder to add later.
- **The bottleneck moves.** Once dot products are nearly free, the remaining operations dominate: softmax, layer normalization, activation functions and requantization, which stay as RISC-V code. A real model should be profiled before deciding what to accelerate next.

### Suggested order

1. Unroll the loop in this example: cheap, and possible today.
2. Prove the larger models (4.7 M and 14.8 M steps) to get the real growth rate of proving time.
3. Use those numbers to decide whether the `dot_i8` precompile is worth building.
