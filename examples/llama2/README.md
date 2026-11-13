# llama2 Example

Two programs that ask for a text prompt, continue it with a small language model, and print the result:

- **`llama2-native`** runs the model natively;
- **`llama2-zisk`** runs the same model as a ZisK guest on `ziskemu`, prints the same text, and checks it against a native run.

The models are Andrej Karpathy's [llama2.c](https://github.com/karpathy/llama2.c) **TinyStories** models (MIT license): Llama-2-architecture transformers trained to write short children's stories. They are small enough to run in a zkVM.

```text
Prompt (empty to quit)> Once upon a time
Once upon a time, there was a little girl named Lily. She loved to play outside in the park. One day,
she saw a big, red ball. She wanted to play with it, but it was too high. She asked her mom, "Can I
play with the ball?" Her mom said,
[ziskemu: 5 prompt + 80 generated tokens, 1083690952 steps, 22.70 s]
[committed hash matches printed tokens: yes; same tokens as native: yes]
```

## Setup

Download the models into `models/`, which git ignores:

```bash
mkdir -p models && cd models
H=https://huggingface.co/karpathy/tinyllamas/resolve/main
curl -L -o stories260K.bin $H/stories260K/stories260K.bin   # 1 MB
curl -L -o tok512.bin      $H/stories260K/tok512.bin
curl -L -o stories15M.bin  $H/stories15M.bin                # 61 MB
curl -L -o tokenizer.bin   https://raw.githubusercontent.com/karpathy/llama2.c/master/tokenizer.bin
```

| Model | dim | Layers | Heads | Vocabulary | Context | int8 weights |
|---|---|---|---|---|---|---|
| `stories260K` (default) | 64 | 5 | 8 (4 KV) | 512 (`tok512.bin`) | 512 | 259 K |
| `stories15M` | 288 | 6 | 6 | 32,000 (`tokenizer.bin`) | 256 | 15.2 M |

## Usage

Run from the `host/` directory (the default model path is relative to it):

```bash
# Native: asks for prompts until an empty line
cargo run --release --bin llama2-native

# On ziskemu (needs ziskemu in PATH, or --ziskemu PATH)
cargo run --release --bin llama2-zisk

# One prompt, the 15M model, 20 tokens
cargo run --release --bin llama2-zisk -- --model ../models/stories15M.bin \
    --prompt "Once upon a time" --max-new 20
```

Options, for both programs:

| Option | Default | Meaning |
|---|---|---|
| `--model PATH` | `models/stories260K.bin` | llama2.c checkpoint |
| `--tokenizer PATH` | `tok512.bin` or `tokenizer.bin`, next to the model | Chosen by vocabulary size |
| `--prompt TEXT` | ask interactively | Generate once for `TEXT` |
| `--max-new N` | 128 | Tokens to generate |
| `--temperature T` | 0 | 0 always picks the most likely token; higher values sample |
| `--seed S` | 42 | Sampler seed, used when `T > 0` |

`llama2-zisk` also takes:

- `--ziskemu PATH`: the `ziskemu` to run.
- `--dot-product soft|precompile`: how the guest computes the dot products. The default, `soft`, is plain Rust. `precompile` uses the experimental `dot_i8` precompile, which needs a `ziskemu` built from this repository and cannot be proven yet (see [below](#with-the-experimental-dot_i8-precompile)).
- `--save-input PATH`: writes the guest input for the first prompt and exits, to run the guest directly:

```bash
cargo run --release --bin llama2-zisk -- --prompt "Once upon a time" --max-new 20 --save-input /tmp/in.bin
ziskemu -e ../../target/elf/riscv64ima-zisk-zkvm-elf/release/llama2-guest -i /tmp/in.bin -X --sdk --top-functions
```

## How it works

### From text to text

```mermaid
flowchart TD
    prompt(["&quot;Once upon a time&quot;"])
    text([text])

    subgraph host1 [host]
        encode[tokenizer]
    end
    subgraph run [native or guest]
        model[model, one token at a time]
    end
    subgraph host2 [host]
        decode[tokenizer]
    end

    prompt --> encode
    encode -- "[1, …]" --> model
    model -- "[token ids]" --> decode
    decode --> text
```

1. **Tokenize (host).** The prompt becomes token ids with llama2.c's byte-pair-encoding tokenizer, starting with BOS (`1`).
2. **Generate (native or guest).** Each step runs one forward pass of the transformer on one token, producing a score (logit) for every vocabulary entry. During the prompt, the next input is the next prompt token. After it, the next token is the highest-scoring one (temperature 0) or a random draw weighted by the scores. Generation stops after `--max-new` tokens, at a BOS (the start of a new story) or when the context is full.
3. **Decode (host).** Token ids become text again.

### The model: int8 like llama2.c's `runq.c`

[`common/`](common/src/lib.rs) is a Rust port of the int8 path of llama2.c (`runq.c`):

- When loading, the host **quantizes** the float32 checkpoint: each matrix row is split into groups of up to 64 values, and each group is stored as int8 values plus one `f32` scale (`max|w| / 127`).
- Every **matrix multiplication** quantizes its input vector the same way and computes an `i32` dot product per group, which is then multiplied by the two scales. These integer dot products are the same kind of loop as in the [`mlp`](../mlp/) example.
- **RMSNorm, attention, softmax, SiLU and RoPE** use `f32`.

The guest target (`riscv64ima`) has no floating-point instructions, so the compiler turns `f32` operations into calls to software routines made of integer instructions. Those can be executed and proven by ZisK like any other code, just slowly. Basic IEEE operations are correctly rounded both natively and in those routines, and `exp`, `sin`, `cos`, `pow` and `sqrt` come from the `libm` crate on both sides. So the native run and the ZisK run produce **the same tokens**, which `llama2-zisk` checks every time.

### Guest input and output

| | Contents |
|---|---|
| Input record 1 | The quantized model blob, used in place by the guest (layout in [`checkpoint.rs`](common/src/checkpoint.rs)) |
| Input record 2 | The generation parameters: prompt tokens, `max_new`, temperature, seed ([`params.rs`](common/src/params.rs)) |
| Input record 3 | Guest options, one `u64` of flags. Bit 0 selects the `dot_i8` precompile. |
| Printed | `tokens: <id> <id> …`, the generated tokens |
| Committed | `sha256(parameters ‖ generated tokens)` |

A proof's public output is limited to 64 32-bit words, too small for a whole story. So the guest prints the tokens and commits their hash, and the host checks that the printed tokens match the committed hash. The model itself is not yet bound by the commitment. Hashing the 15 MB `stories15M` blob in software would add several hundred million steps; it is left for later, to be done with a hash precompile.

### Structure

```text
llama2/
├── common/       # checkpoint quantization, model, forward pass, generation, tokenizer
├── guest/        # reads model + parameters, generates, prints tokens, commits their hash
├── host/
│   ├── src/lib.rs          # options, loading, prompts, text output
│   ├── src/bin/native.rs   # llama2-native
│   └── src/bin/zisk.rs     # llama2-zisk: builds the guest input, runs ziskemu, checks
└── models/       # downloaded checkpoints and tokenizers (not in git)
```

## Measured cost

### Where the steps go

These are profiles from `ziskemu -X --sdk --top-functions`, with the prompt "Once upon a time" and the default `--dot-product soft`:

- `stories260K`, 20 generated tokens: **215.7 M steps**.
- `stories15M`, 6 generated tokens: **2.16 G steps**.

The profiler reports the cost of each function, including the routines it calls. Almost all the cost is in [`matmul`](common/src/forward.rs), which does two things per group:

```rust
for (g, (wg, xg)) in row.chunks_exact(gs).zip(xq.chunks_exact(gs)).enumerate() {
    let ival = dot(wg, xg);                            // (A) int8 dot product, ~7 steps per multiply-add
    val += ival as f32 * row_scales[g] * x.scales[g];  // (B) 4 software-float calls, ~170 steps per group
}
```

- **(A)** is the int8 dot product, `dot_soft`: the same loop as in the [`mlp`](../mlp/) example.
- **(B)** converts the integer sum to `f32` (`__floatsisf`) and does two multiplies (`__mulsf3`) and one add (`__addsf3`), each through a software routine.

So how much (B) costs depends on the group size. All shares below are measured. (B) is `matmul` minus `dot_soft`, so it includes the loop overhead.

| Share of the cost | `stories15M` | `stories260K` |
|---|---|---|
| `matmul` in total | **94%** | **60%** |
| (A) `dot_soft`, the integer multiply-adds | 55% | 24% |
| (B) the float step per group, and the loop | 39% | 36% |
| `expf`, in softmax and SiLU | 2% | 14% |
| `quantize`, of the activations | 1.4% | 7% |
| `rmsnorm`, attention and the rest | ~3% | ~19% |

1. **For `stories15M`, the main cost is the integer dot product (A).** This is the case that matters as models grow. Its groups have 32 values (`dim`) or 64 values (`hidden_dim`), so step (B) adds roughly 5 steps per multiply-add on top of the 7.
2. **The output classifier (`wcls`) is the largest matrix.** It gives a score for each of the 32,000 vocabulary entries. That is 9.2 M of the 15.2 M multiply-adds per token, about 60% of `matmul`, and it runs once per token.
3. **`stories260K` is skewed by its group size.** Its `hidden_dim` is 172, so `w2` gets groups of only 4 values. `w2` then has 81% of all groups, and step (B) runs once every 4 multiply-adds, outweighing (A). Padding `hidden_dim` to a multiple of 32 or 64 would avoid this.
4. **Attention and `expf` matter only for the tiny model.** Their cost grows with the position in the sequence, though, so they come back with long generations.

### End to end

These runs use `llama2-zisk`:

| Model | Tokens (prompt + generated) | ziskemu steps | Steps per token | ziskemu time | Native speed |
|---|---|---|---|---|---|
| `stories260K` | 5 + 80 | 1.08 G | 12.7 M | 22.7 s | ~3,000 tokens/s |
| `stories15M` | 5 + 20 | 5.11 G | 204 M | 81.6 s | ~240 tokens/s |

Steps per token grow with the position in the sequence, because attention looks at every earlier token, so the longer `stories260K` run has a higher average.

These numbers were measured before the dot product became a separate function (`dot_soft`, called through a function pointer so the guest can swap it). The current code takes 2–3% more steps in `soft` mode, because the loop is no longer inlined and its `i64` sum is saturated to `i32` at the end of each call (see `dot_soft`). The comparison below uses the current code for both modes.

### With the experimental `dot_i8` precompile

To measure what a dot-product precompile would save, ZisK has an experimental `dot_i8` operation. It is implemented in the operation table and the emulator only: **`ziskemu` runs it, but no state machine proves it, so a program that uses it cannot be proven yet.**

**What it does.** The guest passes the address of a block of three `u64`: the address of `x`, the address of `y` and the length `n`. It gets back `Σ x[i]·y[i]` over signed bytes, as a wrapping `i32` sum, which is what `dot_soft` computes. Both addresses must be 8-byte aligned and `n` a multiple of 8, so the planned state machine can read one 64-bit word of each vector per row, that is 8 multiply-adds per row.

The weight rows in the model blob are 8-byte aligned, and the guest stores the activations in `u64` words so they are too. Groups whose size is not a multiple of 8 use `dot_soft`. That only happens in `stories260K`'s `w2`, whose groups have 4 values.

**Cost model.** The cost report needs a cost for the new operation, but there is no state machine to measure, so these are estimates:

- each call costs one row, like a DMA call (`DOT_I8_COST = 63`);
- each 8 multiply-adds cost one row of about 40 columns (`DOT_I8_ROW_COST = 40`), which is close to `Dma64AlignedMem`'s 50;
- the parameter block and both vectors are charged as aligned 64-bit memory reads.

**How to run it.** Build `ziskemu` from this repository, then pass it to `llama2-zisk`:

```bash
cargo build --release --bin ziskemu        # at the repository root
cd examples/llama2/host
cargo run --release --bin llama2-zisk -- --ziskemu ../../../target/release/ziskemu --dot-product precompile
```

#### Results

Both modes produce the same tokens, which are also the native tokens, and the committed hash matches in every run.

End to end, with `llama2-zisk` and the prompt "Once upon a time":

| Model | Tokens | `soft` steps | `precompile` steps | Steps saved | `soft` time | `precompile` time |
|---|---|---|---|---|---|---|
| `stories260K` | 5 + 80 | 1.101 G | 0.985 G | **10%** | 24.0 s | 22.9 s |
| `stories15M` | 5 + 20 | 5.24 G | 2.76 G | **47%** | 93.1 s | 57.1 s |

Cost profiles, with `-X --sdk --top-functions` and the same runs as in [Where the steps go](#where-the-steps-go):

| | `stories15M` `soft` | `stories15M` `precompile` | `stories260K` `soft` | `stories260K` `precompile` |
|---|---|---|---|---|
| Steps | 2.157 G | 1.121 G (−48%) | 215.7 M | 182.8 M (−15%) |
| Cost | 228.0 G | 110.1 G (−52%) | 21.96 G | 18.16 G (−17%) |
| Main | 133.8 G | 69.5 G | 13.37 G | 11.34 G |
| Opcodes | 78.7 G | 36.3 G | 7.47 G | 6.09 G |
| Precompiles | ~0 | 1.05 G | ~0 | 0.03 G |
| Memory | 15.3 G | 2.9 G | 0.83 G | 0.42 G |

Where the cost goes for `stories15M` with the precompile:

| Share of the cost | `stories15M` `precompile` |
|---|---|
| (B) the float step per group (`__mulsf3` 42%, `__addsf3` 25%, `__floatsisf` 9%) | **~77%** |
| Calling the precompile: the parameter block and the call | ~6% |
| The `dot_i8` operation itself, in the Precompiles category | 0.9% |
| `expf` | 3.6% |
| `quantize` | 2.8% |
| The rest | ~10% |

1. **The precompile halves the cost of `stories15M`.** The integer multiply-adds were 55% of the cost and now cost about 7%, counting the calls. The operation itself is under 1%, even with its estimated cost per row.
2. **The float step per group is now the bottleneck.** It was 39% of the cost and is now 77%, at the same absolute cost. Each group now costs about 170 steps of software floats for (B), against a single precompile call for its 32 or 64 multiply-adds.
3. **`stories260K` gains little.** Its cost was mostly floats and `expf` to begin with, and 81% of its groups are in `w2`, with only 4 values each, so they still use `dot_soft`.
4. **Each call still costs Main steps.** The guest writes the parameter block to memory and makes the call, which is about 6% of the cost here. A variant that does a whole matrix row per call, writing one `i32` per group, would spread that over the row.

## Next steps

1. **Fixed-point instead of `f32`.** With the dot products in a precompile, the per-group float step (B) is 77% of the cost of `stories15M`. Doing (B), and then the norms, softmax, SiLU and RoPE, with integer fixed-point arithmetic would remove the software-float calls. This is needed anyway for a fully integer model. The cost is that the output would no longer match llama2.c's float math exactly, so accuracy must be measured.
2. **Make `dot_i8` provable.** It needs a state machine (the PIL, witness generation, the memory inputs and the planner), as described in the [`mlp` README](../mlp/README.md#dot-product-precompile). That would also replace the estimated costs above with measured ones.
3. **Bind the model** in the commitment with a hash of the blob, using the SHA-256 or Keccak precompile.
4. **Prove** a short generation with `cargo-zisk prove`, and measure proving time per token.
