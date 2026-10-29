# Nitro Wasm Import Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a library caller supply typed custom Wasm imports that execute as ZisK instructions, as the first step toward Nitro replay.

**Architecture:** Add a small explicit registration slice beside the existing WASI import emitter. Keep the default API and CLI unchanged; the engine validates signatures and wraps custom bodies in its existing call convention.

**Tech Stack:** Rust, existing `wasmparser`/`wat`, ZisK ROM emitter and emulator; no new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-29-nitro-wasm-design.md`

## Global Constraints

- Base: `c8190af8edf783d513266accaa889572cef3dca1`.
- Work only in `Core/Worktrees/zisk-wasm-nitro-compat` on `test/nitro-wasm-compat`.
- Unknown imports fail; WASI cannot be overridden; duplicate registrations fail.
- Imported bodies are trusted translation code that emits proved instructions, never runtime host callbacks.
- No Nitro execution/proof claim from a library API test. No new dependencies or CLI behavior changes.
- Preserve the worktree. No cross-worktree integration, pushes, merges or rebases.

## Review Focus

- A misspelled/unregistered function in a known module must fail, never become a no-op.
- Wrong parameter/result types must fail before body emission.
- Repeated declarations and nested/indirect calls must preserve frames and return values.
- i32 results must have the same sign-extended form as ordinary Wasm i32 operations.
- Traps must fail execution rather than publish a successful result; WASI overrides and ambiguous registrations must fail.

## Task 1: Typed custom imports with executable tests

**Files:** create `transpilers/wasm/src/imports.rs` and `emulator/tests/wasm_imports.rs`; modify `transpilers/wasm/src/lib.rs` and `transpilers/wasm/README.md`.

**Interfaces:** consumes `Code`, `ValKind`, `local_offset`, frame/register constants and `wasi::build_wasi_stub`; produces `Import<'a>` and `wasm2rom_with_imports(bytes: &[u8], imports: &[Import<'_>]) -> Result<ZiskRom, Box<dyn Error>>` exactly as specified.

- [x] Add real emulator tests using tiny WAT modules: custom i64 arithmetic returns 42; nested and indirect calls preserve a caller local; duplicate declarations resolve; a void hook returns; i32 all-ones is normalized; deliberate trap fails.
- [x] Add rejection tests for unknown module/name, parameter/result mismatch, duplicate registrations and reserved WASI registrations. Check signature failures do not invoke the emitter.
- [x] Run the new tests before implementation; record missing API failures, then add the API shell while keeping default rejection and observe the positive execution tests fail at custom imports.
- [x] Implement registration validation, exact lookup/signature checks, normal frame prologue/epilogue and i32 normalization. Keep `wasm2rom(bytes)` as the empty-registry path and preserve its rejection behavior.
- [x] Run `cargo test --locked --release -p ziskemu --features float --test wasm_imports` and require every test to pass.
- [x] Document a complete arithmetic example and calling convention/identity obligations; explicitly retain the CLI and Nitro limitations.
- [x] Run the transpiler and Wasm/float/example/import tests (covered by the combined full-crate command below). Report skipped external-target tests honestly.
- [x] Run the emulator crate's test suite; report platform/toolchain failures separately rather than broadening the implementation into unrelated fixes.
- [x] Check formatting and full diff against the pinned base; obtain a fresh review and fix material findings with failing-then-passing tests.
- [x] Confirm worktree/branch and inspect/stage only this task's files for a coherent commit. Preserve the worktree for the adapter milestone.

## Follow-up boundary

This plan implements stage 1 of the scoped project. Stage 2 starts with adapter
memory/input design and a narrow authenticated transfer, as detailed in the spec.
The standard emulator still rejects Nitro until an adapter-aware runner supplies
real implementations. A registry full of success stubs is not acceptance.

## Verification record

- Baseline: 35 Wasm, 12 floating-point and 2 example tests passed before edits.
- New tests failed first on the absent API, then 9/10 failed against an API shell
  retaining default import rejection. All 10 pass with the implementation.
- `cargo test --locked --release -p zisk-transpiler-wasm -p ziskemu --features float`:
  113 passed including one doctest; 1 existing ignored name-shortening example.
  The stock `wasm32-wasip1` target is installed for the compiled guest test.
- `cargo test --locked --release -p ziskemu --test wasm --test wasm_imports`:
  45 passed with the default feature set.
- Changed Rust files pass `rustfmt --check`; `git diff --check` is clean.
- Fresh independent review approved this bounded slice with no findings.
- No Nitro block execution, Assembly setup, proof, or whole-workspace test claim.
