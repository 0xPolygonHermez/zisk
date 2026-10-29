# Nitro certificate rejection and inbox extension

Continue the authorized Nitro implementation in the existing isolated worktree
`zisk-wasm-nitro-compat`, branch `test/nitro-wasm-compat`, from `e2a2fa19b`.
Spec: `../specs/2026-10-29-nitro-wasm-design.md`.

The user requested another implementation agent, wider Nitro support and a fix
for the newly found CustomDA omission path. The next extension chosen here is
multiple delayed-inbox checkpoints; Stylus, KZG and recovery IDs 2/3 remain
separate work. No branch integration or changes to other worktrees.

## Contract

- Every `validateCertificate` call traps until certificate validity can be
  authenticated. Missing witness is not evidence that a certificate is invalid.
  Ordinary Keccak/SHA preimage resolution retains its existing behavior.
- Each consumed sequencer message folds the delayed accumulator at its declared
  `afterDelayed` count, derived from the starting anchor and actually consumed,
  contiguous delayed messages. No host-selected intermediate accumulator.
- Preserve count/overflow checks, exactly-once folding, ordered windows, complete
  delayed consumption coverage and the existing 256-byte public layout.
- Start anchors still require external L1/binder authentication. No proof claim.

## Tasks and ownership

1. Parent: add failing host and VM regressions for absent/present certificate
   requests, including a caller that would otherwise accept an empty batch.
   Run the failing tests, replace existence-based certificate responses with a
   trap in `runtime/state.c`, then rebuild and run the same tests.
2. Inbox agent: design and implement multiple checkpoint handling in
   `runtime/state.c:finalize` after the parent hands over that file. Add dedicated
   host/VM controls with independently expected complete claims, repeated and
   reordered reads, missing/gapped/out-of-range checkpoints and count overflow.
   Demonstrate a positive two-checkpoint control failing on the old restriction
   before changing production code. Parent coordinates shared builds.
3. Parent: run combined host/VM and relevant regression suites, recorded transfer
   and invalid-input controls; inspect code, obtain independent review, update
   runtime docs/artifact identities and sample bundle, then commit coherent
   changes. Keep the worktree for follow-up; do not push or merge.

## Review focus

An absent or present CustomDA witness must never turn unsupported authentication
into an empty-batch result. Check multiple snapshots at start/intermediate/end
counts, count zero, monotonic checkpoints, unused supplied messages and duplicate
reads. Every snapshot must be derived from the committed consumed window and
the final consumed delayed count must be covered. Rebuilding the helper changes
the exact program identity even if the recorded transfer's public bytes match.

## Validation commands

Native state tests use the existing `clang` sanitizer command in
`runtime/witness-format.md`. Rebuild with `python3 transpilers/wasm/runtime/build.py`.
Run `cargo test --locked --release -p ziskemu --features float` for affected test
targets, explicitly selecting ignored runtime controls. Use the existing
`nitro_replay` example and recorded fixtures for all-256-byte positive comparison
and invalid-input rejection. Run formatting and `git diff --check` before commit.

Assembly execution and proof verification require the previously requested
Linux x86-64 environment; they are not replaced by emulator checks.

## Results

- Both host and VM controls first failed on the old code: absent CustomDA
  certificate validation completed an empty-batch claim, and multiple delayed
  checkpoints trapped. Both now enforce the intended contract.
- Native state controls pass with AddressSanitizer and UndefinedBehaviorSanitizer,
  including the recorded transfer's complete claim crosscheck.
- All 22 explicitly selected runtime VM tests pass. The broader float-enabled
  emulator suite passes 128 tests with 24 intentionally ignored controls; 22 of
  those controls were run explicitly above.
- Independent review found no remaining defect in this bounded change. The
  complete fixed claim vectors were also regenerated with the independent Python
  Keccak oracle and the inbox packing rules.
- The recorded transfer matches all 256 expected bytes in 894,969,877 steps;
  corrupt/missing preimages and the wrong count-start anchor all trap. Exporting
  the exact linked ELF and reloading it through `ziskemu` reproduces the claim.
- Runtime and linked-program digests and current measurements are recorded in
  `transpilers/wasm/runtime/README.md`. Local sample bundles under
  `target/nitro-proof-samples-*` retain the source revision and artifact hashes.

Actual certificate authentication remains unsupported: all certificate calls
trap, including calls with present ordinary hash entries. Multiple inbox
checkpoints have synthetic VM coverage; the recorded transfer uses one sequencer
batch and no delayed messages. Assembly execution, proof generation/verification
and L1/binder authentication remain pending.
