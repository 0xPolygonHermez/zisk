# Builder and Types

Reference for the prover builder, input and result types, proof modes, and verification. Step-by-step flow: [Proving Workflow](./proving_workflow.md). Proof types: [Proof Types](./proof_types.md). Advanced topics: [Proving Advanced](./proving_advanced.md).

---

## Table of Contents

1. [ProverClient and the Builder](#1-proverclient-and-the-builder)
2. [ZiskStdin](#2-ziskstdin)
3. [ProofOpts](#3-proofopts)
4. [ProveBuilder](#4-provebuilder)
5. [Result Types](#5-result-types)
6. [ZiskPublics](#6-ziskpublics)
7. [Standalone Verification](#7-standalone-verification)
8. [ZiskProof and Serialization](#8-ziskproof-and-serialization)
9. [Multiple Programs](#9-multiple-programs)
10. [Feature Flags](#10-feature-flags)

---

## 1. ProverClient and the Builder

`ProverClient::builder()` returns a `ProverClientBuilder`. The builder uses a typestate pattern: select a backend, then an operation, then call `.build()` to obtain a `ZiskProver`. Before `.build()`, chain configuration methods (aggregation, SNARK mode, proving key paths, verbosity). One builder per process; reuse the built prover for setup, execution, and proof generation.

```rust
use zisk_sdk::ProverClient;

let prover = ProverClient::builder()
    .emu()
    .prove()
    .build()?;

let (pk, vk) = prover.setup(&ELF)?;
let result = prover.execute(&pk, stdin.clone())?;
let result = prover.prove(&pk, stdin).run()?;
prover.verify(result.get_proof(), result.get_publics(), &vk)?;
```

### 1.1. Backend

Backend controls execution and proof generation. Default if unspecified: emulator.

| Method | Description |
| --- | --- |
| `.emu()` | Emulator. macOS and Linux. |
| `.asm()` | Assembly backend. Linux. Required for MPI distributed proving. |

### 1.2. Operation

| Method | Description |
| --- | --- |
| `.prove()` | Full proof generation. Aggregation enabled by default. |
| `.verify_constraints()` | Runs the witness and checks constraints, no proof output. Disables aggregation. |
| `.witness()` | Witness only. Disables constraint verification and aggregation. |

### 1.3. Configuration

Chain before `.build()`. ASM-only options, GPU, shared tables, and logging: [Proving Advanced](./proving_advanced.md).

| Method | Description |
| --- | --- |
| `.aggregation(bool)` | Enable or disable proof aggregation. Default is `true` for `.prove()`, `false` for `.verify_constraints()` and `.witness()`. |
| `.snark()` | Enable the SNARK wrapper. Required if you call `.plonk()` on the `ProveBuilder`. |
| `.with_snark(bool)` | Same as `.snark()` but takes a boolean (e.g. from a CLI flag). |
| `.proving_key_path(PathBuf)` | Custom proving key directory. Default is `~/.zisk/provingKey`. |
| `.proving_key_snark_path(PathBuf)` | Custom SNARK proving key directory. Default is `~/.zisk/provingKeySnark`. |
| `.verbose(u8)` | Log verbosity; higher values produce more output. |

### 1.4. Examples

```rust
use zisk_sdk::ProverClient;

// Emulator with constraint verification only (no proof)
let prover = ProverClient::builder()
    .emu()
    .verify_constraints()
    .build()?;

// Emulator with proof generation
let prover = ProverClient::builder()
    .emu()
    .prove()
    .build()?;

// Emulator with SNARK (call .plonk().run() when proving)
let prover = ProverClient::builder()
    .emu()
    .prove()
    .snark()
    .build()?;
```

---

## 2. ZiskStdin

Input to the guest program. Data is held behind an `Arc` (cloning is cheap). Host writes via `ZiskIO`; guest reads via `ziskos::io::read` or `ziskos::read_input_slice`. Order and types must match. Pass the same `ZiskStdin` to `execute` or `prove`.

### 2.1. Creating Stdin

| Method | Description |
| --- | --- |
| `ZiskStdin::new()` | Empty memory-based stdin. Add data with `.write()` or `.write_slice()`. |
| `ZiskStdin::null()` | No input. When the guest does not read any input. |
| `ZiskStdin::from_file(path)` | Load from a binary file (for example one previously saved with `.save()`). |
| `ZiskStdin::from_vec(data)` | Create from a `Vec<u8>` of raw bytes. |
| `ZiskStdin::from_uri(Option<S>)` | Parse a URI: `file://path` or a plain file path. `None` yields null stdin. |

### 2.2. Writing Data (Host)

| Method | Description |
| --- | --- |
| `.write(&value)` | Serialize the value with bincode and append. The guest reads with `ziskos::io::read::<T>()` using the same type. |
| `.write_slice(&[u8])` | Append raw bytes. The guest reads with `ziskos::read_input_slice()`. |

### 2.3. Reading and Persistence

`.read::<T>()` deserializes and consumes the next value (advances internal read pointer). `.read_slice()`, `.read_into()`, `.read_bytes()` for raw bytes. `.save(path)` writes stdin to a file (e.g. for `cargo-zisk prove -i input.bin`).

---

## 3. ProofOpts

Configures proof-generation behavior. All methods return `Self` (chainable). Pass to `ProveBuilder` via `.with_proof_options(opts)`.

```rust
use zisk_sdk::ProofOpts;

let opts = ProofOpts::default()
    .minimal_memory()
    .save_proofs()
    .output_dir("./proofs".into());
```

| Method | Default | Description |
| --- | --- | --- |
| `.minimal_memory()` | `false` | Reduces memory use during proving at the cost of speed. |
| `.no_aggregation()` | aggregation on | Individual sub-proofs without final aggregation. |
| `.save_proofs()` | `false` | Write intermediate and final proofs to disk (under `.output_dir()` if set). |
| `.verify_proofs()` | `false` | Verify each proof immediately after generation. |
| `.output_dir(PathBuf)` | none | Directory for saved proofs when `.save_proofs()` is enabled. |

Details: [Proving Advanced](./proving_advanced.md#proofopts-in-depth).

---

## 4. ProveBuilder

`prover.prove(&pk, stdin)` returns a `ProveBuilder`; it does not run until `.run()` is called. Chain methods to choose proof type (and optionally proof options), then call `.run()`. Default: Vadcop STARK. Alternatives: `.compressed().run()` (compressed STARK), `.plonk().run()` (Plonk SNARK). If both `.compressed()` and `.plonk()` are chained, the last call wins. SNARK requires the prover to be built with `.snark()`; otherwise `.plonk().run()` fails at runtime.

```rust
// Default STARK proof
let result = prover.prove(&pk, stdin.clone()).run()?;

// Compressed STARK proof (smaller, same security)
let result = prover.prove(&pk, stdin.clone()).compressed().run()?;

// Plonk SNARK proof (requires .snark() on the builder)
let result = prover.prove(&pk, stdin.clone()).plonk().run()?;

// With proof options
let result = prover.prove(&pk, stdin)
    .with_proof_options(ProofOpts::default().minimal_memory())
    .run()?;
```

| Method | Description |
| --- | --- |
| `.run()` | Generate a Vadcop STARK proof (`ZiskProof::VadcopFinal`). |
| `.compressed()` | Use compressed STARK mode. Call `.run()` after. Produces `ZiskProof::VadcopFinalCompressed`. |
| `.plonk()` | Use SNARK mode. Call `.run()` after. Produces a Plonk proof (`ZiskProof::Plonk`). Requires `.snark()` on the prover builder. |
| `.with_proof_options(opts)` | Apply a `ProofOpts` configuration for this run. |

---

## 5. Result Types

Return type depends on the operation: `execute` (execution and publics, no proof), `prove(...).run()` (proof and publics), `verify_constraints` (execution and publics only). Each exposes getters for execution metrics (steps, duration, cost) and public outputs.

### 5.1. ZiskExecuteResult

Returned by `prover.execute(&pk, stdin)`. No proof.

| Method | Description |
| --- | --- |
| `.get_execution_steps()` | Execution cycles (main cost metric). |
| `.get_duration()` | Wall-clock execution time. |
| `.get_publics()` | Raw public outputs. |
| `.get_public_values::<T>()` | Deserialize public outputs; `T` must match what the guest committed. |
| `.get_execution_total_cost()`, `.get_execution_cost_per_type()` | Execution cost breakdown. |

### 5.2. ZiskProveResult

Returned by `prover.prove(&pk, stdin).run()` (and compressed/Plonk variants).

| Method | Description |
| --- | --- |
| `.get_proof()` | The generated proof. |
| `.get_publics()` | Public outputs committed by the guest. |
| `.get_program_vk()` | Program verification key (derived from the ELF). |
| `.get_proof_with_publics()` | Bundle of proof, publics, and program VK. |
| `.get_public_values::<T>()` | Deserialize public outputs. |
| `.save_proof_with_publics(path)` | Save the bundle to a binary file. |
| `.get_execution_steps()`, `.get_duration()` | Execution cycles and wall-clock time. |
| `.get_execution_total_cost()`, `.get_execution_cost_per_type()`, `.get_stats()`, `.get_proof_id()` | Cost breakdown, stats, proof ID. |

### 5.3. ZiskVerifyConstraintsResult

Returned by `prover.verify_constraints(&pk, stdin)`. Execution metrics and publics only; no proof. Holds `ExecutorStatsHandle` for per-AIR statistics.

| Method | Description |
| --- | --- |
| `.get_execution_steps()`, `.get_duration()` | Execution cycles and wall-clock time. |
| `.get_publics()` | Raw public outputs. |
| `.get_public_values::<T>()` | Deserialize public outputs. |

---

## 6. ZiskPublics

64 public output values (32-bit each) committed by the guest with `ziskos::io::commit()`. Internal read pointer; deserialize sequentially with `.read::<T>()`. STARK verification: `.public_bytes()` (little-endian u64). Solidity/on-chain: `.public_bytes_solidity()`, `.bytes_solidity(program_vk, vadcop_verkey)`, `.hash_solidity(...)` (SHA-256 of the same sequence the contract hashes).

| Method | Description |
| --- | --- |
| `.read::<T>()` | Deserialize the next value from the public outputs. Advances the pointer. |
| `.read_slice(&mut [u8])` | Read raw bytes from the current pointer position. |
| `.head()` | Reset the read pointer to the beginning. |
| `.public_bytes()` | All public values as `Vec<u8>` in little-endian u64 format (for STARK verification). |
| `.public_bytes_solidity()` | Public values as `Vec<u8>` in big-endian u32 format (for Solidity contract verification). |
| `.bytes_solidity(program_vk, vadcop_verkey)` | Full byte sequence for on-chain verification: `program_vk` + public values (solidity format) + `vadcop_verkey`. The contract hashes this with SHA-256. |
| `.hash_solidity(program_vk, vadcop_verkey)` | SHA-256 hash of the same byte sequence as `.bytes_solidity(...)`. |
| `ZiskPublics::write(&value)` | Static method: serialize a value into a new `ZiskPublics`. Used internally. |

---

## 7. Standalone Verification

Verify without a prover instance. Default keys: `verify_zisk_proof`, `verify_zisk_snark_proof`. Custom paths: `verify_zisk_proof_with_proving_key`, `verify_zisk_snark_proof_with_proving_key`. Program VK from ELF: `prover.vk(&ELF)?` or `get_program_vk(&elf)?` / `get_program_vk_with_proving_key(&elf, path)`. Full details: [Proving Advanced](./proving_advanced.md#standalone-verification).

---

## 8. ZiskProof and Serialization

`ZiskProof` variants: default STARK (`VadcopFinal`), compressed STARK (`VadcopFinalCompressed`), Plonk (`Plonk`), or `Null` (witness-only, no proof).

```rust
pub enum ZiskProof {
    Null(),                         // No proof (witness-only mode)
    VadcopFinal(Vec<u8>),           // Default STARK proof
    VadcopFinalCompressed(Vec<u8>), // Compressed STARK proof
    Plonk(Vec<u8>),                 // Plonk SNARK proof
}
```

Single proof: `proof.save(path)?`, `ZiskProof::load(path)?`. Bundle: `result.get_proof_with_publics()` then `.save(path)`; load with `ZiskProofWithPublicValues::load(path)?`, then `prover.verify(loaded.get_proof(), loaded.get_publics(), &vk)?`. Full flow: [Proving Advanced](./proving_advanced.md#saving-and-loading-proof-bundles).

```rust
proof.save("my_proof.bin")?;
let loaded = ZiskProof::load("my_proof.bin")?;
```

---

## 9. Multiple Programs

One prover can serve multiple ELFs. Call `setup` once per ELF; pass the corresponding `pk` to `execute` and `prove`. Build each guest in `build.rs`. Full example: [Proving Advanced](./proving_advanced.md#multiple-programs).

---

## 10. Feature Flags

Cargo features for `zisk-sdk`; add under `[dependencies]`. Other features (GPU, stats, diagnostic): [Proving Advanced](./proving_advanced.md#feature-flags).

| Feature | Description |
| --- | --- |
| `disable_distributed` | Disables MPI distributed proving. **Required on macOS** or without OpenMPI. |

Example (macOS or without OpenMPI):

```toml
[dependencies]
zisk-sdk = { git = "https://github.com/0xPolygonHermez/zisk.git", features = ["disable_distributed"] }
```
