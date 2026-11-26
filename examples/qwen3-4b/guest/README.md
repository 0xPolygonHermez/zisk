# Qwen3-4B BF16 guest

One ZisK executable accepts the measured Qproof embedding, decoder-layer and
vocabulary-shard records. Supply the original little-endian, length-prefixed
stdin written by `prepare`. The eight-byte magic inside the payload
selects the stage.

| Magic | Model entry point | Captured output |
| --- | --- | --- |
| `PCIEMBD1` | `stages::embed::execute` | T × 2560 embedding values |
| `PCILAYR1` | `layer::execute_batched` | T × 2560 decoder activations |
| `PCIHEAD1` | `stages::head::execute_with` | R × shard rows of logits |

Every capture contains row-major little-endian FP32 values. The guest commits
the 32-byte stage receipt digest, the little-endian u64 guest address of the
capture, and its little-endian u64 byte length. The buffer stays allocated
through halt. Receipts can be compared across builds even when the
allocator places the buffer at a different address.

## Build and run

From the ZisK repository root with the ZisK Rust toolchain and guest runtime
archive installed:

```sh
root=$PWD
cd examples/qwen3-4b/guest
RUSTFLAGS="--cfg zisk_guest -C target-feature=+unaligned-scalar-mem -C link-arg=-T$root/ziskbuild/zisk_linker_script.ld -L native=$HOME/.zisk/bin" \
  cargo +zisk build --release --locked --target riscv64ima-zisk-zkvm-elf
cargo +stable test --locked -p qwen-model

elf=target/riscv64ima-zisk-zkvm-elf/release/qwen3-4b-guest
ziskemu -e "$elf" -i /path/to/embed.stdin -o embed.public \
  --max-input-mem 268435456 -n 1000000000000
```

Use an emulator built with the BF16 opcode. The same command accepts a framed
layer or head record with its input activations populated.

## Arithmetic and framing

The model crate contains the BF16 execution and receipt paths used by the
original benchmark guest. It reads raw checkpoint tensors from each stage
record. RMSNorm, RoPE, causal attention (including the prefill/decode
boundary), SiLU and residual operations retain the frozen rounding rules.
The runner reads `rope_theta` and `head_dim` from the checkpoint configuration. The decoder checks the layer profile and uses these fields to compute the rotary frequencies and the BF16 cosine and sine at each position. The rotary regression test covers all 40,960 positions supported by the runner. Its reference is the checkpoint’s `Qwen3RotaryEmbedding` under Transformers 5.15.1 and PyTorch CPU arm64 build `cf30153c`, with cosine and sine serialized as interleaved little-endian BF16 pairs.

The build script computes a weight-independent BF16 SiLU lookup table.

The seven decoder projections and the head call `bf16_projection::project`.
Each scalar `syscall_dot_bf16` consumes a BF16 activation row and a BF16
checkpoint row. The syscall rounds the exact dot product to FP32, and the
guest rounds the result to BF16 before widening it for the next operation.
This reduction can differ from ordered PyTorch accumulation. The parity
reference for this guest is the measured BF16-syscall replay.

## Verified sample

For the saved 39-position request, the public guest reproduced the original
benchmark guest's 32-byte receipt and every captured byte for these stages.
The head input uses the measured layer-35 output in its activation field.

| Stage | Capture bytes | Receipt SHA-256 digest (hex) |
| --- | ---: | --- |
| Embedding | 399,360 | `b6e499a5694db94dc64ec9ad855f156f9599e673dc7250ea1f03fe75f71d059f` |
| Layer 0 | 399,360 | `e7635583d04788633aa93aaeb4bb976646d18353ac6163bf072e76bc9c4fc01f` |
| Head shard 74 | 18,432 | `ddef1f274eef43ef1b64cf45e20c9fe1b6846e183ba071c96ea1ceff876377cb` |
