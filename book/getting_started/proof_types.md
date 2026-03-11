# Proof Types

Three proof types: every proof starts as a Vadcop STARK; from there the pipeline can produce either a compressed STARK or a Plonk SNARK, not both for the same run. CLI: `--compressed` and `--snark` are mutually exclusive. SDK: if both `.compressed()` and `.plonk()` are chained on the `ProveBuilder`, the last call wins. Flow: [Proving Workflow](./proving_workflow.md). Reference: [Builder and Types](./builder_and_types.md).

---

## Pipeline

The prover pipeline (`ProverEngine` in `sdk/src/prover/`) runs in stages. Compression and SNARK are alternative paths from the same STARK; one is chosen at prove time.

1. **Witness generation**. The emulator (or ASM backend) executes the RISC-V ELF with the given input and produces the execution trace.
2. **STARK proof**. The proof manager takes the trace and generates a Vadcop STARK proof. If aggregation is enabled (`-a` or `.aggregation(true)`), sub-proofs are aggregated into a single final proof.
3. **Compression** _(optional)_. When compressed output is requested, the proof manager applies recursive composition and produces a `VadcopFinalCompressed` proof.
4. **SNARK** _(optional)_. When SNARK mode is selected (prover built with `.snark()` and `.plonk().run()` at prove time), the prover generates a Plonk proof via the SNARK wrapper. Separate proof mode, not a conversion of an existing STARK.

Compression and SNARK both start from an uncompressed STARK; only one path applies. A SNARK cannot be compressed; a compressed STARK is not converted to SNARK. The CLI enforces this:

```
Compressed proofs are not supported for SNARK generation.
```

---

## STARK (`ZiskProof::VadcopFinal`)

The default proof type. It is the fastest to generate and produces the largest output. No special flags are required.

### SDK

Call `prove` and then `.run()`:

```rust
let result = prover.prove(&pk, stdin).run()?;
```

### CLI

```bash
cargo-zisk prove -e guest.elf -i input.bin -a
```

The `-a` flag enables aggregation (sub-proofs are combined into one final proof).

### Verification

Use `verify_zisk_proof` for both STARK and compressed STARK; it detects the proof variant and uses the appropriate verification key.

```rust
use zisk_sdk::verify_zisk_proof;
verify_zisk_proof(&proof, &publics, &program_vk)?;
```

```bash
cargo-zisk verify -p ./proof/vadcop_final_proof.bin
```

---

## Compressed STARK (`ZiskProof::VadcopFinalCompressed`)

Recursive composition is applied to the STARK to produce a smaller proof. Verification is the same as for STARK: `verify_zisk_proof` handles both variants and fetches the compressed verification key when needed.

### SDK

```rust
let result = prover.prove(&pk, stdin).compressed().run()?;
```

### CLI

```bash
cargo-zisk prove -e guest.elf -i input.bin -a --compressed
```

### Verification

Same as STARK, `verify_zisk_proof` or `cargo-zisk verify`.

---

## SNARK (`ZiskProof::Plonk`)

A Plonk proof. You must have the SNARK proving key installed. When building the prover, call `.snark()` on the builder; when generating a proof, call `.plonk()` on the `ProveBuilder` (e.g. `prover.prove(&pk, stdin).plonk().run()?`). For the CLI, use `-w` / `--proving-key-snark` to set the SNARK proving key path; verification uses the `verify-snark` command with `-p` (proof) and `-k` (verkey). For verification in a Solidity contract, see [On-Chain Verification](#on-chain-verification).

### SDK

Build the prover with `.snark()`, then use `.plonk().run()` when calling `prove`:

```rust
let prover = ProverClient::builder()
    .emu()
    .prove()
    .snark()
    .build()?;

let (pk, vk) = prover.setup(&ELF)?;
let result = prover.prove(&pk, stdin).plonk().run()?;
```

The resulting proof is `ZiskProof::Plonk(Vec<u8>)`.

### CLI

```bash
cargo-zisk prove -e guest.elf -i input.bin -a --snark
```

The `-w` / `--proving-key-snark` flag sets a custom SNARK proving key path (default: `~/.zisk/provingKeySnark`). The STARK proving key path is `-k` / `--proving-key`.

### Verification

SNARK proofs use a separate verification function and CLI command. Verification reformats the public values to Solidity-compatible big-endian format, hashes them with SHA-256, and passes the hash to the Plonk verifier.

```rust
use zisk_sdk::verify_zisk_snark_proof;
verify_zisk_snark_proof(&proof, &publics, &program_vk)?;
```

```bash
cargo-zisk verify-snark -p ./proof/snark_proof.bin -k path/to/verkey.json
```

Here `-p` / `--proof` is the proof file and `-k` / `--verkey` is the verification key (e.g. the `.verkey.json` from the SNARK proving key).

---

## On-Chain Verification

ZisK provides a Solidity verifier contract for SNARK proofs; the source lives in `zisk-contracts/`. The caller supplies `rootCVadcopFinal` (and the other arguments) as calldata; the contract does not validate this value itself. Use `getRootCVadcopFinal()` on the contract to obtain the value to pass. Encode public values with `ZiskPublics`: `.public_bytes_solidity()` for big-endian u32 format, and `.bytes_solidity(program_vk, vadcop_verkey)` for the full byte sequence that the contract hashes.

### Interface (`IZiskVerifier.sol`)

```solidity
interface IZiskVerifier {
    function verifySnarkProof(
        uint64[4] calldata programVK,
        uint64[4] calldata rootCVadcopFinal,
        bytes calldata publicValues,
        bytes calldata proofBytes
    ) external view;
}
```

### How it works (`ZiskVerifier.sol`)

1. The contract hashes `programVK || publicValues || rootCVadcopFinal` with SHA-256.
2. The hash is reduced modulo the BN254 scalar field (`21888242871839275222246405745257275088548364400416034343698204186575808495617`).
3. The proof bytes are decoded as `uint256[24]` (polynomial commitments and evaluations).
4. The reduced hash and decoded proof are passed to the Plonk verification logic (`PlonkVerifier.sol`, generated by snarkJS).
5. The contract reverts with `InvalidProof()` on failure.

The caller must pass `rootCVadcopFinal` (and `programVK`, `publicValues`, `proofBytes`) as calldata. The contract exposes `getRootCVadcopFinal()` so that callers (e.g. a fault dispute game) can obtain the correct value to pass.

---

## Summary

| Proof type        | SDK method           | CLI flag      | Verification (SDK)        | Verification (CLI)   | On-chain |
| ----------------- | -------------------- | ------------- | ------------------------- | ------------------- | -------- |
| STARK             | `.run()`             | _(default)_   | `verify_zisk_proof`       | `cargo-zisk verify` | No       |
| Compressed STARK  | `.compressed().run()`| `--compressed`| `verify_zisk_proof`       | `cargo-zisk verify` | No       |
| SNARK (Plonk)     | `.plonk().run()`     | `--snark`     | `verify_zisk_snark_proof` | `cargo-zisk verify-snark` | Yes  |

**Constraints:**

- `--compressed` and `--snark` cannot be combined.
- SNARK requires `.snark()` on the **ProverClientBuilder** (when building the prover) and `.plonk()` on the **ProveBuilder** (when calling `prove`), plus the SNARK proving key installed.
- `verify_zisk_proof` handles both STARK and compressed STARK (auto-detects).
- `verify_zisk_snark_proof` verifies Plonk SNARK proofs.
