# Primes Example

A guest program filters a list of integers, sums the prime values, and commits the result. Four host variants cover different input serialization strategies, including an async Unix-socket streaming interface.

## Overview

```text
input: Vec<u64>  →  sum of primes  →  u64  (committed public output)
```

Each guest variant reads the same list but deserializes it differently. The host writes the matching serialization and verifies the committed sum.

| Variant | Serialization strategy |
| --- | --- |
| `struct` | `serde` — `InputDTO` struct read in one call |
| `multiple` | Length prefix followed by individual `u64` reads |
| `slice` | `rkyv` zero-copy — raw byte slice deserialized in-place |
| `streaming` (host only) | Async Unix-socket stream; values written one by one |

## Structure

```text
primes/
├── common/             # is_prime(), InputDTO (serde), InputZeroCopyDTO (rkyv)
├── guest/
│   ├── struct.rs       # read::<InputDTO>()
│   ├── multiple.rs     # reads length, then each u64
│   └── slice.rs        # read_slice() + rkyv zero-copy
└── host/
    ├── struct.rs
    ├── multiple.rs
    ├── slice.rs
    └── streaming.rs    # async ZiskStream over Unix socket (Linux + ASM only)
```

## Host

Run from the `host/` directory. Default input is `5 11 18 23 45`.

```bash
# Default input, software emulation
cargo run --release --bin struct-host

# Custom input
cargo run --release --bin struct-host -- 5 11 18 23 45

# ASM backend
cargo run --release --bin struct-host -- 5 11 18 23 45 --asm

# ASM + GPU
cargo run --release --bin struct-host -- 5 11 18 23 45 --asm --gpu
```

Replace `struct-host` with `multiple-host` or `slice-host` for the other variants. All three accept the same flags.

The `streaming-host` binary always uses the ASM backend (Linux only) and accepts `--gpu` as an additional flag:

```bash
# Streaming — default input
cargo run --release --bin streaming-host

# Streaming — custom input, GPU
cargo run --release --bin streaming-host -- 5 11 18 23 45 --gpu
```

## Guest

Run from the `guest/` directory. Each variant has its own sample input file.

```bash
# Build all variants
cargo-zisk build --release

# struct-guest
cargo-zisk run     --release --bin struct-guest   -i samples/example-input-struct.bin
cargo-zisk execute --release --bin struct-guest   -i samples/example-input-struct.bin --asm
cargo-zisk prove   --release --bin struct-guest   -i samples/example-input-struct.bin --verify-proof
cargo-zisk prove   --release --bin struct-guest   -i samples/example-input-struct.bin --asm --gpu --verify-proof

# multiple-guest
cargo-zisk run     --release --bin multiple-guest -i samples/example-input-multiple.bin
cargo-zisk execute --release --bin multiple-guest -i samples/example-input-multiple.bin --asm
cargo-zisk prove   --release --bin multiple-guest -i samples/example-input-multiple.bin --verify-proof

# slice-guest
cargo-zisk run     --release --bin slice-guest    -i samples/example-input-slice.bin
cargo-zisk execute --release --bin slice-guest    -i samples/example-input-slice.bin --asm
cargo-zisk prove   --release --bin slice-guest    -i samples/example-input-slice.bin --verify-proof
```

## Key concepts

- **`struct` variant** — `serde` serialization; the easiest starting point for structured input.
- **`multiple` variant** — reads values incrementally; input size does not need to be known at compile time.
- **`slice` / rkyv variant** — zero-copy deserialization; the byte buffer is reinterpreted in-place with no allocation in the guest.
- **`streaming` variant** — uses `ZiskStream` over a Unix socket; the host flushes values asynchronously while the prover runs, enabling pipelined input for long sequences.