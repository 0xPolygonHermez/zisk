# Run Nitro on upstream ZisK Wasm

## Intent and evidence

Extend the upstream Wasm implementation at
`c8190af8edf783d513266accaa889572cef3dca1` so Nitro's own replay program can
eventually execute and prove Arbitrum transitions. The user asked to scope this
work and then start it. This is follow-up work in
`Core/Worktrees/zisk-wasm-nitro-compat`, branch `test/nitro-wasm-compat`.

The unmodified float-enabled emulator builds. Integer/WASI and f64 controls
execute with exact expected public bytes. The historical transfer replay and
four archived replay binaries fail at custom import resolution by default.
After explicitly registering diagnostic trapping imports, every one of the
historical replay's 14,478 defined function bodies lowers, and complete ROM
construction succeeds under the existing layout bounds. That diagnostic emits
20,596,298 instructions for the defined bodies alone; the final integration
check reaches about 16.2 GiB sampled peak RSS. This establishes compiler compatibility, not execution:
Nitro runtime behavior, execution cost and proof generation remain untested.

The historical replay has SHA-256
`15a1bfa995d941d4ec2edc5d9dc3acbe0bab839499144029058944e1f0833521` and imports
25 functions across `arbcrypto`, `wavmio`, `arbcompress`, `hooks`, and `programs`,
in addition to WASI. The witness is still the previous guest's rkyv format.

## Scope and milestones

### 1. Upstream import extension (first implementation slice)

Provide a library API for explicitly registered custom imports whose bodies
are generated ZisK instructions. Preserve `wasm2rom(bytes)` and the standard
CLI's behavior. The extension is a compilation mechanism, not an emulator host
callback or a completed Nitro adapter.

Use a static slice of registrations, not a plugin loader or trait framework:

```rust
pub struct Import<'a> {
    pub module: &'a str,
    pub name: &'a str,
    pub params: &'a [ValKind],
    pub results: &'a [ValKind],
    pub emit: fn(&mut Code),
}
pub fn wasm2rom_with_imports(
    bytes: &[u8], imports: &[Import<'_>],
) -> Result<ZiskRom, Box<dyn std::error::Error>>;
```

The engine matches both module and function name and validates the exact
parameter and result types before calling an emitter. Unknown imports remain
errors. Duplicate registrations and registrations in the reserved
`wasi_snapshot_preview1` namespace are errors. Duplicate import declarations
inside a Wasm module may use the same registration.

The engine emits the normal function prologue and epilogue around each body.
Parameters are in `local_offset(i)`; a returning body places its result in
`REG_RET`. i32 results are normalized to the engine's sign-extended convention.
Bodies preserve `REG_FP`, `REG_MEM_END` and frame header slots and must check any
guest pointer before memory access. The emitter is trusted compiler code, not
sandboxed application code. It runs during translation; it must not consume
private witness data. Its generated instructions execute as part of the ROM.

This is an implementation choice made by the builder: it follows the existing
WASI instruction-emission pattern, keeps Arbitrum-specific semantics outside
the generic compiler, and adds no dependencies. A proof consumer must bind the
exact resulting ROM/program verification key; Wasm bytes alone do not identify
an import implementation. Setup and execution must use the same registrations.

Acceptance: real emulator tests exercise arguments, results, nested/indirect
calls, repeated imports, i32 normalization and failure/trap behavior. Negative
tests cover unknown imports, wrong signatures, duplicate registrations and
attempted WASI overrides. Existing Wasm and float tests still pass. This slice
does not claim a Nitro block runs.

### 2. Authenticated single-transfer replay

Implement a concrete adapter using the extension, reusing the old adapter's
semantics rather than porting all ArbOS rules into Rust:

- Define bounded witness decoding and private input access for the recorded
  start state, inbox messages and preimages. Missing required data fails.
- Implement `wavmio` state reads/writes and inbox/preimage access with bounds,
  hash authentication, contiguous counts and accumulator checks.
- Implement `arbcrypto` Keccak and signer recovery and Nitro-compatible Brotli
  compression/decompression. Prefer existing ZisK routines where their calling
  convention, scratch memory and proof support can be verified. Do not promise
  that Rust helpers can be directly called from the Wasm frame convention.
- Match required WASI behavior and `hooks.beforeFirstIO`. For the first
  non-Stylus milestone, explicitly trap on unsupported `programs` calls; a
  successful transfer must never reach them.
- Separate console logging from a structured 256-byte execution claim. Bind
  replay/adapter identity, start/end GlobalState and inbox claim hashes.
- Wire a deterministic adapter-aware execution entry point. Check the complete
  Nitro lowering, ROM window, stack and memory limits once imports are supplied.

Acceptance: the recorded transfer ends at block hash
`a67bfee73eec54ea031ebe065a14f2a34f7a783a4e9cf57d1060eb730cfb76bc`, zero send
root, batch 5, position 0. Public bytes match the specified claim. Corrupted used
preimages, missing required data and invalid counts fail; changed inbox data
changes the committed claim and fails against the L1 anchor. Report measured
steps, cost and memory. This milestone still requires a separate proof check.

### 3. Proof integration and wider coverage

- Enable and verify Wasm Assembly setup/execution, starting with small controls.
  Inspect all program-loading paths; changing one ELF guard is insufficient.
- Generate and verify a real transfer proof under the new program identity.
- Make native crypto hint failures fail closed and test adversarial hints:
  review found inherited secp256k1 failure paths using plain `end` without an
  emulator error. Establish successful finalization before relying on proofs.
- Connect existing L1 storage verification and binder logic; pin new keys and
  public layouts, test rejected mismatches, then demonstrate wrapping and the
  intended settlement verifier. Preserve the existing L1 finality assumptions.
- Add full Stylus execution and metering, blob/KZG authentication, broader
  Nitro corpus coverage and ranges with explicit inbox continuity.
- Benchmark actual proving time and introduce acceleration only where measured.

Each later milestone needs a concrete implementation plan after the previous
one resolves its blockers. They remain project scope, not claimed deliverables
of the initial import API change.

The first runtime implementation is planned in
`../plans/2026-10-29-nitro-wasm-crypto.md`: reuse the assembled ZisK library
through a safe native call bridge, starting with Keccak and recovery IDs 0/1.
Recovery IDs 2/3 require explicit traps until the native implementation supports
them. These crypto services alone do not complete the single-transfer milestone.

## Trust boundary

The eventual theorem is: the pinned Nitro replay plus pinned runtime maps the
committed start state to the committed end state using authenticated state data
and exactly the committed contiguous inbox windows. The L1/binder layer binds
those windows to the accepted Ethereum anchor. Host input is private witness,
not trusted execution. Crypto, compression and state updates must execute in
the proved program or be verified by supported proof-constrained operations.

No unchecked callbacks, success stubs, witness-selected program identities,
or bypassed pointer/hash/signature checks are acceptable. Unknown/unsupported
behavior must fail. Existing worktrees, toolchains and deployments remain
unchanged; no merges, rebases, pushes or cross-worktree integration.

## Files and consumers for the first slice

- `transpilers/wasm/src/imports.rs`: registration validation and emitted call
  wrapper; new public `Import` type.
- `transpilers/wasm/src/lib.rs`: additive entry point and import resolution;
  existing `wasm2rom` delegates with an empty registration slice.
- `emulator/tests/wasm_imports.rs`: execution and rejection tests.
- `transpilers/wasm/README.md`: usage, calling convention, identity requirements
  and unchanged CLI/Nitro/proof limitations.

Existing callers in `transpilers/common`, `wasm2zisk`, emulator tests and
downstream SDK/executor paths continue to use the default entry point. No
dependency, input format, CLI flag, existing fixture or export is removed.
