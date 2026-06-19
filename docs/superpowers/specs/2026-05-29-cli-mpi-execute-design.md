# CLI MPI Execute — Design

**Date:** 2026-05-29
**Status:** Draft → for review
**Scope:** Restore MPI capability for `cargo zisk execute` after the recent split that made the default execute path standalone (no proving keys).

## Background

`cargo zisk execute` was recently rewritten to use `ProverClientBuilder::build_execute_only()`, which returns an `EmuExecClient` / `AsmExecClient` that runs the executor without loading proving keys. This is the right default — fast iteration, no multi-GB artifacts, suitable for CLI dev loops, `cargo test`, and embedded SDK use.

The trade-off: the standalone path uses `NoopProofRegistry` as its `Dctx + ProofRegistry`. It is single-process by design and never initialises `MpiCtx`. As a result, the new CLI cannot run under `mpirun`, which is a regression from the pre-split CLI.

## Goal

Allow `cargo zisk execute` to run under MPI when the user opts in explicitly, while keeping the standalone path as the default for the common case.

The non-goal: enabling MPI in the standalone (no-keys) path. Rank-aware planning needs cost weights, which come from setup data. Without weights, MPI execute cannot distribute work correctly. The MPI path therefore necessarily loads the proving key, matching `prove.rs`.

## Design

CLI-only change. No modifications to `executor`, `prover-backend`, or any other crate. All changes are in `cli/src/commands/execute.rs`.

### New flags

```rust
/// Run execute under MPI. Loads the proving key for rank-aware planning.
#[arg(long, requires = "proving_key")]
pub mpi: bool,

/// Path to the proving key. Required when `--mpi` is set.
#[arg(short = 'k', long)]
pub proving_key: Option<PathBuf>,
```

`--mpi` requires `--proving-key` because the proofman path needs the key to compute per-instance cost weights for rank assignment. Enforced declaratively via clap `requires`.

### Branch in `run()`

```
if !self.mpi {
    // existing standalone path — build_execute_only(), StandaloneExecutionResult,
    // plan summary + execution summary. No change.
} else {
    // full prover client path, mirroring prove.rs:249-287
    let mut prover_options = BackendProverOpts::default().verbose(self.verbose);
    prover_options = prover_options.proving_key(self.proving_key.as_ref().unwrap().clone());
    // (+ ASM options for the asm branch)

    let prover = ProverClientBuilder::new()
        .emu()  // or .asm()
        .with_prover_options(prover_options)
        .build()?;

    let guest_program = GuestProgram::from_uri(...)?;
    let setup = prover.setup(&guest_program);
    let setup = if with_hints { setup.with_hints() } else { setup };
    setup.run()?;

    let result = prover.execute(&guest_program, stdin)?;  // ExecuteOutput
    // print execution summary; skip plan summary (proofman prints its own)
}
```

### Output differences

- **Standalone path** (default): prints the plan summary block (`--- PLAN SUMMARY ---` / one-liner per air group) and the execution summary.
- **MPI path**: skips the plan summary block. `ExecuteOutput` does not carry a plan, and proofman emits its own per-rank plan via `tracing::info!`. The execution summary still works via `result.get_execution_steps()` / `get_execution_time()`.

### Rank gating

Under `mpirun -np N`, every rank runs `cargo zisk execute --mpi …`. Banner, prompts, and the execution summary must print only on rank 0. Approach: check `OMPI_COMM_WORLD_RANK` env var in the CLI before each non-rank-gated print, OR rely on proofman's existing `info!` rank gating where it already applies.

To be verified during implementation: which env var proofman expects, and whether the banner prints (`print_banner_*`) need an explicit rank-0 check or whether they already short-circuit somewhere. The verification step writes the answer into the implementation plan, not into this spec.

## Architecture notes

- `EmuExecClient` / `AsmExecClient` (the standalone path) never touch `MpiCtx::new()`. Confirmed: `build_execute_only()` skips proofman entirely. Running them under `mpirun` is a no-op (just N independent single-process runs).
- The full prover client already exposes `pub fn execute(&self, program, stdin) -> Result<ExecuteOutput>` at `prover-backend/src/prover/mod.rs:490`, backed by `proofman.execute_from_lib()` in `backend.rs:182`. This path supports MPI via the `ProofmanAdapter`.
- The pattern for building the full client, setting it up, and running it under MPI is already established in `cli/src/commands/prove.rs:249-287`. The MPI branch in `execute.rs` is essentially this pattern minus the `prove.run()` step plus a call to `prover.execute(...)`.

## Trade-offs considered

| Approach | Pros | Cons | Decision |
|---|---|---|---|
| **A. Two paths in CLI (chosen)** | Cheapest. Reuses existing APIs. Default stays fast (no keys). MPI users opt in. | Two code paths to maintain in `execute.rs`. | Picked. |
| **B. Inject MPI-aware Dctx into standalone path** | Most flexible. MPI without proving keys. | Requires lifting MPI bindings out of proofman. Bigger refactor. Standalone-MPI use case is narrow (you'd still want weights). | Deferred — revisit only if a real use case appears. |
| **C. Status quo (no MPI in execute CLI)** | Simplest. | Regression vs. pre-split CLI. | Rejected. |

## Risks

- **None structural.** The full client already supports execute under MPI; this change wires the CLI to it.
- **Rank-gating verification** is the only implementation-time unknown — covered above.

## Out of scope

- Auto-detection of MPI (e.g., reading `OMPI_COMM_WORLD_SIZE > 1` to flip the flag implicitly). Rejected as fragile: a user running `mpirun -np 4 cargo zisk execute` without intending MPI execute would unintentionally trigger a heavy proving-key load. Explicit `--mpi` is clearer.
- Capturing a plan summary on the MPI path. Not blocking; proofman already prints one. Could be added later by exposing `ProofmanAdapter`'s instance-info to the CLI in the same shape as `NoopProofRegistry::take_instance_counts()`.
