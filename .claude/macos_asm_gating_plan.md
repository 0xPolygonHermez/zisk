# Handoff: compile-time gate the ASM backend off macOS (no `Asm*` on non-Linux-x86_64)

> Pick this up in a Claude Code session **on a real Mac (`aarch64-apple-darwin`)**, where
> `cargo check`/`cargo build` actually compile the macOS code path. On the Linux dev host this
> work was started, the macOS arms are *invisible* to the compiler (see "Verification reality"),
> which is why it's being continued here.

## Goal

On macOS, ZisK **cannot** run the ASM backend (the CLI/worker force the Rust emulator via
`should_use_emulator(asm, cfg!(target_os = "macos"))` — `--asm` is ignored on macOS, confirmed by
the test at `cli/src/commands/dev/execute.rs:276`). Today the ASM types still *exist* on macOS as
panic/`unreachable!` **stubs** purely so cross-platform code typechecks. The objective is the
conceptually-honest design: **gate the entire ASM backend behind
`#[cfg(all(target_os = "linux", target_arch = "x86_64"))]`** so that no `Asm*` type or enum variant
exists on macOS at all. Runtime panics become compile-time absence; both stub files get deleted.

Decision already made by the repo owner: **proceed with full compile-time gating** (not the leaf-stub,
not partial).

## Starting state (branch `pre-develop-1.0.0-beta`, base commit `8f8cfdd2f`)

A prior step already **consolidated** the asm-runner stubs (this is the interim resting point; the
full refactor below will ultimately delete the consolidated file too):

- Deleted `emulator-asm/asm-runner/src/{asm_mo_runner_stub,asm_mt_runner_stub,asm_rh_runner_stub,hints_shmem_stub,inputs_shmem_stub}.rs`.
- Added `emulator-asm/asm-runner/src/platform_stub.rs` — one module holding the 4 stub types still
  named cross-platform: `AsmRunnerMO`, `AsmRunnerRH`, `HintsShmem`, `InputsShmemWriter`.
- Rewired `emulator-asm/asm-runner/src/lib.rs`: runner/shmem modules are now single-gated
  `#[cfg(linux-x86_64)]` (no `*_stub` counterpart); one `#[cfg(not(...))] mod platform_stub; pub use platform_stub::*;`.

Linux `cargo check --workspace --all-targets` is green at this point.

**⚠️ Also staged but UNRELATED — do not fold into this work:**
`executor/src/error.rs` (removes `BundleComponentMissing`/`BundleComponentDuplicate` variants) and
`test-artifacts/programs/Cargo.lock`. Leave them out of any ASM-gating commit.

## Verification reality (why this moved to a Mac)

- On **Linux x86_64**, `cargo check` only ever compiles the `#[cfg(linux-x86_64)]` arms. The
  `#[cfg(not(...))]` (macOS) arms are never built, so cfg mistakes there are **undetectable** on Linux.
- A Linux→darwin **cross-compile is impractical**: the dep graph (`ring`, `aws-lc-sys`, `blst`,
  `secp256k1-sys`, `blake3`, `libffi-sys`) won't cross-build without a real macOS toolchain. (`zig cc`
  gets Mach-O linking working but the C/asm libs each fight back — abandoned.)
- **On this Mac you have the advantage:** `cargo check`/`cargo build` natively compile the macOS
  path. Use that after every step. The repo also has a `test-macos` CI job (`.github/workflows/pr.yml`,
  `macos-14`, runs `tools/test-env/build_zisk.sh`) as the final authority.

## Architecture map — the seam

The whole ASM backend funnels through **`ExecutionPhase`** in `executor/src/execution.rs`:

```rust
pub struct ExecutionPhase {
    emulator_rust: EmulatorRust,            // always present
    emulator_asm: Option<EmulatorAsm>,      // built only via with_asm_emulator.then(|| EmulatorAsm::new(..))
    is_asm_execution: AtomicBool,           // runtime selector
}
```

`with_asm_emulator` is **always false on macOS**, so `EmulatorAsm` is **never constructed** there —
it exists purely as a type placeholder. That's the thing to delete.

`EmulatorAsm` itself is already split: `executor/src/execution/asm.rs` selects
`mod emulator` (real, linux) vs `mod stub` (`executor/src/execution/asm/stub.rs`, the panic stub).
The panic stub `EmulatorAsm` has these methods (all to be removed with the type): `new`,
`get_asm_execution_info`, `set_asm_resources`, `submit_hint_direct`, `append_raw_input`,
`set_hints_stream_src`, `set_inputs_stream_src`, `get_hints_processor`, `set_active_services`,
`reset`, `signal_cancellation`, `execute`.

## Type-by-type spread — every cross-platform naming site to gate

| Type | Cross-platform sites (must stop naming it on macOS) |
|------|------------------------------------------------------|
| `EmulatorAsm` | `executor/src/execution.rs` (field + methods), `executor/src/executor.rs:18,206`, `prover-backend/src/prover/backend.rs:9,50` |
| `AsmResources` | `executor/src/execution/asm/resources.rs` (def), `executor/src/executor.rs`, `executor/src/execution.rs`, `executor/src/error.rs` (just the `AsmResourcesNotInitialized` string variant — harmless, can stay), `prover-backend/src/prover/asm.rs`, `asm_exec.rs`, `backend.rs` |
| `HintsShmem` | `resources.rs`; `executor/src/execution/asm/stub.rs`; **prover-backend trait+impls**: `mod.rs:430` (trait method sig `get_hints_processor`), `mod.rs:690`, `emu.rs:293`, `asm.rs:643`, `backend.rs:7` |
| `InputsShmemWriter` | `resources.rs` only |
| `AsmRunnerMO` | `executor/src/execution/output.rs` (the `BackendArtifacts::Asm` variant + `await_mem_plans`) |
| `AsmRunnerRH` | `executor/src/execution/output.rs`; **`state-machines/rom`**: `rom.rs` (`set_rh_data`, `rh_data: Mutex<Option<AsmRunnerRH>>`), `rom_instance.rs`; `executor/src/witness.rs`, `sm/mod.rs`, `witness/collector.rs`, `witness/handlers/rom_rust.rs` |

### Two notable complications

1. **`HintsShmem` is in a trait** — `prover-backend/src/prover/mod.rs:430`:
   `fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>>;`. Gating this means
   the trait method must be `#[cfg(linux-x86_64)]` (and every impl), or the trait restructured. This
   is the deepest coupling — handle prover-backend carefully.

2. **`AsmRunnerRH` foreign-memory `Drop`** — on Linux, `AsmRunnerRH::drop` `mem::forget`s its
   `AsmRHData.inst_count` because it's a `Vec::from_raw_parts` view over shared memory (see
   `emulator-asm/asm-runner/src/asm_rh_runner.rs:35-45`). **Good news for this refactor:** full gating
   keeps `AsmRunnerRH` Linux-only, so the `Drop` stays exactly where it belongs — *no memory-safety
   retyping is needed* (unlike the rejected "retype sm-rom to `AsmRHData`" alternative). On macOS the
   ROM SM's `rh_data`/`set_rh_data` simply don't exist; the Rust path computes the histogram via
   `witness/handlers/rom_rust.rs` and `await_rom_histogram()` returns `None`.

## Execution plan (incremental, bottom-of-stack last)

Order so each step leaves **Linux green** (`cargo check --workspace`) and, on this Mac, **macOS green**
(`cargo check --workspace --target aarch64-apple-darwin` or native `cargo check`). Verify *both* after
each step. Gate expression to use everywhere: `#[cfg(all(target_os = "linux", target_arch = "x86_64"))]`
(and `#[cfg(not(all(...)))]` for any macOS-only simplified arm).

**Step 1 — `executor` core (`execution.rs`, `output.rs`).**
- `ExecutionPhase`: gate the `emulator_asm` field; in `new()` drop the `.then(|| EmulatorAsm::new)`
  on macOS; gate `asm_emulator()`, `set_asm_resources()`, `get_asm_execution_info()`'s asm arm,
  `reset()`'s asm arm, and `run()`'s asm arm. On macOS `run()` collapses to
  `self.emulator_rust.execute::<F>(zisk_rom, stdin)` and the asm methods either disappear or return the
  Rust-path default (`Ok(None)`/`Ok(())`). Keep `is_asm_execution`/`clear_asm_resources` if simplest,
  or gate — your call.
- `output.rs`: make `BackendArtifacts::Asm { mo, rh }` `#[cfg(linux-x86_64)]`. On macOS the enum is
  effectively just `Rust`. In `await_mem_plans`/`await_rom_histogram`, gate the `Self::Asm { .. }`
  match arms (the `Self::Rust` arm alone is exhaustive once `Asm` is cfg'd out). This removes
  `AsmRunnerMO`/`AsmRunnerRH` naming here.

**Step 2 — `executor` ASM resources + emulator (`execution/asm/*`).**
- `resources.rs` (`AsmResources`/`AsmSharedResources`) is entirely ASM machinery → gate the whole
  module to linux (it already has internal cfg for the readers). This removes `HintsShmem` /
  `InputsShmemWriter` naming from executor.
- `executor/src/execution/asm.rs`: `mod stub;` / `pub use stub::*;` becomes unnecessary →
  **delete `executor/src/execution/asm/stub.rs`** and gate the `emulator`/`resources` re-exports to linux.
- `executor/src/executor.rs`: gate `asm_emulator()` (line ~206), the `set_asm_resources` plumbing,
  and the `AsmResources, EmulatorAsm` import (line 18). `with_asm_emulator` param can stay (a bool),
  but on macOS it must be inert.

**Step 3 — `state-machines/rom`.** Gate `RomSM`'s `rh_data` field and `set_rh_data` (rom.rs),
and the `AsmRunnerRH` usage in `rom_instance.rs`, to linux. macOS ROM path uses the Rust histogram only.
This is the one place platform cfg enters a domain SM crate — keep it minimal and well-commented.

**Step 4 — `executor` witness pipeline.** Gate the `AsmRunnerRH` carriers in `witness.rs`,
`sm/mod.rs`, `witness/collector.rs`, `witness/handlers/rom_rust.rs` (the `set_rh_data` plumbing).

**Step 5 — `prover-backend`.** `asm.rs` + `asm_exec.rs` are ASM-specific files → gate to linux
(clean, already file-isolated). In `backend.rs`/`mod.rs`/`emu.rs`, gate the `asm_emulator()`,
`set_asm_resources`, and especially the `HintsProcessor<HintsShmem>` trait method (`mod.rs:430`) +
its impls. This is the trickiest crate — the trait coupling may push you to gate the whole
`get_hints_processor` trait method on linux.

**Step 6 — delete the stubs & finish `asm-runner`.**
- Delete `emulator-asm/asm-runner/src/platform_stub.rs` and its `#[cfg(not(...))] mod/pub use` in
  `lib.rs`. With all callers gated, nothing names `AsmRunnerMO/RH`, `HintsShmem`, `InputsShmemWriter`
  on macOS anymore, so the stub is dead.
- The asm-runner crate then has no macOS surface at all (like `multi_shmem` already).

**Step 7 — verify.** `cargo check --workspace` (Linux) AND native macOS build here; run
`tools/test-env/build_zisk.sh`; let `test-macos` CI confirm. Then a scoped commit of only the
ASM-gating files (exclude the unrelated `error.rs`/`Cargo.lock`).

## Gotchas checklist
- macOS unused-import warnings after gating — the asm-runner crate already has
  `#![cfg_attr(not(linux-x86_64), allow(dead_code, unused_imports))]`; executor/prover-backend/sm-rom
  may need targeted `#[cfg]` on imports instead of crate-wide allows.
- `BackendArtifacts` becoming a single-variant enum on macOS: ensure no `match` arm references the
  cfg'd-out `Asm` variant without its own `#[cfg]`.
- Tests in `output.rs` that build `AsmRunnerMO/RH` (`#[cfg(test)] mod tests`) must be gated to linux too.
- `error.rs::AsmResourcesNotInitialized`/`AsmNotAvailable` are just `&str`/`String` variants — they can
  stay cross-platform; they don't name ASM types.

## Quick reference: prove macOS path locally on this Mac
```
cargo check -p executor -p sm-rom -p prover-backend -p asm-runner   # native macOS build
cargo check --workspace
cd tools/test-env && ./build_zisk.sh
```
Compare against Linux (`cargo check --workspace` on the Linux host) to ensure neither path regressed.
