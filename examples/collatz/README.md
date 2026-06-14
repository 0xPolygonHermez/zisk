# Collatz Example

A guest program generates the Collatz sequence for a given starting number. Three guest/host pairs cover different strategies for committing the public output: a serialized struct, individual typed values, and a SHA-256 digest.

## Overview

```text
input: u64 (n)  →  Collatz sequence  →  output (varies by variant)
```

The Collatz sequence starts at `n` and applies `n / 2` when `n` is even, `3n + 1` when odd, until reaching 1.

| Variant | Committed output |
| --- | --- |
| `single` | `OutputDTO { n, sequence }` — serialized as one value |
| `sequential` | `n` and `sequence` committed separately |
| `compressed` | SHA-256 digest of `n` concatenated with the sequence |

## Structure

```text
collatz/
├── common/             # collatz(n) algorithm, OutputDTO type
├── guest/
│   ├── single.rs       # Commits OutputDTO struct
│   ├── sequential.rs   # Commits n and sequence separately
│   └── compressed.rs   # Commits SHA-256 digest
└── host/
    ├── single.rs
    ├── sequential.rs
    └── compressed.rs
```

## Host

Run from the `host/` directory. Default input is `n = 55`.

```bash
# Default input, software emulation
cargo run --release --bin single-host

# Custom input
cargo run --release --bin single-host -- 27

# ASM backend
cargo run --release --bin single-host -- 27 --asm

# ASM + GPU
cargo run --release --bin single-host -- 27 --asm --gpu
```

Replace `single-host` with `sequential-host` or `compressed-host` for the other variants. All three accept the same flags.

## Guest

Run from the `guest/` directory. All variants share the same sample input file.

```bash
# Build all variants
cargo-zisk build --release

# Emulate
cargo-zisk run --release --bin single-guest -i samples/example-input.bin

# Execute (ASM backend)
cargo-zisk execute --release --bin single-guest -i samples/example-input.bin --asm

# Prove
cargo-zisk prove --release --bin single-guest -i samples/example-input.bin --verify-proof

# Prove (ASM + GPU)
cargo-zisk prove --release --bin single-guest -i samples/example-input.bin --asm --gpu --verify-proof
```

Replace `single-guest` with `sequential-guest` or `compressed-guest` for the other variants.

## Key concepts

- **Struct output (`single`)** — the entire result is serialized into one committed value; the host deserializes it back into `OutputDTO`.
- **Sequential output (`sequential`)** — values are committed one at a time; the host reads each field independently.
- **Compressed output (`compressed`)** — only a hash of the output is committed; the host re-computes the sequence locally and compares digests, keeping verification cost constant regardless of sequence length.