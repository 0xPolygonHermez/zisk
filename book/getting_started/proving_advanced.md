# Proving Advanced

ASM backend options, standalone verification, multiple programs per prover, GPU and logging, ProofOpts in depth, proof bundles, and optional feature flags. Core reference: [Builder and Types](./builder_and_types.md).

---

## Table of Contents

1. [ASM-Specific Options](#1-asm-specific-options)
2. [Standalone Verification](#2-standalone-verification)
3. [Multiple Programs](#3-multiple-programs)
4. [Advanced Builder Options](#4-advanced-builder-options)
5. [ProofOpts in Depth](#5-proofopts-in-depth)
6. [Saving and Loading Proof Bundles](#6-saving-and-loading-proof-bundles)
7. [Feature Flags](#7-feature-flags)

---

## 1. ASM-Specific Options

Available when the ASM backend (`.asm()`) is selected. Not applicable with `.emu()`. Cluster setup: [Distributed Execution](./distributed_execution.md).

| Method | Description |
| --- | --- |
| `.asm_path(PathBuf)` | Path for ASM artifacts. Exposed on the builder but not currently used by the SDK `build_asm()` path. |
| `.base_port(u16)` | Base port for the ASM microservice (default: 23115). The ASM backend uses three consecutive ports from this value. |
| `.unlock_mapped_memory(bool)` | When `true`, memory-mapped ROM files are not locked into RAM. For limited memory. |

---

## 2. Standalone Verification

Verify proofs without constructing a prover (e.g. verifier services, custom CLIs). Default keys: `~/.zisk/provingKey`, `~/.zisk/provingKeySnark`. Custom paths: `verify_zisk_proof_with_proving_key`, `verify_zisk_snark_proof_with_proving_key`.

Program VK from ELF: `prover.vk(&ELF)?` with a prover instance, or `get_program_vk(&elf)?` / `get_program_vk_with_proving_key(&elf, path)` without. The `elf` argument is any `ElfBinaryLike` (e.g. `ElfBinary` from `include_elf!` or `ElfBinaryFromFile`).

```rust
use zisk_sdk::{verify_zisk_proof, verify_zisk_snark_proof};

// STARK (VadcopFinal or VadcopFinalCompressed)
verify_zisk_proof(&proof, &publics, &program_vk)?;

// SNARK
verify_zisk_snark_proof(&proof, &publics, &program_vk)?;
```

---

## 3. Multiple Programs

One `ProverClient` (one `.build()`) can serve multiple guest programs. Call `setup` once per ELF; pass the corresponding `pk` to `execute` and `prove`. In `build.rs`, build each guest (e.g. `build_program("../guest")`, `build_program("../guest_2")`). Full example: `examples/multiple-programs/`.

```rust
use zisk_sdk::{include_elf, ElfBinary, ProofOpts, ProverClient, ZiskIO, ZiskStdin};

pub const ELF: ElfBinary = include_elf!("fibonacci-guest");
pub const ELF2: ElfBinary = include_elf!("fibonacci-guest-2");

let client = ProverClient::builder().build()?;

let (pk, vkey) = client.setup(&ELF)?;
let (pk2, vkey2) = client.setup(&ELF2)?;

let result = client.execute(&pk, stdin.clone())?;
let vadcop_result = client.prove(&pk, stdin).with_proof_options(ProofOpts::default().minimal_memory()).run()?;
client.verify(vadcop_result.get_proof(), vadcop_result.get_publics(), &vkey)?;

let stdin2 = ZiskStdin::new();
stdin2.write(&n2);
let result2 = client.execute(&pk2, stdin2.clone())?;
let vadcop_result2 = client.prove(&pk2, stdin2).with_proof_options(ProofOpts::default().minimal_memory()).run()?;
client.verify(vadcop_result2.get_proof(), vadcop_result2.get_publics(), &vkey2)?;
```

---

## 4. Advanced Builder Options

Chain before `.build()` for distributed proving, GPU, or custom logging. Core options: [Builder and Types](./builder_and_types.md#1-proverclient-and-the-builder).

| Method | Description |
| --- | --- |
| `.shared_tables(bool)` | Share lookup tables across AIRs. Used in distributed proving. |
| `.gpu(Option<ParamsGPU>)` | GPU acceleration. Pass `None` for CPU only. Requires the `gpu` feature. |
| `.logging_config(LoggingConfig)` | Custom logging configuration. |

---

## 5. ProofOpts in Depth

- **`.minimal_memory()`** — Reduces RAM use during proving at the cost of speed.
- **`.no_aggregation()`** — Individual sub-proofs without final aggregation.
- **`.save_proofs()`** — Writes intermediate and final proofs to disk (under `.output_dir()` if set).
- **`.verify_proofs()`** — Verifies each proof immediately after generation.

---

## 6. Saving and Loading Proof Bundles

Full bundle (proof + publics + program VK): `result.get_proof_with_publics()` then `.save(path)`. Load with `ZiskProofWithPublicValues::load(path)?`; pass `.get_proof()`, `.get_publics()`, and the VK to `prover.verify(...)`.

```rust
use zisk_sdk::ZiskProofWithPublicValues;

let bundle = result.get_proof_with_publics();
bundle.save("bundle.bin")?;

let loaded = ZiskProofWithPublicValues::load("bundle.bin")?;
prover.verify(loaded.get_proof(), loaded.get_publics(), &vk)?;
```

---

## 7. Feature Flags

Optional Cargo features for `zisk-sdk`; add under `[dependencies]`. `disable_distributed` is documented in [Builder and Types](./builder_and_types.md#10-feature-flags). The rest:

| Feature | Description |
| --- | --- |
| `gpu` | GPU acceleration (NVIDIA CUDA). Enables `packed`. Build on the target machine for optimal kernels. |
| `packed` | Packed Goldilocks field representations. Enabled automatically by `gpu`. |
| `stats` | Detailed execution statistics (opcode counts, memory access patterns). |
| `diagnostic` | Diagnostic mode in the proof manager for debugging proof generation failures. |
