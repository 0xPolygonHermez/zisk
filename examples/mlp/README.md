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
