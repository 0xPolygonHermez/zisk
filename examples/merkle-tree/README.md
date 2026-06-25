# Merkle Tree Example

A guest program builds a binary SHA-256 Merkle tree and commits the root. Two variants compare hash implementations — pure-Rust `sha2` versus ZisK's built-in `zisklib` accelerated hash. The host runs both in profiling mode to measure the cycle difference; no proof is generated.

## Overview

```text
input: u32 (leaf count)  →  SHA-256 Merkle tree  →  [u8; 32]  (committed root)
```

Each guest generates `n` leaves by hashing sequential indices, then hashes pairs of nodes up the tree. Odd-length levels duplicate the last node, matching standard Merkle tree construction.

| Variant | SHA-256 source |
| --- | --- |
| `inline-guest` | `sha2` crate (pure Rust); profiling markers around leaf prep and root computation |
| `zisklib-guest` | `ziskos::zisklib::sha256` (ZisK built-in accelerated hash) |

## Structure

```text
merkle-tree/
├── common/           # merkle_root(), merkle_root_zisklib(), Hash type
├── guest/
│   ├── inline.rs     # Pure-Rust SHA-256 with profiling markers
│   └── zisklib.rs    # ZisK built-in SHA-256
└── host/             # Profiling runner for both variants
```

## Host

The host prints cycle and instruction reports for both variants. It does not generate proofs, so `--asm` and `--gpu` are not applicable. Run from the `host/` directory.

```bash
# Default leaf count (1000)
cargo run --release

# Custom leaf count
cargo run --release -- 500
```

## Guest

Run from the `guest/` directory. `inline-guest` is for profiling only; `zisklib-guest` supports the full prove pipeline.

```bash
# Build all variants
cargo-zisk build --release

# inline-guest — emulate only (profiling, not for proving)
cargo-zisk run --release --bin inline-guest -i samples/example-input.bin

# zisklib-guest — emulate
cargo-zisk run     --release --bin zisklib-guest -i samples/example-input.bin

# zisklib-guest — execute (ASM backend)
cargo-zisk execute --release --bin zisklib-guest -i samples/example-input.bin --asm

# zisklib-guest — prove
cargo-zisk prove   --release --bin zisklib-guest -i samples/example-input.bin --verify-proof

# zisklib-guest — prove (ASM + GPU)
cargo-zisk prove   --release --bin zisklib-guest -i samples/example-input.bin --asm --gpu --verify-proof
```

## Key concepts

- **`ziskos::zisklib::sha256`** — ZisK's accelerated SHA-256 built-in; substantially reduces cycle count compared to the `sha2` crate running inside the ZisK VM.
- **`profile_report_start!/end!`** — macros that bracket named regions in `inline-guest`, reporting leaf preparation and root computation separately.
- **`ProfilingMode::Inline` / `ProfilingMode::Summary`** — the host uses both modes to emit per-region and aggregate instruction counts without running the full prover.