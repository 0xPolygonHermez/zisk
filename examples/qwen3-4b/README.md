# Qwen3-4B on the ZisK emulator

The example replays a saved transcript using the original BF16 weights of Qwen3-4B-Instruct-2507. Its 28 prompt tokens and first 11 reply tokens supply 39 input positions. The head predicts the 12 reply tokens at positions 27–38. The runner checks the greedy choice at each position against the saved reply.

## How the checkpoint fits

[prepare.rs](host/src/prepare.rs) reads the checkpoint shards, verifies the tensor inventory and writes the inputs for 112 separate ZisK executions. [run.rs](host/src/run.rs) runs the same [guest](guest/src/main.rs) for each stage. Every invocation has a 256 MiB input limit.

| Stage | Executions | Input per execution on this transcript | Output |
|---|---:|---:|---|
| Embedding | 1 | 222,416 bytes | One embedding vector for each of the 39 positions |
| Decoder | 36 | 202,261,176 bytes (about 193 MiB) | The next layer’s 39 × 2,560 activations |
| Vocabulary head | 75 | 2,370,912–10,890,592 bytes | Logits for 12 positions over one vocabulary slice |

Each decoder receives one layer’s weights and the preceding guest’s actual output. This boundary is 399,360 bytes because each activation travels as four little-endian FP32 bytes. At halt the guest publishes the address and length of its output buffer. The emulator copies that buffer to a file, and the runner places its contents into the next input. All 75 head jobs consume the final decoder output. The runner assembles their logits in token-ID order before taking the argmax, so the lowest token ID wins ties.

## Reproduce the saved request

Use Linux with the installed ZisK guest toolchain and runtime archive, the repository's native build prerequisites, and curl for the checkpoint download. Allow about 8 GB each for the checkpoint and the prepared inputs, plus build and run output. From the repository root in Bash:

```sh
root=$PWD
example=$root/examples/qwen3-4b
work=$root/target/qwen3-benchmark
mkdir -p "$work/checkpoint"
checkpoint_url=https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507/resolve/cdbee75f17c01a7cc42f958dc650907174af0554
for file in config.json model.safetensors.index.json model-0000{1,2,3}-of-00003.safetensors; do
  curl --fail --location --retry 3 "$checkpoint_url/$file" -o "$work/checkpoint/$file" || exit 1
done
cargo build --release --locked --manifest-path "$example/host/Cargo.toml"
host=$example/host/target/release/qwen-host
"$host" prepare "$work/checkpoint" "$work/records"
cargo build --release --locked -p ziskemu
(
  cd "$example/guest"
  RUSTFLAGS="--cfg zisk_guest -C target-feature=+unaligned-scalar-mem -C link-arg=-T$root/ziskbuild/zisk_linker_script.ld -L native=$HOME/.zisk/bin" \
    cargo +zisk build --release --locked --target riscv64ima-zisk-zkvm-elf
)
"$host" run --records "$work/records" --emulator "$root/target/release/ziskemu" \
  --elf "$example/guest/target/riscv64ima-zisk-zkvm-elf/release/qwen3-4b-guest" --output "$work/run"
```

To check all 112 prepared files against the frozen input digests, run `QWEN_RECORDS="$work/records" cargo test --locked --manifest-path "$example/host/Cargo.toml" --test records -- --ignored`.

The run directory contains per-job logs, public outputs, captured tensors, cost CSVs and a summary with total guest steps, emulator cost, greedy token IDs and the logit digest. To compare every logit with a separate reference, pass `--reference-logits FILE` containing 12 × 151,936 little-endian FP32 values in position-major order.

The original 112-execution run matched all 12 greedy IDs and produced a logit SHA-256 of `4fdefa2a36a47fddc5fc767007270b2cfaaf18536647085a24faf5a84066066c`. The BF16 precompile sums exact products before rounding to FP32, whereas the pinned reference accumulates in ordered FP32 lanes. This accounts for differences in scores even when the winning token agrees. Emulator counters depend on the compiled guest. See the [full benchmark table](../../precompiles/dot_bf16/README.md#qwen3-4b-emulator-benchmark) and the [matched tile comparison and reproduction commands](tile/README.md).
