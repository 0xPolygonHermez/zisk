# GCD Example

A guest program computes the greatest common divisor of two 64-bit integers using the Euclidean algorithm. The host side covers multiple prover-client configurations (embedded, remote) and proof formats (STARK, minimal, PLONK).

## Overview

```text
input: (u64, u64)  →  Euclidean GCD  →  u64  (committed public output)
```

The guest reads two `u64` values, computes their GCD, and commits the result. The host variants below show different ways to produce and format that proof.

## Structure

```text
gcd/
├── common/                    # gcd(a, b) algorithm
├── guest/                     # Reads two u64, commits GCD result
└── host/
    ├── prover-clients/
    │   ├── embedded.rs        # In-process prover
    │   └── remote.rs          # Remote prover via HTTP
    └── proof-formats/
        ├── stark.rs           # Plain STARK proof
        ├── minimal.rs         # Compact VadcopFinalMinimal proof
        └── plonk.rs           # PLONK proof for on-chain verification
```

## Host

Run from the `host/` directory. Default inputs are `a = 5`, `b = 10`.

### Prover-client variants

```bash
# Embedded — default input, software emulation
cargo run --release --bin embedded-host

# Embedded — custom inputs
cargo run --release --bin embedded-host -- 48 36

# Embedded — ASM backend
cargo run --release --bin embedded-host -- 48 36 --asm

# Embedded — ASM + GPU
cargo run --release --bin embedded-host -- 48 36 --asm --gpu

# Remote — requires a running prover server
cargo run --release --bin remote-host -- 48 36
```

### Proof-format variants

```bash
# Plain STARK
cargo run --release --bin stark-host -- 48 36 --asm

# Compact (VadcopFinalMinimal)
cargo run --release --bin minimal-host -- 48 36 --asm

# PLONK (on-chain verification)
cargo run --release --bin plonk-host -- 48 36 --asm
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

- **`ProverClient::embedded()`** — runs the prover in-process; `EmbeddedOpts::minimal_memory()` reduces RAM usage.
- **`ProverClient::remote()`** — connects to a prover server over HTTP with configurable connect and request timeouts.
- **`ProofKind::Stark`** — standard STARK proof with no additional wrapping.
- **`ProofKind::VadcopFinalMinimal`** — compact proof for efficient off-chain or on-chain verification.
- **`ProofKind::Plonk`** — PLONK proof for EVM-compatible on-chain verification.