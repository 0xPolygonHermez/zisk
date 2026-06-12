# Fibonacci Example

A guest program computes the n-th Fibonacci number as a 256-bit integer and commits the result. The host wraps the proof in a compact `VadcopFinalMinimal` format and verifies the committed value.

## Overview

```text
input: u8 (index n)  →  fibonacci(n)  →  U256  (committed public output)
```

The guest reads an index `n`, computes the Fibonacci number using `U256` arithmetic to avoid overflow at large indices, and commits the result. The host checks the committed value against a locally computed reference.

## Structure

```text
fibonacci/
├── common/       # fibonacci(n: u8) -> U256 and U256 re-export
├── guest/        # Reads u8 index, commits U256 result
└── host/         # Writes index, proves with VadcopFinalMinimal, verifies output
```

## Host

Run from the `host/` directory. Default input is `n = 10`.

```bash
# Default input, software emulation
cargo run --release

# Custom index
cargo run --release -- 20

# ASM backend
cargo run --release -- 20 --asm

# ASM + GPU
cargo run --release -- 20 --asm --gpu
```

## Guest

Run from the `guest/` directory.

```bash
# Build
cargo-zisk build --release

# Emulate
cargo-zisk run --release -i samples/example-input.bin

# Execute (ASM backend)
cargo-zisk execute --release -i samples/example-input.bin --asm

# Prove
cargo-zisk prove --release -i samples/example-input.bin --verify-proof

# Prove (ASM + GPU)
cargo-zisk prove --release -i samples/example-input.bin --asm --gpu --verify-proof
```

## Key concepts

- **`U256` arithmetic** — uses the `ruint` crate for big-integer values that exceed the 64-bit range.
- **`ProofKind::VadcopFinalMinimal`** — compact proof wrapper that reduces proof size at the cost of slightly more prover work.
- **`ProverClient::embedded()`** — runs the prover in-process without a separate server.