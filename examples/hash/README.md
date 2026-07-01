# Hash Example

A guest program reads a string, computes its SHA-256 digest, and commits the 32-byte result as a public output. The host proves the computation and verifies the committed hash.

## Overview

```text
input: String  →  SHA-256  →  [u8; 32]  (committed public output)
```

The guest reads a `String` from the input stream, hashes it with SHA-256, and commits the digest. The host writes the same string, runs the prover, and checks the committed hash against a locally computed reference.

## Structure

```text
hash/
├── common/       # Hash type alias and re-exports (sha2, hex)
├── guest/        # Reads String, commits SHA-256 digest
└── host/         # Writes input, proves, verifies output
```

## Host

Run from the `host/` directory. Default input is `"Hello Zisk!"`.

```bash
# Default input, software emulation
cargo run --release

# Custom input
cargo run --release -- "my message"

# ASM backend
cargo run --release -- "my message" --asm

# ASM + GPU
cargo run --release -- "my message" --asm --gpu
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

- **`ziskos::io::read::<T>()`** — deserializes typed input from the guest's input stream.
- **`ziskos::io::commit_slice(&bytes)`** — exposes a byte slice as a public output the host can read and verify.
- **`ProverClient::embedded()`** — runs the prover in-process without a separate server.