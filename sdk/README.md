# ZisK SDK

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

Type-safe Rust SDK for building and proving zero-knowledge programs with ZisK zkVM.

## Quick Start

```toml
[dependencies]
zisk-sdk = { path = "../sdk" }
```

```rust
use zisk_sdk::*;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let prover = ProverClient::builder()
        .emu()                           // or .asm()
        .prove()                         // or .verify_constraints()
        .elf_path(PathBuf::from("target/riscv64ima-zisk-zkvm-elf/release/app"))
        .output_dir(PathBuf::from("proofs"))
        .verify_proofs(true)
        .build()?;

    let stdin = ZiskStdin::from_file("input.bin")?;
    let (result, elapsed, _stats, proof) = prover.prove(stdin)?;

    println!("Proof: {:?} ({}) steps in {:?}", proof.id, result.executed_steps, elapsed);
    Ok(())
}
```

## Backends

**EMU** - Software backend for development (macOS/Linux):
```rust
ProverClient::builder().emu().prove()...
```

**ASM** - Hardware-accelerated for production (Linux, supports MPI):
```rust
ProverClient::builder().asm().prove()...
```

## Operations

**Prove** - Generate proof:
```rust
let (result, elapsed, stats, proof) = prover.prove(stdin)?;
```

**Verify Constraints** - Faster, no proof:
```rust
let (result, elapsed, stats) = prover.verify_constraints(stdin)?;
```

**Witness** - Generate witness:
```rust
prover.execute(stdin, PathBuf::from("output.json"))?;
```

## Input Sources

```rust
// File
let stdin = ZiskStdin::from_file("input.bin")?;

// Memory
let stdin = ZiskStdin::from_vec(vec![1, 2, 3]);

// Null
let stdin = ZiskStdin::null();
```

## Builder Methods

### Common
```rust
.elf_path(PathBuf)                    // Required
.verbose(u8)
.shared_tables(bool)
.print_command_info()
.witness_lib_path(PathBuf)
.proving_key_path(PathBuf)
```

### ASM-Specific
```rust
.asm_path(PathBuf)
.base_port(u16)                       // For MPI
.unlock_mapped_memory(bool)
```

### Prove-Specific
```rust
.output_dir(PathBuf)
.save_proofs(bool)
.verify_proofs(bool)
.minimal_memory(bool)
.gpu(ParamsGPU)
```

## Distributed Proving

Run with MPI:
```bash
mpirun -n 4 your_program
```

Access ranks:
```rust
let world_rank = prover.world_rank();
let local_rank = prover.local_rank();
```

## Examples

### Basic Proof
```rust
let prover = ProverClient::builder()
    .emu()
    .prove()
    .elf_path(PathBuf::from("target/.../app"))
    .output_dir(PathBuf::from("proofs"))
    .build()?;

let stdin = ZiskStdin::from_file("input.bin")?;
let (_result, _elapsed, _stats, proof) = prover.prove(stdin)?;
```

### Constraint Verification
```rust
let prover = ProverClient::builder()
    .emu()
    .verify_constraints()
    .elf_path(PathBuf::from("target/.../app"))
    .build()?;

let stdin = ZiskStdin::from_file("input.bin")?;
let (_result, elapsed, _stats) = prover.verify_constraints(stdin)?;
```

### ASM with Custom Config
```rust
let prover = ProverClient::builder()
    .asm()
    .prove()
    .elf_path(PathBuf::from("target/.../app"))
    .asm_path(PathBuf::from("custom.bin"))
    .base_port(8080)
    .output_dir(PathBuf::from("proofs"))
    .build()?;
```

## Error Handling

```rust
match prover.prove(stdin) {
    Ok((result, elapsed, stats, proof)) => println!("Success"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Types

- `ProverClient` - Entry point (`builder()`)
- `ProverClientBuilder<Backend, Operation>` - Typestate builder
- `ZiskProver<Backend>` - Configured prover
- `ZiskStdin` - Input source (file/memory/null)
- `Proof` - Generated proof (`id`, `proof`)
- `ZiskExecutionResult` - Execution metadata (`executed_steps`)

## License

Licensed under the same license as the ZisK project.

---

For more details, see [ZisK Main Documentation](../../book/).
