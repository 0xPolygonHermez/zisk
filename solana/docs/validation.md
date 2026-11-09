# Validation record — 2026-11-09

Scope: the fixed ZisK v1.4.0-alpha BLAKE3 leaf Pilfflonk profile. This is not a
production sign-off or support for other keys, recursion roots or aggregated
proofs.

## Current checks

`bash scripts/check.sh` passes on macOS. It reproduces
the fixture/profile, checks formatting and Clippy, runs the JS reference,
3 unchanged Solidity tests, 9 Rust tooling tests, 9 Rust verifier tests, 4 client
tests and 11 compiled-SBF tests, then enforces the CU/packet regression limits.
The host-tool port was checked with Rust 1.95.0 and Node 22.15.1; CI remains
pinned to Node 26.12.1.

The compiled tests cover all 211 proof-word mutations, late pairing failure,
off-curve coordinates, swapped openings, changed outputs, malformed
accounts/instructions, upload authority, sealing, close/recreate, and rent return.
They also check:

- Both compiled programs load with SBPF v3 disabled.
- Stateless verification does not produce a saved result.
- Invalid verification cannot create or change a verified result.
- Saved results bind the expected guest and complete output bytes.
- Consumer CPI failure creates no receipt.
- Both consumer modes authenticate the verifier and its linked loader metadata.
- A remaining upgrade authority and substituted ProgramData are rejected.
- Permanent receipts prevent replay, including a new upload of the same proof.
- A prefunded receipt PDA can be initialized; missing signatures and wrong
  receipt addresses are rejected.
- Saved-result consumption stays below 50,000 CUs.

The new deployment test was first run against the v3 ELF and failed with
“Detected sbpf_version required by the executable which are not enabled”.
It passes after the v2 rebuild. Guest-selection and record tests also failed
against the previous behavior before their fixes.

The host tools are Rust, including the profile generator, benchmark checks,
EVM runner and fuzz-seed builder. The generator reproduces every byte of the
previous arithmetic output apart from its filename in the first comment.
The compiled verifier/consumer hashes and compute measurements are unchanged;
the EVM runner reproduces every field of the recorded benchmark report.
Tool tests cover unsupported syntax, canonical scalar bounds, missing or
duplicate measurements, resource limits, fixed seed bytes and build diagnostics.

No proof equations, scalar backend, transcript or reference fixtures changed.
Positive and negative comparisons remain evidence, not a proof that every
cryptographic edge case is covered.

## Real local validator

`node tools/test_validator.mjs` passes against Agave 2.2.21. It creates an
isolated loopback ledger, starts the actual compiled verifier and consumer,
runs the real uploader through RPC, and stops only that validator afterwards.
The consumer is loaded at its configured test ID; no matching deployment
private key is needed for local genesis setup.

The test completes create, eight appends, seal, verify-and-record, standalone
verification, direct consumer CPI, later consumption and close. It checks
wrong guest/output claims, an upgradeable verifier, replay, and unchanged
receipts after rejection. Later consumption is submitted with a 50,000-CU
limit. No public-cluster transaction or deployment is performed.

[validator.json](../benchmarks/validator.json) records measurements and ELF
hashes. [validator-features.json](../benchmarks/validator-features.json) records
the inactive gates queried from mainnet using the pinned old CLI. The local
validator disables those gates instead of using all features. This reproduces
features known to Agave 2.2.21, not full current-mainnet runtime equivalence.

The v3 compatibility finding needs this qualification: Agave 2.2.21 queries
`C8XZNs1bfzaiT3YDeXZJ7G5swQWQv7tVzDnCxtHvnSpw`, which was inactive on
mainnet and devnet. Agave 4.2.2 defines the gate as
`5cC3foj77CWun58pC51ebHFUWavHWKarWyR5UUik7dnC`; direct account queries
found that feature active at mainnet slot 428976000 and devnet slot 461808000.
The new build targets v2 for tested older-validator compatibility. It is
incorrect to infer from the old feature ID alone that no live cluster runs v3.
LiteSVM 0.16 uses a bundled mainnet feature snapshot, not an all-features default.

## Compute and resource measurements

[integration.json](../benchmarks/integration.json) contains current LiteSVM
measurements and hashes; the README compares them with the real validator.
Verification and record creation take 1,375,268 CUs. The later consumer uses
12,468 CUs on the validator, leaving the rest of a separate transaction for
application logic. Direct consumer CPI remains close to the 1.4M ceiling.

The 1M-CU verification target is still unmet. [profile.json](../benchmarks/profile.json)
is historical v3 instrumentation: Q cost 757,573 CUs and R cost 184,908 CUs.
Those figures identify field arithmetic as the main optimization candidate;
they are not fresh stage measurements of the v2 build. Curve syscalls are not
the dominant measured cost. Arithmetic bounds remain in [scalar.md](scalar.md).

A proof has 6,752 bytes. The new buffer is 6,829 bytes and reserves 48,420,720
lamports at the measured rent settings, returned on close. Eight append
transactions upload it. Largest signed transactions were 1,150 bytes in
LiteSVM and 1,149 bytes through the real client, below 1,232 bytes.
The permanent 32-byte consumption receipt reserves another 1,113,600 lamports;
retaining it is necessary for this example's replay policy.

Execution uses the normal stack/heap and compute ceilings. No exact stack or
allocator high-water measurement was made. Build checks still reject LLVM
stack-frame errors even when the compiler exits successfully.

## Historical evidence

The previous version passed the full suite in a clean Ubuntu 24.04.5 x86_64
container under Docker emulation on Apple silicon. [linux.json](../benchmarks/linux.json)
retains that version's source revision and artifact hashes. This current
account/consumer revision has not been rerun in that clean Linux environment.

[fuzz.json](../benchmarks/fuzz.json) records earlier campaigns: 3,944,810
proof/arithmetic executions and 4,012,854 buffer-state executions. Those
campaigns predate the new record format and do not claim coverage of it.
The current smoke command is `bash scripts/fuzz.sh 60`; the updated state model
covers the new buffer layout. Cryptographically valid saved-result and consumer
replay transitions are covered by compiled-SBF and real-validator tests, not
by the random upload-state model.

The Rust-tool smoke passed 113,178 proof/arithmetic and 119,372 upload-state
executions in 11 seconds per target (10 seconds requested).

Before the host-tool port, a smoke passed 616,747 proof/arithmetic and 512,413
upload-state executions in 61 seconds each. After adding malformed record requests to the
state model, another run passed 101,085 executions. That requested 60-second
run took 553 seconds and reported one 543-second slow input. The retained input
replayed successfully in 0 ms; the wall-clock delay was not reproduced as input
execution cost. No crash or model disagreement occurred.

Historical EVM gas evidence remains in [evm.json](../benchmarks/evm.json).
The EVM source and proof are unchanged. Gas and CUs are not interchangeable.

## Remaining gates

- Two more independently generated proofs with distinct outputs and confirmed
  key identity. One real release proof plus mutations is not broad proof coverage.
- The 1M-CU target and broader proof/application cost measurements.
- External cryptographic audit of the generated verifier and custom field backend.
- Deployment against the intended live cluster and verification of its runtime
  support, trusted setup, initial binary and program ownership history.
- Application-specific output semantics, beneficiary/action authorization and
  any committed nonce/expiry. The example is a permissionless statement registry
  with once-per-statement consumption, not a payment authorization scheme.
- Clean Linux validation of this revision. The separate real-validator test is
  available locally; CI currently runs the LiteSVM and fuzz suites.

The format changes from `ZKPF0001` to `ZKPF0002`; old buffers cannot be reused.
The consumer ABI and receipt model also change. See [wire.md](wire.md) before
integrating. Keep the proof buffer until consumption, then close it; the replay
receipt persists.
