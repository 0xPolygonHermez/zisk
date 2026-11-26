# Layer 0 key-projection tile

Run one ELF twice on rows 0–63 of the Qwen3-4B-Instruct-2507 key projection. Each mode computes 39 × 64 outputs of width 2,560. Mode 5 uses the pinned qproof lossless `dot_f8` kernel with ordered FP32 accumulation. Mode 6 uses the full guest’s `dot_bf16` projection, which rounds the exact sum once to FP32. Both round the final score to BF16 and hash its widened FP32 bits in position-major order.

Use Linux with the ZisK guest Rust toolchain and the repository’s native build prerequisites. Set `MODEL` to the local [original BF16 checkpoint](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507/tree/cdbee75f17c01a7cc42f958dc650907174af0554). The tile reads the tensor index and the shard containing the embedding, layer 0 input norm and key weights.

From the ZisK repository root, run

```sh
root=$(pwd)
cargo build --locked --release -p ziskemu
cd examples/qwen3-4b/tile
RUSTFLAGS="--cfg zisk_guest -C target-feature=+unaligned-scalar-mem -C link-arg=-T$root/ziskbuild/zisk_linker_script.ld -L native=$HOME/.zisk/bin" \
  cargo +zisk build --locked --release --target riscv64ima-zisk-zkvm-elf
cargo run --locked --release --manifest-path "$root/examples/qwen3-4b/runner/Cargo.toml" -- tile "$MODEL" target/comparison --emulator "$root/target/release/ziskemu" --elf target/riscv64ima-zisk-zkvm-elf/release/qwen-key-tile
```

Choose a fresh output directory for each run. The `tile` command uses the bundled `saved_request.json` and the pinned RMSNorm reduction to reconstruct the 39 activation rows. It checks each framed input against its archived SHA-256, runs `ziskemu -X`, and compares each public commitment with the frozen digest of all 2,496 reference values (`f7c1ceb9b4258f70eee60ef741e0a18e56aabe8ce331e25054aeb8ffd7d7cbf3`). The output directory retains inputs, public outputs, cost CSVs and emulator logs with the command and checked results.

| Build (emulator cost version 2) | Mode 5 steps | Mode 5 cost | Mode 6 steps | Mode 6 cost |
|---|---:|---:|---:|---:|
| Archived measurement | 333,622,869 | 29,950,370,697 | 1,288,035 | 695,973,694 |
| This tile guest, `rustc +zisk 1.94.0-dev` | 332,944,994 | 29,897,502,468 | 1,272,689 | 693,148,896 |

Both builds reproduced the same reference digest. The rebuilt ELF used 43.13× less emulator cost in mode 6. Guest code and toolchain changes can affect the counters.
