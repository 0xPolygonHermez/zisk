# ZisK Threat Model (for OSS Scanner)

> Written against ZisK 1.4.0-alpha.

## 1. What ZisK is

ZisK is a zkVM: it executes RISC-V (RV64IMA) guest programs and produces a STARK-based
proof that the execution was correct, with recursive aggregation and an optional final SNARK
wrapper (pilfflonk or PLONK) for cheap on-chain verification. A primary use case is proving
Ethereum blocks (real-time proving), so proofs produced by ZisK may be relied on by third parties
with real economic value at stake. ZisK is in alpha and is undergoing security audits.

Main pipeline:

1. **Guest build**: the guest program is compiled with the ZisK Rust toolchain
   (`riscv64ima-zisk-zkvm-elf`) and linked against `ziskos` (`ziskos/entrypoint`). Floating point
   is emulated by `lib-float` (SoftFloat, embedded as `ziskfloat.elf`). WebAssembly guests are also
   supported (`transpilers/wasm`, `wasm2zisk`).
2. **Transpilation**: `transpilers/riscv` (`elf2rom`) converts the RISC-V ELF into the ZisK ROM and
   merges the `zisklib` routines from `ziskasm`. (`transpilers/riscv2zisk` is a deprecated
   compatibility wrapper.)
3. **Program setup**: `rom-setup` computes the ROM Merkle commitment (the program verification key)
   and generates the native x86-64 assembly used by the ASM runner.
4. **Execution**: the Rust emulator (`emulator`, binary `ziskemu`) and the ASM runner
   (`emulator-asm`: the ROM compiled to native x86-64 code, driven from `emulator-asm/asm-runner`
   over POSIX shared memory) execute the program and produce traces.
5. **Witness generation**: the executor (`executor`) drives the state machines to fill the AIR
   traces defined in PIL (`pil/zisk.pil` instantiates all of them):
   - `state-machines/`: Main (with registers), ROM, Mem (also used for the `InputData` region),
     RomData, MemAlign / MemAlignByte / MemAlignRom, Arith, Binary, BinaryAdd, BinaryExtension and
     the frequent-operations (FROPS) tables.
   - `precompiles/`: DMA (memcpy/memset/compare), Keccakf, Sha256f, Poseidon (Poseidon1 and
     Poseidon2 over Goldilocks) and Poseidon2KoalaBear, Blake2 (Blake2b round, Blake2s), Blake3f,
     ArithEq (256-bit modular arithmetic, secp256k1, secp256r1, BN254), ArithEq384 (384-bit
     modular arithmetic, BLS12-381), BigIntAdd, TwistedEdwards (BabyJubJub, Ed25519) and the EVM
     JUMPDEST-analysis precompile (`precompiles/evm`).
6. **Proving**: pil2-proofman (see section 1.1) builds STARK proofs per AIR
   instance, then aggregation into a `VadcopFinal` proof, optionally compressed, and optionally
   wrapped into a pilfflonk or PLONK SNARK (`prover-backend/src/snark.rs`). `recurser/` aggregates
   several final proofs (optionally restricted to an allow-list of program VKs). Proving may run
   on CPU, GPU or distributed workers (`distributed/`).
7. **Verification**: the native verifier (`verifier/`, `no_std`, also usable inside a guest and in
   WASM), the SDK / `common/src/proof.rs` (`Proof::verify`, `VerifyBuilder`), `cargo-zisk verify`
   and the Solidity verifiers in `zisk-contracts/` check the final proof against the verification
   key, the program verification key and the public values (64 × 32-bit publics).

### 1.1 pil2-proofman

ZisK and [pil2-proofman](https://github.com/0xPolygonHermez/pil2-proofman) are developed by the same
team and form one system: ZisK defines the state machines and drives execution and witness
generation, while pil2-proofman provides the proving and verification machinery. pil2-proofman is
**in scope** as part of ZisK, not as a third-party dependency. The parts that matter most:

- the STARK prover in C++ (`pil2-stark/`, `provers/starks-lib-c/`) and its witness and hint handling
  (`witness/`, `hints/`, `common/`);
- the verifier (`verifier/`) and the recursion/aggregation setup and circuits
  (`setup/stark-recurser/`, `setup/pil2-stark/`, `setup/exps-codegen/`, `pil-info`);
- the PIL2 standard library (`pil2-components/lib/std/`): lookups, permutations, range checks and
  connections that every ZisK state machine relies on;
- the pilfflonk SNARK wrapper (`pilfflonk/`, `setup/pilfflonk/`, `wrap-witness/`).

A finding in pil2-proofman should explain how it affects ZisK proofs (section 3), and point to the
file in pil2-proofman.

## 2. Trust model

- **The prover is untrusted.** The central security assumption is that whoever produces a proof may
  be malicious and may run modified prover, emulator, executor or witness-generation code. Only the
  constraints (PIL/AIR), the setup/verification keys and the verifier are trusted.
- **The verifier trusts only**: the verification key, the program verification key (ROM
  commitment) that it expects, and the public values bound to the proof. A verification that takes
  the program verification key from the proof itself only proves that *some* program ran; callers
  that care which program ran must pin the expected key (`VerifyBuilder::with_program_vk`,
  `verify_saved(.., Some(program_vk))`). `cargo-zisk verify` has no option to pin it and verifies
  against the program verification key carried in the proof file: it is a convenience tool that
  checks a proof is well formed and valid, and it does not tell the operator which program it is
  for. This is known and intended; reports that `cargo-zisk verify` does not pin the program are
  out of scope.
- **Guest inputs are untrusted.** Input data fed to a guest program (e.g. an Ethereum block and
  witness) may be adversarial.
- **Hints and fcalls are untrusted.** Fcalls (free-input CSR reads, `ziskos/.../zisklib/fcalls`)
  and the hints stream (`precompiles/hints`, `ziskos-hints`, `stream/`) are values supplied by the
  host/prover. Guest library code that uses them must check the result, and constraints must
  verify every hinted precompile result.
- **Guest ELF**: chosen by the party that wants something proven, so it is trusted *by the
  verifier* in the sense that the verifier picks which program it accepts proofs for. However, any
  ELF must be handled safely by the transpiler, setup and prover tooling.
- **Distributed proving**: the coordinator ↔ worker gRPC control plane, the client API and the
  QUIC/unix hints/input streams are plaintext and unauthenticated by design; peer authentication is
  delegated to network isolation (see the comment in `stream/src/quic.rs`), and they must be
  deployed on a trusted network. Within that model, worker responses are still treated as untrusted
  by the coordinator (`coordinator/src/coordinator/worker_handlers.rs`). The absence of
  authentication or TLS is out of scope; memory-safety or logic bugs that let a worker or client
  compromise the coordinator (or vice versa) are in scope.
- **Setup generation** (`cargo-zisk-dev proofman-setup`, proving/verification keys) is run by the
  ZisK maintainers on trusted inputs, and users download the result.

## 3. Security properties (in priority order)

1. **Soundness**: it must be infeasible to produce an accepting proof for an execution that is not a
   correct RV64 + ZisK-precompile execution of the committed program on the claimed inputs, or for
   public outputs that differ from those of the correct execution.
2. **Verifier robustness**: the verifier must never crash, hang, or behave incorrectly on arbitrary
   (malformed or adversarial) proof bytes, verification keys or public values.
3. **Completeness**: an honest prover must be able to prove every valid execution within documented
   limits. An adversary must not be able to craft inputs that make honest proving fail or stall.
4. **Guest runtime correctness**: `ziskos`, `zisklib`, `lib-float` and the precompile interfaces
   must behave as documented so that correct guest code is not miscompiled or misexecuted.
5. **Zero-knowledge**: ZisK does not currently claim that its STARK proofs hide the witness (no
   blinding at the STARK/aggregation level); only the final SNARK wrappers are blinded: pilfflonk
   (`pilfflonk/docs/protocol.md#blinding` in pil2-proofman) and PLONK (the rapidsnark prover in
   `pil2-stark/src/rapidsnark/`). Witness leakage through STARK proofs is therefore out of scope,
   while a flaw in the pilfflonk or PLONK blinding (including the test-only
   `insecure_blinding_seed` being usable in a real proof) is in scope. Where the book or README
   describe ZisK as privacy-preserving, this document is authoritative for the scan. Note that the
   guest `sys_rand` is a fixed-seed, insecure RNG by design and prints a warning.

## 4. Where untrusted input enters

- Proof bytes, public values and verification keys passed to the verifier: `verifier/src/decode.rs`,
  `Proof::load` / `from_bytes` in `common/src/proof.rs`, `cargo-zisk verify`, the Solidity
  verifiers and calldata export.
- Guest program inputs (`ZiskStdin`: files, inline, framed `ZiskStream` over unix/QUIC) consumed by
  the emulator, executor and guest.
- ELF (and WASM) files consumed by the transpilers, `rom-setup`, the emulator and the ASM runner.
- Messages between coordinator, workers and clients in distributed proving (gRPC/protobuf, bincode
  proofs decoded by workers, uploaded ELFs and inputs).
- Hints / fcalls provided by the host to the guest.
- POSIX shared memory and semaphores between the ASM runner and the native emulator processes.
- Downloaded release artifacts and keys handled by `ziskup`.

## 5. Areas of highest interest

- **Under-constrained PIL**: columns, lookups, permutations, range checks or selectors that let a
  malicious witness satisfy constraints with incorrect values. Includes bus/connection arguments
  between state machines, memory consistency (read-after-write, alignment, initial/final state,
  `InputData` and `RomData`), register handling, and DMA.
- **Constraint vs. RISC-V spec mismatch**: any instruction (including edge cases: signed division,
  overflow, shifts, dynamic jumps, misaligned access, `x0` writes) where the constraints accept a
  result different from the RV64 specification, and the FROPS tables.
- **Precompile constraints**: Keccakf, Sha256f, Poseidon / Poseidon2KoalaBear, Blake2b/Blake2s,
  Blake3f, ArithEq / ArithEq384 (including curve and field edge cases), BigIntAdd, TwistedEdwards
  and the EVM JUMPDEST precompile accepting incorrect outputs.
- **Public input/output binding**: public values or the program verification key not tied to the
  committed trace, in base, aggregated, compressed, recursive (`recurser/`) or wrapped proofs (a
  previous bug of this class has already been fixed in proofman).
- **Recursion/aggregation**: verification circuits that fail to check something the base verifier
  checks (transcript/Fiat–Shamir inputs, FRI parameters, number of instances, AIR selection,
  program VK allow-lists).
- **Hints and fcalls**: prover-supplied values that reach the proven result without being verified,
  either by constraints or by the guest library code that consumes them.
- **Verifier and deserialization**: memory safety, panics, unbounded allocation on untrusted proofs.
- **Emulator / ASM runner**: divergence between emulator, ASM runner and constraints. An emulator
  bug that the constraints reject is a completeness issue; one that the constraints accept is a
  soundness issue.

## 6. Out of scope

- Benchmarks, examples, documentation and test fixtures.
- Performance issues without a security impact (slow proving, high memory use on valid inputs),
  unless an adversary can trigger them cheaply to block honest proving.
- Crashes in developer tooling that only processes files supplied by the local operator (e.g.
  `cargo-zisk-dev` setup commands, `tools/`, profiling and report tools).
- The RISC-V C extension (compressed instructions), behind the `compressed` Cargo feature (off by
  default), for now.
- Vulnerabilities in third-party dependencies (other than pil2-proofman, see section 1.1), unless
  reachable through ZisK in an exploitable way.
- Missing authentication or encryption in the distributed-proving services, which are designed to
  run on a trusted network (see section 2).
- Theoretical weaknesses of the underlying cryptographic assumptions (field size, FRI conjectures)
  rather than of their implementation.

## 7. Severity guidance

**Critical**
- A demonstrated forged proof: the verifier accepts a proof for an incorrect execution or for
  incorrect public outputs.
- An under-constrained column or constraint with a working end-to-end exploit (a malicious witness
  that passes verification).
- Remote code execution in any network-facing component.

**High**
- A soundness flaw (under-constrained constraint, missing binding, missing recursion check) with a
  clear exploit path but no end-to-end forged proof. Soundness findings without a demonstrated
  forgery are capped at High.
- Memory-safety bugs in the verifier reachable from proof bytes.
- An adversary-controlled guest input that makes honest proving fail for an otherwise valid
  Ethereum block (liveness impact on real-time proving).

**Medium**
- Verifier panics or denial of service on malformed proofs without memory corruption.
- Completeness bugs on specific valid programs or instructions (honest prover cannot prove).
- `ziskos` / precompile interface bugs causing incorrect guest behavior.

**Low**
- Crashes or incorrect behavior in local tooling on malformed operator-supplied files.
- Defense-in-depth issues with no demonstrated impact.

## 8. How to exercise it

The image built from `.oss-scanner/Dockerfile` contains a CPU-only build of ZisK:

- Source and build tree: `/src`, built with
  `cargo build --release --target x86_64-unknown-linux-gnu --features cpu-only`.
- pil2-proofman is cloned at `/pil2-proofman` (the tag matching the version in `Cargo.lock`) and
  ZisK is built against it through `[patch.crates-io]` in `/home/ziskuser/.cargo/config.toml`, so
  changes made in `/pil2-proofman` are picked up by the next ZisK build. This is why the
  `Cargo.lock` files that depend on pil2-proofman appear modified in `/src`; do not include those
  changes in patches.
- Binaries (`cargo-zisk`, `cargo-zisk-dev`, `ziskemu`, `zisk-coordinator`, `zisk-worker`, ...) in
  `/home/ziskuser/.zisk/bin` (on `PATH`), and the ZisK guest toolchain installed in rustup.
- Tests are precompiled. `cargo test --release --target x86_64-unknown-linux-gnu --features cpu-only
  --no-run` (exactly these flags) lists the test executables without recompiling anything; run the
  one for a crate directly, e.g. `target/x86_64-unknown-linux-gnu/release/deps/zisk_core-<hash>`.
  Adding `-p <crate>` changes feature unification and recompiles a large part of the workspace,
  which is slow on a small machine.
- Prebuilt guest ELFs for the emulator: `elf-regressions/`, e.g.
  `ziskemu -e elf-regressions/prebuilt-elfs/cpp_static_init.elf`.
- Guest programs can be built offline with the ZisK toolchain, e.g.
  `cd examples/fibonacci/guest && cargo-zisk build --release`.
- The proving key is not installed: proving needs far more memory than the analysis environment
  has. Prefer emulator runs, unit tests and constraint-level reasoning (PIL files under `pil/`,
  `state-machines/*/pil`, `precompiles/*/pil`).

## 9. How we would like reports

- State which property from section 3 is broken and why.
- For constraint issues: point to the exact PIL file and constraint/column, and explain what
  incorrect witness satisfies it.
- Provide a reproducer whenever possible: preferably a test or a small guest program plus the
  command sequence that shows an invalid witness or proof being accepted.
- Patches should be minimal and focused: fix the constraint or check, add a regression test, and
  avoid unrelated refactoring or formatting changes. Note if a fix changes the setup or verification
  keys, since that requires regenerating them.

## 10. Contacts

- Security contact: security@silentsig.build
- Disclosure policy: [SECURITY.md](../SECURITY.md). Reports go through GitHub private vulnerability
  reporting; there is no bug bounty.
