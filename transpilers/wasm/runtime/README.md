The Nitro helper is compiled C code executed inside the ZisK ROM. It uses the
same Brotli source commit as the Nitro fixture checkout and the existing
`zkvm_calls.s` crypto entry points. No host codec callback is involved.

From the repository root:

```sh
python3 transpilers/wasm/runtime/build.py --test
```

This downloads SHA-256 checked source archives into `target/nitro-runtime`, runs
native Brotli controls, and emits `nitro-runtime.elf`, its linker map, and
`build.json` with source identities, tool versions and ELF digest. Python 3.12+
and LLVM clang/ld.lld with RISC-V support are prerequisites. Set `NITRO_CC`,
`NITRO_LD`, or `HOST_CC` if the tools are not on PATH. On macOS the builder also
checks the usual Homebrew LLVM location; Apple's clang lacks a RISC-V backend.
A clean local source checkout can avoid the Brotli download:

```sh
python3 transpilers/wasm/runtime/build.py --test --brotli-source /path/to/brotli
```

The checkout must be exactly `f4153a09f87cbb9c826d8fc12c74642bb2d879ea` with no
modified or untracked files. The builder extracts a Git archive, rather than
copying working files. The default Brotli archive is pinned by revision and
SHA-256. `log2` comes from musl v1.1.19, with its original Sun license intact and
its bytes pinned by SHA-256. Both source archives stay outside version control.

The ELF uses RV64IMFD, the integer-register `lp64` calling convention, no
compressed instructions, no relaxation, and no fused floating point operations.
The Nitro compiler must enable the existing ZisK float instruction lowering.
The ELF has separate execute-only/read-only/writable segments. The linker rejects
ROM beyond `[0x97000000,0x97e00000)` or RAM beyond
`[0xa0600000,0xa1000000)`; the caller supplies the native stack. `nitro_dispatch`
is the ELF entry point. Its arguments are `(operation, args[8], wasm_mem_end)`;
operations 20 and 21 are Brotli decompression and compression.

Brotli inputs and temporary outputs are copied, matching Nitro's overlap and
failure behavior. Output scratch starts at the smaller of caller capacity and
64 KiB, and doubles only when the streaming codec needs more output space.
Caller capacity is a limit, rather than an allocation request. Status is 1 on
complete success and 0 for codec failure. Output bytes and the little-endian
output-length word are published only on success. Input and length pointers are bounds checked before reading;
the output pointer is checked only after successful coding. Quality and window
are passed as Wasm `u32` values to the pinned encoder. Dictionary selection uses
Nitro's `u8` conversion. Only dictionary zero is supported; Stylus and unknown
dictionaries explicitly trap.

The per-call allocator is an 8 MiB arena with free block reuse and coalescing;
state storage uses another approximately 328 KiB. Arena state resets on the
next call. Allocation exhaustion traps without publishing output.
`runtime_arena_highwater()` reports peak live allocation bytes, including block
headers. The native controls measure about 730,400 bytes for the small quality
11 fixture and 5,719,504 bytes for 64 KiB of incompressible input. Larger quality
11 inputs can exhaust the arena and explicitly trap. The recorded transfer below
fits this arena. A tiny valid stream with a `u32::MAX` output
capacity succeeds using bounded scratch; actual codec memory exhaustion traps.

Native controls cover a fixed compressed fixture, round trips, exact and
partial overlaps, invalid/truncated streams, streaming output growth, capacity
errors, output bounds,
allocator reuse and exhaustion. Actual VM controls require the built ELF:

```sh
cargo test -p ziskemu --features float --test wasm_nitro_brotli -- --ignored
```

See `witness-format.md` for the state witness and public output contract. VM
execution alone does not establish Assembly execution or proof verification.

## Execute the recorded transfer

Use the replay Wasm and `TestRecordBlockTransfer/block_inputs_5.json` from the
existing Nitro recording. The fixture encoder supports one complete non-Stylus
batch and keeps the expected end state in a separate file, never in VM input:

```sh
python3 transpilers/wasm/runtime/prepare-witness.py --self-test
python3 transpilers/wasm/runtime/prepare-witness.py \
  --block-file /path/to/block_inputs_5.json \
  --replay-wasm /path/to/transfer-replay.wasm \
  --output target/nitro-runtime/transfer.stdin \
  --expected target/nitro-runtime/transfer.expected
cargo run --locked --release -p ziskemu --features float --example nitro_replay -- \
  /path/to/transfer-replay.wasm target/nitro-runtime/nitro-runtime.elf \
  target/nitro-runtime/transfer.stdin target/nitro-runtime/transfer.expected \
  target/nitro-runtime/transfer.actual
```

The runner requires successful termination and compares all 256 public bytes.
An optional final path exports the exact linked program as a ZisK binary ELF
for subsequent Assembly/prover setup. The default Wasm CLI does not install
this adapter. Use a host with at least 24 GiB available RAM for this large module;
its runner has a two-billion-step execution limit.

Validated artifact SHA-256 values (LLVM 23.1.4):

| Artifact | SHA-256 |
| --- | --- |
| Replay Wasm | `15a1bfa995d941d4ec2edc5d9dc3acbe0bab839499144029058944e1f0833521` |
| Runtime ELF | `7e6568c440d7569af308244fd0ee1b189944e16ba0928688f9a2d7c60e13385b` |
| Framed witness | `eac529c2a6701f0998dcf15b5850e22f67c252837d2d01b18465acc928bf1ea2` |
| Expected and actual 256-byte claim | `438636f44cce44870bf9ba300cec05cf2c9911d3faa4bf722a1ebce39f049754` |

The linked ROM contains 20,806,583 instructions. The transfer executes
894,975,514 steps and computes block hash
`a67bfee73eec54ea031ebe065a14f2a34f7a783a4e9cf57d1060eb730cfb76bc`, zero send
root, batch 5, position 0. The local run took 20.48 seconds including compilation
and cleanup, with 15.15 GiB peak sampled RSS. These are emulator measurements,
not proving costs. Corrupting preimage bytes, removing preimages, and changing
the sequencer count-start anchor each trap before a claim is returned.

Native host controls use sanitizers; the following tests execute the real helper
and production hashes inside the VM (13 tests, approximately 40 seconds):

```sh
cargo test --locked --release -p ziskemu --features float \
  --test wasm_nitro_brotli --test wasm_nitro_state --test wasm_nitro_runtime \
  -- --ignored --test-threads=1
```

## Proof route and remaining scope

A small native-crypto control verifies the linked ELF round-trip and generates
MT, RH, MOH and MOL Assembly sources, with and without hints:

```sh
cargo test --locked --release -p ziskemu --test native_crypto_failure \
  export_native_crypto_proof_control -- --ignored --nocapture
```

Outputs are under `target/nitro-proof-route`. Actual Assembly execution, proof
generation and proof verification remain untested: this host is Darwin ARM64,
its assembler rejects the required GNU options, and Docker is unavailable.
The next step is to build this branch on Linux x86-64, validate the small control,
then generate and verify the recorded transfer proof under the new program key.

Wider Nitro support still requires Stylus and its dictionary, blob/KZG
authentication, recovery IDs 2/3, multiple delayed-count checkpoints and broader
recording coverage. Current unsupported operations trap. Witness limits are
8,192 preimages, 1,024 messages per inbox and 64 MiB payload; codec memory is
bounded as described above. Existing L1/binder logic has not been connected to
this new program identity. The witness's before-accumulator anchors are private
openings committed by the claim and require that external authentication.
