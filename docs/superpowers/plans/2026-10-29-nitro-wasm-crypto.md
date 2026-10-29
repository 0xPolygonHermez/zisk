# Nitro runtime: lowering check and crypto imports

Continue in `Core/Worktrees/zisk-wasm-nitro-compat`, branch
`test/nitro-wasm-compat`, based on `f3459a598` and upstream `c8190af8e`.
The user authorized starting implementation after the import-registration slice.
This plan implements the first actual Nitro runtime services. The full replay
milestone remains defined in `../specs/2026-10-29-nitro-wasm-design.md`.

This file records the earlier crypto slice. The subsequent
[`2026-10-29-nitro-wasm-replay.md`](2026-10-29-nitro-wasm-replay.md) implements the
state/Brotli/lifecycle runtime, executes the recorded transfer, and fixes/tests
the five native hint-failure instructions identified below. Assembly execution
and end-to-end proof verification remain outstanding.

## 1. Establish full-module compiler compatibility

Add a diagnostic example which registers exact inspected custom signatures
with trapping bodies. It must never execute the module or imply those bodies
implement Nitro. Lower the recovered historical transfer replay, first one
function at a time, then through the normal complete ROM construction. Keep
existing memory/layout validation and bound host resource use.

Observed: all 14,478 defined functions lower to 20,596,298 instructions. The full
float-enabled ROM builds in 12.356 seconds with peak sampled RSS 12,396,464 KiB.
An initial 8 GiB host limit was insufficient; one 24 GiB / 120-second bounded
run succeeded. Artifact SHA-256:
`15a1bfa995d941d4ec2edc5d9dc3acbe0bab839499144029058944e1f0833521`.
No unsupported operator or ROM-layout failure was found; no speculative
compiler optimization is needed.

The parent reran the diagnostic after crypto integration: exit 0, reported
lowering time 12.243 seconds, total process time 15.929 seconds, peak sampled
RSS 16,997,664 KiB (about 16.2 GiB), within the same 24 GiB / 120-second bound.
The instruction count is unchanged. This still uses trapping custom imports
and does not execute a block. Logs are retained under
`target/nitro-lowering-diagnostic-final{,-resources}.log`.

## 2. Link existing ZisK crypto and implement Nitro's ABI

Reuse `ziskasm::assemble_zisk_library()` and the RISC-V linker's collision
checks/index renumbering. Add only the symbolic library-call support needed by
custom emitters. Assemble/link the library only when an emitted call needs it;
unknown library symbols fail translation. Preserve the existing default API.

Reserve library ROM when linked and place the Wasm stack below library RAM.
Native routines use register 2 as SP, whereas Wasm uses it as FP. Bridge calls
on a distinct bounded native stack, preserve Wasm FP and memory-end register,
and restore the import's return path. Document native-call clobbers and forbid
reentrant callbacks into Wasm. Do not assume the RISC-V ABI preserves the Wasm
register convention. Existing normal and float callers must still pass.

Expose explicit Nitro crypto registrations, keeping Nitro names/semantics
separate from the generic compiler. Implement:

- `arbcrypto.keccak256(i32, i32, i32) -> ()`, using the production library
  sponge. Check unsigned Wasm ranges before native access, including empty
  input and overflowing/negative-as-unsigned pointers. Preserve input/output
  overlap semantics and support unaligned guest pointers.
- `arbcrypto.ecrecovery(i32, i32, i32, i32, i32) -> i32`, using production
  recovery for IDs 0/1. Preserve Nitro's 32-byte hash, 65-byte signature (last
  byte recovery ID), 65-byte uncompressed output and status codes. The pinned
  library cannot recover IDs 2/3: explicitly trap for those IDs and document
  this incomplete support; do not return a false semantic result. Invalid
  lengths and IDs greater than 3 retain Nitro's status behavior.

Every returned crypto result is calculated by instructions in the generated
ROM, with the existing constrained operations. No witness-supplied results or
host callbacks. The resulting ROM/VK, not the Wasm hash alone, identifies the
program. The standard CLI remains unchanged.

## 3. Validate and review

First add failing emulator regressions. Check known Keccak vectors at empty,
135/136/137-byte and multiple-block boundaries; unaligned and overlapping
buffers; repeated/nested calls and preserved Wasm frame/memory bounds. Check
valid recovery and failure statuses, unsupported IDs, output prefix, aliasing,
and bounds. Validate unknown native symbols and disjoint layout/indexes.
Use existing test dependencies and fixtures; add no new crypto implementation.

Run the relevant Wasm, float, import and RISC-V linker tests, formatting and
`git diff --check`. Obtain independent review of the full branch against
`c8190af8e`, then commit only this task's coherent changes. Preserve worktree;
no push, merge, rebase or cross-worktree integration.

## Remaining work at the crypto-slice checkpoint

Authenticated witness decoding, `wavmio` state/preimage/inbox services and
accumulator accounting; pinned Brotli; console/structured-output separation;
successful-exit claim finalization; explicit unsupported Stylus traps; a
deterministic adapter-aware runner. Then replay and match the expected state,
test corrupted witnesses, and separately validate Assembly/proof setup.
This crypto slice cannot yet execute a Nitro block or generate its proof.

## Implementation and test record

Implemented `nitro::crypto_imports()`, lazy production-library linking and a
native call bridge restricted to the two supported crypto targets. Existing
linker collision/index checks are reused. The fixed Wasm stack now starts at
`0xbffc0000`, 128 KiB below its previous top, including for ordinary callers;
linear-memory capacity is unchanged. No external dependency was added.

The new execution tests first failed with trapping placeholder implementations,
then passed with real imports. Eleven regressions cover 192 Keccak boundary,
alignment and alias combinations, nested/repeated calls and memory growth,
pointer traps, recovery statuses and failure ordering, valid recovery, unsupported
IDs, output preservation, library metadata and unique instruction indexes.
The full expected recovered key is checked for ID 1; ID 0 currently checks
success status. A separately pinned ID-0 key is a remaining coverage improvement.

Passed checks:

- `cargo test -p ziskemu --release`: 112 passed, one pre-existing ignored test.
- `ZISK_CI_NO_DOCKER_REBUILD=1 cargo test -p ziskemu --release --features float
  --test wasm --test wasm_float --test wasm_imports --test wasm_nitro_crypto
  --test wasm_example`: 70 passed.
- `cargo test -p zisk-riscv -p zisk-transpiler-wasm --release`, also with
  `--features float` and `ZISK_CI_NO_DOCKER_REBUILD=1`: 24 RISC-V tests passed
  in each configuration; Wasm crate/bin/doc targets passed.
- Scoped `rustfmt --check` and `git diff --check`: clean.
- Parent's bounded full historical-module lowering check: passed as recorded
  above, with trapping custom services; no block execution/proof claim.

Assembly execution, proof generation, forged-hint injection and unrelated
workspace packages were not tested. Recovery IDs 2/3 and the remaining replay
services above are explicit limitations.

Independent whole-branch review against `c8190af8e` approved this bounded slice
with no blocking findings. It identified an inherited pre-proof issue: native
secp256k1 hint-check failure paths in `ziskasm/zisklib/secp256k1/{scalar,field,curve}.zisk`
use plain `end`, which does not mark emulator execution as failed. Before proof
use, establish fail-closed rejection of adversarial hints and bind successful
finalization; test forged hints explicitly. Honest emulator-vector results do
not establish that property. The current change does not modify those paths.
