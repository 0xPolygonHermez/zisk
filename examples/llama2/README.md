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
| `--scales float\|fixed` | `float` | How matrix multiplications apply the group scales: in `f32` like llama2.c, or in fixed point (see [below](#with-fixed-point-scales)) |

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

### The native program, step by step

[`llama2-native`](host/src/bin/native.rs) is the shortest path through the code, so it is a good place to start reading. Its `main()` does five things:

1. **Reads the options** (`parse_options`): which model to use, the prompt, how many tokens to generate, and how to pick them.
2. **Loads the model and the tokenizer** (`load`).
   - **The model** is a long list of numbers, its *weights*, learned during training. The llama2.c checkpoint stores them as 32-bit floats. `load` converts them to 8-bit integers plus one float *scale* per group of values. This is called *quantization*, and it is the format the ZisK guest uses. The result is a single byte buffer, the *model blob*.
   - **The tokenizer** converts between text and *tokens*. A token is a word or part of a word, identified by a number. The model only reads and writes these numbers.
3. **Parses the model** (`Model::parse`): it reads the sizes and finds each weight matrix inside the blob, without copying the int8 weights.
4. **For each prompt**, from `--prompt` or typed by the user:
   - **Builds the request** (`gen_params`): the prompt as tokens, and the settings. The first token is BOS ("beginning of sequence", here the start of a story). Temperature 0 always picks the most likely next token.
   - **Starts printing** (`TextPrinter`): it prints the prompt, and then each new token as text as soon as it is generated.
   - **Chooses how to compute** (`MatmulMode`). Almost all of the work is matrix multiplications. Natively they use the plain-Rust integer dot product, with the scales in `f32` or fixed point (`--scales`). The ZisK guest can use the `dot_i8` precompile instead.
   - **Generates** (`generate_with`, in [`generate.rs`](common/src/generate.rs)). For each position, it runs the model once on the current token: a *forward pass* ([`forward`](common/src/forward.rs)). That gives a score for every token of the vocabulary as the possible next one. During the prompt, the next token is simply the next prompt token. After the prompt, the next token is chosen from those scores and fed back in. Generation stops after `--max-new` tokens, at a BOS (a new story would start), or when the model's context is full.
5. **Prints the timing**: the prompt and generated token counts, the time, and the tokens per second.

`llama2-zisk` does the same steps 1 to 4, except that the guest runs the generation on ziskemu. The host then also runs it natively to check that the tokens match.

### How the whole prompt is taken into account: the KV cache

Each forward pass takes a single token as input. The new tokens still depend on the whole prompt, and on everything generated so far, because the model keeps a record of every token it has seen:

1. **Every prompt token goes through the model.** [`generate_with`](common/src/generate.rs) calls `forward` once per position, including every prompt position. During the prompt, the scores it produces are ignored, because the next token is simply the next prompt token. Those passes matter for what they leave behind.
2. **Each pass stores a summary of its token, in every layer.** These are a *key* and a *value*: vectors computed from the token, in the context of everything before it. [`forward`](common/src/forward.rs) stores them in the *KV cache* (`key_cache` and `value_cache`) at that token's position.
3. **Attention looks back at all earlier positions.** In every layer, the current token's *query* is compared with the key of every position from 0 to `pos`. The resulting weights then mix the stored values. This is how each position's output depends on the whole sequence so far.

So when the first new token is chosen, the input of the forward pass is the last prompt token. But in each layer, attention reads the keys and values of all the prompt tokens. Each generated token is added to the cache the same way.

- **The context limit is the cache size.** The cache has room for `seq_len` positions: 512 for `stories260K` and 256 for `stories15M`. Generation stops when it is full.
- **This is why the steps per token grow along the sequence** (see [End to end](#end-to-end)): attention at position `pos` reads `pos + 1` cache entries.

The cache saves time without changing the result. Without it, the model would have to run every previous token again at each step to rebuild the same keys and values.

### The layers

The model is a stack of layers, each with its own trained weights: 5 in `stories260K` and 6 in `stories15M`. A layer is one round of "look at the context, then process what was found". All of it is in [`forward`](common/src/forward.rs).

**The vector that flows through the layers.** The model represents the current token as a vector `x` of `dim` numbers: 64 in `stories260K`, 288 in `stories15M`.

1. **Embedding.** At the start, `x` is read from a table with one row per vocabulary token, so it depends only on the token.
2. **Each layer adds a correction to `x`** instead of replacing it (`x += …`). What earlier layers found is kept, and each layer refines it. This running vector is often called the *residual stream*.
3. **At the end**, `x` is normalized and multiplied by the classifier matrix (`wcls`). That gives one score (*logit*) per vocabulary token.

Over the layers, `x` goes from "this is the word *upon*" to something like "this is the position after *Once upon a*, and the next word should be *time*".

**Inside one layer, there are two blocks.**

1. **Attention: collect information from other positions.** This is the only place where tokens see each other.
   - `rmsnorm` rescales `x` to a standard size, so the numbers stay in a stable range from layer to layer.
   - Three matrix multiplications turn `x` into a *query* (`wq`: what this position is looking for), a *key* (`wk`: what it offers) and a *value* (`wv`: what it hands over if selected).
   - RoPE rotates the query and the key by an angle that depends on the position, so comparing them also takes into account how far apart two tokens are.
   - The key and value go into the [KV cache](#how-the-whole-prompt-is-taken-into-account-the-kv-cache). The query is compared, with a dot product, with the key of every position so far.
   - `softmax` turns those scores into weights that add up to 1, and the output is the weighted mix of the values. A position about to write a pronoun, for example, can put most of its weight on the position that holds the character's name.
   - This is done in parallel by several *heads* (6 in `stories15M`), each on its own slice of the vector, so each can learn a different kind of lookup. In `stories260K`, pairs of heads share their keys and values to save memory.
   - One more matrix (`wo`) mixes the outputs of the heads, and the result is added to `x`.
2. **Feed-forward: process each position on its own.**
   - `x` is normalized again.
   - Two matrices (`w1`, `w3`) expand it to a wider vector of `hidden_dim` values (768 in `stories15M`).
   - `silu(w1·x) * (w3·x)` lets one of them act as a soft on/off switch for the other. Besides softmax, this is the only non-linear step, and it is what lets the model do more than linear algebra.
   - A third matrix (`w2`) brings the vector back to `dim`, and the result is added to `x`.

A common simplification: attention is "communicate", and feed-forward is "think". Much of what the model knows about language and stories is stored in the feed-forward weights.

**Why stack several layers.** All layers run the same code, but each has its own weights, so each learns a different job. Nobody assigns those jobs; they come out of training. Roughly, earlier layers handle local things, such as grammar and word pieces, and later ones handle meaning and the plot. Each layer reads what the earlier ones added to `x`, so it can build on their results. More layers and a wider `x` make a more capable model, at a proportional cost.

**What this means for ZisK.** Each layer does 7 matrix multiplications (`wq`, `wk`, `wv`, `wo`, `w1`, `w2`, `w3`). With the final classifier, these are the matmuls that dominate the step counts below. Attention, softmax, SiLU and RoPE are most of the remaining float code.

### The model: int8 like llama2.c's `runq.c`

[`common/`](common/src/lib.rs) is a Rust port of the int8 path of llama2.c (`runq.c`):

- When loading, the host **quantizes** the float32 checkpoint: each matrix row is split into groups of up to 64 values, and each group is stored as int8 values plus one `f32` scale (`max|w| / 127`).
- Every **matrix multiplication** quantizes its input vector the same way and computes an `i32` dot product per group, which is then multiplied by the two scales. These integer dot products are the same kind of loop as in the [`mlp`](../mlp/) example. With `--scales fixed`, the scales are applied in fixed point instead (see [below](#with-fixed-point-scales)).
- **RMSNorm, attention, softmax, SiLU and RoPE** use `f32`.

The guest target (`riscv64ima`) has no floating-point instructions, so the compiler turns `f32` operations into calls to software routines made of integer instructions. Those can be executed and proven by ZisK like any other code, just slowly. Basic IEEE operations are correctly rounded both natively and in those routines, and `exp`, `sin`, `cos`, `pow` and `sqrt` come from the `libm` crate on both sides. So the native run and the ZisK run produce **the same tokens**, which `llama2-zisk` checks every time.

### Guest input and output

| | Contents |
|---|---|
| Input record 1 | The quantized model blob, used in place by the guest (layout in [`checkpoint.rs`](common/src/checkpoint.rs)) |
| Input record 2 | The generation parameters: prompt tokens, `max_new`, temperature, seed ([`params.rs`](common/src/params.rs)) |
| Input record 3 | Guest options, one `u64` of flags. Bit 0 selects the `dot_i8` precompile, bit 1 fixed-point scales. |
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

### With fixed-point scales

With `--scales fixed`, step (B) uses integers instead of software floats ([`fixed.rs`](common/src/fixed.rs)).

**How it works.** The scales of each matrix row, and those of each quantized vector, become 15-bit integers that share one power-of-two exponent. The largest scale becomes an integer between 2^14 and 2^15:

```text
w_scale[g] ≈ w_fixed[g] · 2^-w_shift      x_scale[g] ≈ x_fixed[g] · 2^-x_shift
out[i]     = 2^-(w_shift + x_shift) · Σ_g ival[g] · w_fixed[g] · x_fixed[g]
```

- The sum is exact in `i64`. Each term is below 2^44 per value, so rows of up to 2^19 values cannot overflow.
- Only the final sum of each row becomes an `f32`, with one conversion (`__floatdisf`). Dividing by the power of two then only changes the float's exponent bits.
- The weight scales are converted when the model is loaded, and the activation scales in `quantize`. Both conversions work on the `f32` bits with integer instructions only.
- The native run and the ZisK run still produce the same tokens.

Loading always converts the weight scales, even with `--scales float`. That adds about 1.4 G of cost for `stories15M`, so the `float` numbers below are up to 1.3% above those in the previous sections.

#### Accuracy

The scales lose precision: each is rounded to 2^-15 of the largest scale it shares an exponent with. That is far below the error of the int8 quantization itself, about 2^-8 of each group's largest value. But greedy generation is sensitive to any change. When the two best tokens score almost the same, a tiny difference in the logits picks the other one, and the story then goes another way. With the prompts below, the `float` and `fixed` texts are the same for the first 60 to 700 characters, and then differ.

To measure the difference, I fed both modes the same tokens: the prompt, then the `float` choice at each position (teacher forcing). I then compared the logits at every position. As a baseline, I did the same between `float` and a `float` variant that only adds the groups in reverse order. Its results differ only by `f32` rounding.

5 prompts ("Once upon a time", "Lily and Tom went to the park", "The little dog was sad because", "One day, a bird", "There was a big red ball"):

| | Positions | Top-1 token differs | Largest logit difference | Mean of (largest difference / logit range) |
|---|---|---|---|---|
| `stories15M`: `fixed` vs `float` | 1,000 | 13 (1.3%) | 0.68 | 1.0% |
| `stories15M`: reversed `float` vs `float` | 1,000 | 12 (1.2%) | 0.46 | 0.75% |
| `stories260K`: `fixed` vs `float` | 1,280 | 15 (1.2%) | 0.56 | 0.85% |
| `stories260K`: reversed `float` vs `float` | 1,280 | 0 | 0.34 | 0.05% |

1. **For `stories15M`, fixed point is about as close to `float` as `float` is to itself.** Reordering the float additions changes the top-1 token about as often. Each disagreement is a near tie: the `float` scores of the two tokens differ by at most 0.15.
2. **The differences come from the int8 quantization of the activations, not from the scales.** A tiny change can move a value to the next int8 step, and that grows from layer to layer, whatever caused the change.
3. **The `stories260K` baseline is not comparable.** Most of its rows have a single group, so reversing the order changes little. Its `fixed` results are close to the `stories15M` ones.

Neither mode is more exact than the other in a meaningful way. Both are int8 approximations of the float model.

#### Results

End to end, with `llama2-zisk` and the prompt "Once upon a time". Every run produces the same tokens as the native run in the same mode, and the committed hash matches:

| Model | Tokens | `float` `soft` | `float` `precompile` | `fixed` `soft` | `fixed` `precompile` |
|---|---|---|---|---|---|
| `stories260K` | 5 + 80 | 1.101 G steps, 23.9 s | 0.986 G, 22.3 s | 0.868 G, 19.2 s | **0.753 G, 17.7 s** |
| `stories15M` | 5 + 20 | 5.26 G steps, 92.9 s | 2.77 G, 58.5 s | 3.37 G, 53.8 s | **0.888 G, 19.4 s** |

Cost profiles, with `-X --sdk --top-functions` and the same runs as in [Where the steps go](#where-the-steps-go):

| | Steps | Cost | Main | Opcodes | Precompiles | Memory |
|---|---|---|---|---|---|---|
| `stories15M` `float` `soft` | 2.171 G | 229.4 G | 134.6 G | 79.1 G | ~0 | 15.5 G |
| `stories15M` `float` `precompile` | 1.135 G | 111.5 G | 70.3 G | 36.7 G | 1.05 G | 3.1 G |
| `stories15M` `fixed` `soft` | 1.386 G | 153.8 G (−33%) | 85.9 G | 52.0 G | ~0 | 15.5 G |
| `stories15M` `fixed` `precompile` | **0.350 G** | **35.8 G (−68%)** | 21.7 G | 9.6 G | 1.05 G | 3.2 G |
| `stories260K` `float` `soft` | 216.3 M | 22.02 G | 13.41 G | 7.49 G | ~0 | 0.84 G |
| `stories260K` `float` `precompile` | 183.4 M | 18.22 G | 11.37 G | 6.11 G | 0.03 G | 0.43 G |
| `stories260K` `fixed` `soft` | 149.7 M | 15.60 G (−29%) | 9.28 G | 5.18 G | ~0 | 0.85 G |
| `stories260K` `fixed` `precompile` | **116.8 M** | **11.80 G (−35%)** | 7.24 G | 3.80 G | 0.03 G | 0.45 G |

The percentages compare each `fixed` row with the `float` row that uses the same dot product.

Where the cost goes for `stories15M` with `fixed` and `precompile`:

| Share of the cost | `stories15M` `fixed` `precompile` |
|---|---|
| `matmul`, without the dot products: the loop, the integer scaling and one `f32` conversion per row | ~36% |
| Calling `dot_i8`: the parameter block and the call (`dot_precompile`) | ~21% |
| `expf`, in softmax and SiLU | 11% |
| `quantize`, of the activations | 8.7% |
| Loading the model (`Model::parse`), including converting the weight scales | 7.5% |
| The rest: RMSNorm, attention, RoPE, all in `f32` | ~16% |

1. **Combined with the precompile, fixed point makes `stories15M` 6.4 times cheaper than plain code.** The cost drops from 229.4 G to 35.8 G, and end to end it takes 5.9 times fewer steps. Each improvement alone cuts the cost by a third (fixed point) or by half (the precompile). Together they remove both halves of the matmul.
2. **There is no longer a single bottleneck.** What remains is the matmul loop around the calls (~36%), the calls themselves (~21%) and the other `f32` code (~36%: `expf`, `quantize`, RMSNorm, attention, RoPE).
3. **The calls are now worth reducing.** A row-level precompile, which computes a whole row of groups per call, scales included, would remove most of the loop and the call costs, about 57% of the cost here.
4. **Loading the model is now 7.5% of the cost** of this short run, half of it for the weight scales. It is paid once per run, so it matters less with more tokens. It could also be removed by storing the fixed-point scales in the blob.

## Next steps

1. **A row-level `dot_i8`.** With fixed-point scales, a precompile that computes `Σ_g dot(w_g, x_g) · w_fixed[g] · x_fixed[g]` for a whole row would replace the loop and most of the calls, about 57% of the remaining cost of `stories15M`. It needs the same state machine work as `dot_i8`.
2. **Fixed point in the rest of the model.** RMSNorm, softmax, `expf`, SiLU, RoPE and `quantize` still use software floats, about 36% of the remaining cost. Moving them to fixed point would make the model fully integer, which a proof-friendly model needs anyway. Accuracy has to be measured as above.
3. **Make `dot_i8` provable.** It needs a state machine (the PIL, witness generation, the memory inputs and the planner), as described in the [`mlp` README](../mlp/README.md#dot-product-precompile). That would also replace the estimated costs above with measured ones.
4. **Bind the model** in the commitment with a hash of the blob, using the SHA-256 or Keccak precompile. Storing the fixed-point scales in the blob would also remove their conversion when loading.
5. **Prove** a short generation with `cargo-zisk prove`, and measure proving time per token.
