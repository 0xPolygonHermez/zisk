# Writing Programs

This page explains how to write a guest program for ZisK. The guest defines the computation to be proven; the host orchestrates execution and proof generation. A guest program runs inside the zkVM — it reads input, does computation, and commits public output.

## Setup

Writing a ZisK guest program is standard Rust with two additions:

1. Add `#![no_main]` and the `entrypoint!` macro to `main.rs`:

    ```rust
    #![no_main]
    ziskos::entrypoint!(main);

    fn main() {
        // your program here
    }
    ```

    `#![no_main]` tells the compiler not to emit the standard `main` entry point. The `entrypoint!` macro generates a `main()` that calls your function — the ZisK runtime invokes it after setting up the stack and heap.

2. Add `ziskos` as a dependency in `Cargo.toml`:

    ```toml
    [dependencies]
    ziskos = { git = "https://github.com/0xPolygonHermez/zisk.git" }
    ```

That's it. Everything else is normal Rust — you can use `std`, `println!`, third-party crates, custom types, etc.

## Inputs and Outputs

### Reading Input

The host writes data with `ZiskStdin::write(&value)` (see [Proving Workflow](./proving_workflow.md)). The guest reads it with `ziskos::io::read()`. Both sides use **bincode** serialization, so the types must match.

```rust
// Read a single value
let n: u32 = ziskos::io::read();

// Read a struct (must implement serde::Deserialize)
let config: MyConfig = ziskos::io::read();
```

Read calls are sequential — the first `read()` in the guest gets the first `write()` from the host, the second gets the second, and so on.

For raw bytes without serde overhead, use `read_vec()`:

```rust
// Read raw bytes (faster, no deserialization)
let bytes: Vec<u8> = ziskos::io::read_vec();
```

Input data is **private** — it is not visible to the verifier. Only committed output is public.

### Committing Public Output

Public output is what the verifier sees. The guest writes it with `ziskos::io::commit()`:

```rust
// Commit a single value
ziskos::io::commit(&result);

// Commit a struct (must implement serde::Serialize)
ziskos::io::commit(&output);
```

The host reads committed output with `result.get_public_values::<T>()` using the same type.

ZisK supports up to **64 public output slots** (each 32-bit). The committed data is bincode-serialized and written to these slots. If your output exceeds 256 bytes (64 x 4 bytes), the commit will panic.

### Custom Types

Any type that implements `Serialize` and `Deserialize` works with `read()` and `commit()`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Output {
    hash: [u8; 32],
    iterations: u32,
}
```

### Example

Here is the [sha-hasher example](https://github.com/0xPolygonHermez/zisk/tree/main/examples/sha-hasher) — the guest reads `n`, computes SHA-256 `n` times, and commits the result:

**`guest/src/main.rs`:**
```rust
#![no_main]
ziskos::entrypoint!(main);

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Debug)]
struct Output {
    hash: [u8; 32],
    iterations: u32,
    magic_number: u32,
}

fn main() {
    let n: u32 = ziskos::io::read();

    let mut hash = [0u8; 32];

    for _ in 0..n {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let digest = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*digest);
    }

    let output = Output { hash, iterations: n, magic_number: 0xDEADBEEF };

    println!("Computed hash: {:02x?}", output.hash);
    println!("Iterations: {}", output.iterations);

    ziskos::io::commit(&output);
}
```

**`guest/Cargo.toml`:**
```toml
[package]
name = "sha-hasher-guest"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", default-features = false, features = ["derive"] }
sha2 = "0.10.8"
ziskos = { git = "https://github.com/0xPolygonHermez/zisk.git" }
```

Note: `println!` works in guest programs — output goes to UART during emulation, which is useful for debugging. It has no effect on the proof.

## Build

Before compiling for ZisK, you can test on your native architecture like any Rust program using `cargo`.

Once ready, compile into an ELF file for the ZisK target:

```bash
cargo-zisk build
```

The resulting ELF file is generated in `./target/riscv64ima-zisk-zkvm-elf/debug`.

For deployment:

```bash
cargo-zisk build --release
```

The release ELF is generated in `./target/elf/riscv64ima-zisk-zkvm-elf/release`.

## Execute

Test your program with the ZisK emulator before generating a proof:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i host/tmp/input.bin
```

If execution exceeds the default step limit, you'll see `EmulationNoCompleted`. Increase it with `-n`:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i host/tmp/input.bin -n 10000000000
```

## Metrics and Statistics

### Performance Metrics

Get execution metrics with the `-m` flag:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i host/tmp/input.bin -m
```

Output includes execution time, throughput (Msteps/s), and cycles per step.

### Execution Statistics

Get cost breakdown with the `-X` flag:

```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i host/tmp/input.bin -X
```

Output includes total cost, cost per type (main, memory, opcodes), register access patterns, and per-opcode statistics.

## Prove

### Program Setup

Generate program setup files (required once after building, or when the ELF changes):

```bash
cargo-zisk rom-setup -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest
```

The `-k` flag sets a custom proving key path (default: `$HOME/.zisk/provingKey`). Setup files go to `$HOME/.zisk/cache`. Clean them with `cargo-zisk clean`.

### Verify Constraints

Check all constraints are satisfied without generating a proof:

```bash
cargo-zisk verify-constraints -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i host/tmp/input.bin
```

### Generate Proof

```bash
cargo-zisk prove -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i host/tmp/input.bin -o proof -a -y
```

Flags: `-o` output directory, `-a` enable aggregation, `-y` verify after generation.

### Concurrent Proof Generation

Use MPI for multi-process proving:

```bash
mpirun --bind-to none -np <num_processes> \
  -x OMP_NUM_THREADS=<threads_per_process> \
  -x RAYON_NUM_THREADS=<threads_per_process> \
  target/release/cargo-zisk prove <args>
```

Rule of thumb: `num_processes * threads_per_process` should match available CPU cores. Each process needs ~25GB of RAM.

### GPU Proof Generation

1. Requires NVIDIA GPU with [CUDA Toolkit](https://developer.nvidia.com/cuda-downloads)
2. Build with GPU support: `cargo build --release --features gpu`
3. Regenerate constant trees: `cargo-zisk check-setup -a`

Can be combined with MPI. Ensure sufficient GPU memory per process.

### Verify Proof

```bash
cargo-zisk verify -p ./proof/vadcop_final_proof.bin
```

The `-k` flag sets a custom proving key path (default: `$HOME/.zisk/provingKey`).
