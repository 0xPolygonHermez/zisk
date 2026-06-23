# ZisK per-AIR unit tests

Run constraint verification against hand-authored state-machine inputs, with no
ELF, no ROM, and no emulation. Useful when developing or debugging a single
state machine: you craft the inputs that would normally come from ROM
execution, and the unit-test backend feeds them straight into the SM's
witness-generation path, then runs proofman in `verify_constraints` mode over
the result.

> **Scope note.** Today the framework is driven entirely from **Rust test
> code** via the `zisk_prover_backend` test API (`with_prover` + the
> `prover.input()/.hook()/.trace()….run()` builder). There is **no
> `cargo-zisk unit-test` CLI command and no JSON input path yet** — the
> `example.json` in this directory documents the on-the-wire input *shape* for
> each SM (handy when authoring typed inputs), but nothing consumes it
> directly. The only new CLI surface is `cargo-zisk-dev get-constraints` (see
> below).

## What gets verified

Unit-test mode runs proofman in `verify_constraints` mode and **skips global
constraints** by default — by design. Globals span multiple SMs and would never
balance when only a subset is fed inputs. Per-AIR constraints (the ones that
catch most SM bugs) are fully checked, plus the auxiliary `SpecifiedRanges` and
`VirtualTable0` tables. An explicit `.global_constraints([..])` list opts back
in for just those ids (e.g. a memory-continuity constraint when testing a
segment chain).

Out of scope today:

- `Mem`, `InputData`, `RomData` — these are driven by full memory traffic and
  their witness path needs a planner-grade segment checkpoint the framework
  does not yet build. They are registered but return a clear "not yet
  supported" error; only `MemAlign` is testable. (`example.json` therefore does
  not include them.)
- Full proof generation. `verify_constraints` is the only mode.

## Programmatic API (Rust tests)

`UnitTestProver` exposes a typed builder — no JSON, no temp files, no string
column names. Inputs are constructed as the same typed structs the SM uses
internally; trace-row hooks fire post-witness and operate on the typed row
struct.

### Where tests live

All unit tests live in the **`unit-tests/`** workspace crate, one file per SM
family under [`unit-tests/tests/`](../../../unit-tests/tests/):

```
unit-tests/tests/binary.rs
unit-tests/tests/arith_eq.rs
unit-tests/tests/hashes.rs
unit-tests/tests/dma.rs
…
```

The crate has no library or binary of its own — only the `tests/` directory
matters. It is a workspace member, so `cargo test --workspace` runs these
tests. (An SM author may equally drop a `tests/unit_test.rs` in their own crate
and add `zisk-prover-backend` as a dev-dependency; the API is identical. The
centralized `unit-tests/` crate is just the convention this repo follows.)

### Anatomy of a test file

The shared `UnitTestProver` singleton (one per process — `MPI_Init` is
one-shot) and the "skip if no proving key" guard are encapsulated in
`zisk_prover_backend::testing::with_prover`. A typical test file is:

```rust
use zisk_prover_backend::{inputs::BinaryInput, testing::with_prover, BinarySm};

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn binary_or_passes() {
    with_prover(|prover| {
        let result = prover
            .input::<BinarySm>(BinaryInput { op: 15, a: 5, b: 3 })
            .run()
            .expect("verification run failed");
        assert!(result.valid, "honest input should satisfy all constraints");
    });
}
```

`run()` returns a `ConstraintsVerificationResult`: constraint violations are
**data** (`result.valid == false`, per-instance failures in `result.instances`
with `failed_constraints`), not an `Err`. `Err` means the run itself broke
(setup, planning, witness computation).

### The three entry points

- **`.input::<S>(input)` / `.inputs::<S, _>(iter)`** — feed typed inputs
  through the SM's normal `compute_witness`. Repeated calls accumulate; mix as
  many SMs as you like in one run.
- **`.hook::<S>(|input_idx, clock, row| …)`** — `compute_witness` builds the
  trace, then the hook fires post-witness for every row and mutates the typed
  `S::Row` in place (call its macro-generated `set_<col>` / `get_<col>`
  methods). Use it to corrupt a known-good trace and assert the constraint
  catches it. Repeated hooks for the same SM stack in registration order.
- **`.trace::<S>(|trace, std| …)` / `.traces::<S>(n, |i, trace, std| …)`** —
  author the trace directly, bypassing `compute_witness`. No `.input()` needed;
  the executor plans one (or `n`) instance(s) from the AIR's fixed shape. The
  closure gets the freshly-allocated zeroed typed trace and the shared `Std`
  handle (to emit the range-check / lookup multiplicities a fully *valid* trace
  needs). `.traces` additionally passes the 0-based instance index.

See [`unit-tests/tests/arith_eq.rs`](../../../unit-tests/tests/arith_eq.rs) for
all three exercised together: an honest baseline, a hook that flips one column
and asserts the per-AIR identity catches it, and a raw authored trace that
breaks the `sel_op` latch.

### Running the suite

```bash
# A single crate's tests:
cargo test -p unit-tests --test arith_eq --release -- --ignored --test-threads=1

# Every test at once:
cargo test --workspace --release -- --ignored --test-threads=1
```

`--ignored` is required — the tests are marked `#[ignore]` because they need
`~/.zisk/provingKey` and spin up `ProofMan` (~3s each); `with_prover` skips
silently when no proving key is present.

`--test-threads=1` is **mandatory** — `proofman-starks-lib-c` is not re-entrant
within a single process, and `with_prover` serialises access through a `Mutex`
to enforce this.

## `cargo-zisk-dev get-constraints`

Lists every constraint (index + PIL source line) of every AIR in the proving
key, plus the global constraints. The indices are the `constraint_id`s reported
by constraint verification — the lookup table for writing tests that pin a
specific constraint.

```bash
# All AIRs + global constraints:
cargo-zisk-dev get-constraints

# Only the named AIRs (case-insensitive; repeat or comma-separate):
cargo-zisk-dev get-constraints --air ArithEq --air Binary
cargo-zisk-dev get-constraints --air ArithEq,Binary

# Override the proving-key path (defaults to ~/.zisk/provingKey):
cargo-zisk-dev get-constraints -k /path/to/provingKey
```

## Adding a new SM to the framework

Each SM owns its own `UnitTestSm` impl, **inside its own crate**. The trait
lives in `zisk-common` (a leaf crate everyone depends on), so SM crates
implement it without depending on `executor`.

1. **In your SM crate's `lib.rs`**, after the production re-exports, invoke the
   `unit_test_sm!` macro. The shorthand form (most SMs) drops the `compute`
   closure — the macro auto-generates the packed/non-packed branch:

   ```rust
   use zisk_common::unit_test_sm;
   use zisk_pil::{NewTrace, NewTraceRow, NewTraceRowPacked, NEW_AIR_IDS};

   unit_test_sm! {
       NewSm => {
           name: "New",
           air: NEW_AIR_IDS[0],
           input: NewInput,
           manager: NewSM<F>,            // the inner witness-producing SM
           row: NewTraceRow<F>,
           row_packed: NewTraceRowPacked<F>,
           trace: NewTrace,
           chunk_size: |_| NewTrace::<usize>::NUM_ROWS,
       }
   }
   ```

   The full form (with an explicit
   `compute: |sm, sctx, inputs, buf, packed| { … }` closure) is for SMs whose
   witness call has a non-standard shape — extra segment args, custom
   `used_rows`, etc. See
   [`state-machines/mem/src/lib.rs`](../../../state-machines/mem/src/lib.rs) and
   [`precompiles/dma/src/lib.rs`](../../../precompiles/dma/src/lib.rs) for
   examples. The `trace:` line is required in both forms — it also emits the raw
   trace-authoring override impl, so `.trace()` works for the SM.

2. **In the executor**, add `NewSm` to the `registry![ … ]` list in
   [`executor/src/unit_test_targets/mod.rs`](../../../executor/src/unit_test_targets/mod.rs).
   The macro expands it into both the `REGISTRY` and `OVERRIDE_REGISTRY`.

3. **In the same file**, add one match arm to `build_manager_registry` so the
   executor can extract the inner SM `Arc` from the bundle:

   ```rust
   StateMachines::Precompile(Precompiles::New(p)) => {
       map.insert(NEW_AIR_IDS[0], erase(p.new_sm()));
   }
   ```

That's it — the new SM is usable from Rust tests via
`prover.input::<NewSm>(input)`. No edits to `ZiskExecutorTest`, the dispatcher,
or the trait file.

## Architecture (for reference)

- Backend: [`prover-backend/src/prover/unit_test.rs`](../../../prover-backend/src/prover/unit_test.rs) — builds a `ProofMan` whose witness component is `ZiskExecutorTest`; the `VerifyInput` builder lives in [`verify_input.rs`](../../../prover-backend/src/prover/verify_input.rs).
- Executor: [`executor/src/executor_test.rs`](../../../executor/src/executor_test.rs) — plans AIR instances and dispatches `compute_witness` / trace overrides per AIR id.
- Trait + registry: [`common/src/unit_test_sm.rs`](../../../common/src/unit_test_sm.rs) (the `UnitTestSm` trait and `unit_test_sm!` macro) and [`executor/src/unit_test_targets/mod.rs`](../../../executor/src/unit_test_targets/mod.rs) (the registry).
