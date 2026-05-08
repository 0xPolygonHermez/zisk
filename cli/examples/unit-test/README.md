# `cargo-zisk unit-test`

Run constraint verification against hand-authored state-machine inputs, with no
ELF, no ROM, and no emulation. Useful when developing or debugging a single
state machine: you craft the inputs that would normally come from ROM
execution, and the unit-test backend feeds them straight into the SM's
witness-generation path.

## Quick start

```bash
cargo-zisk unit-test --inputs cli/examples/unit-test/example.json
```

Optional flags:

- `-k, --proving-key <PATH>` — override `~/.zisk/provingKey`.
- `-g, --gpu` — run constraint verification on GPU. Requires a CUDA-enabled
  build (the default unless `--features cpu-only`) and a GPU with enough free
  VRAM for proofman's pre-allocated trace buffers (typically a few × 10s of
  GB). Implies packed trace rows, which are wired through automatically.
- `-v, --verbose` — bump verbosity (`-v`, `-vv`).
- `-d, --debug [JSON]` — passes a debug-instances JSON to proofman (advanced).

The command exits 0 when every per-AIR constraint passes and a non-zero code
otherwise; the failing constraint is printed with its row coordinates.

## What gets verified

Unit-test mode runs proofman in `verify_constraints` mode and **skips global
constraints** — by design. Globals span multiple SMs and would never balance
when only a subset is fed inputs. Per-AIR constraints (the ones that catch
most SM bugs) are fully checked, plus the auxiliary `SpecifiedRanges` and
`VirtualTable0` tables.

Out of scope for v1:

- `Mem`, `InputData`, `RomData` — these are driven by full memory traffic and
  can't be reconstructed from individual inputs.
- Full proof generation. `--prove` is not wired yet; `verify_constraints` is
  the only mode.
- Cell-injection mode (overriding individual trace cells of a real ELF run).

## JSON shape

```json
{
  "<KeyName>": [ <input1>, <input2>, ... ],
  "<OtherKey>": [ ... ]
}
```

Each top-level key selects an AIR. Each entry in its array becomes one input
to that SM. Inputs in the same key are packed into one or more AIR instances
according to the trace size of that AIR. You can mix as many keys as you want
in a single file — they're verified independently.

Field types match the in-tree `*Input` structs verbatim (snake_case). Below is
the field list per key.

### Subset / partial inputs

Every input struct derives `Default` and is annotated with `#[serde(default)]`,
so missing fields fall back to zero / `false` / empty arrays. The two examples
below are equivalent:

```json
{ "Poseidon2": [ { "step_main": 0, "addr_main": 0, "state": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0] } ] }
```

```json
{ "Poseidon2": [ { "state": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0] } ] }
```

You can even pass `{ "Poseidon2": [ {} ] }` — every field defaults to zero.
This makes targeted negative testing easy: only set the fields you care about,
let the rest stay zero. Note: an empty array (e.g. `[ ]` for `state`) is *not*
the same as omitting the field — Serde will reject a length mismatch.

### Binary / BinaryExtension

```json
{ "op": <u8>, "a": <u64>, "b": <u64> }
```

Valid `op` values:

| AIR              | Ops accepted                                                    |
| ---------------- | --------------------------------------------------------------- |
| `Binary`         | `0x0a` add, `0x0b` sub, `0x0e` and, `0x0f` or, `0x10` xor, plus comparators (`0x04` ltu, `0x05` lt, `0x06` leu, `0x07` le, `0x08` eq) and 32-bit variants in `0x14`-`0x18`, plus `min*` / `max*` |
| `BinaryExtension` | `0x21` sll, `0x22` srl, `0x23` sra, `0x24-0x26` *_w variants, `0x27-0x29` sign-extends |

### BinaryAdd

```json
[<a: u64>, <b: u64>]
```

Two-operand array — fast path for unsigned 64-bit addition, used by the
dedicated `BinaryAddSM`.

### Arith

```json
[<op: u64>, <op_type: u64>, <a: u64>, <b: u64>]
```

`OperationData<u64>` is `[u64; 4]` = `[op, op_type, a, b]`. For Arith, set
`op_type = 2` (`ZiskOperationType::Arith`). Common ops are `0xb0` mulu,
`0xb1` muluh, `0xb3` mulsuh, `0xb4` mul, `0xb5` mulh, `0xb6` mul_w,
`0xb8` divu, `0xb9` remu, `0xba` div, `0xbb` rem, `0xbc-0xbf` *_w divides.

### Keccakf

```json
{ "step_main": <u64>, "addr_main": <u32>, "state": [<u64>; 25] }
```

A single Keccak-f permutation block. The 25 lane values are the input state.

### Sha256f

```json
{
  "step_main": <u64>,
  "addr_main": <u32>,
  "state_addr": <u32>,
  "input_addr": <u32>,
  "state": [<u64>; 4],
  "input": [<u64>; 8]
}
```

### Poseidon2

```json
{ "step_main": <u64>, "addr_main": <u32>, "state": [<u64>; 16] }
```

### Blake2

```json
{
  "addr_main": <u32>,
  "step_main": <u64>,
  "index": <u64>,
  "state_addr": <u32>,
  "input_addr": <u32>,
  "state": [<u64>; 16],
  "input": [<u64>; 16]
}
```

### ArithEq

`ArithEqInput` is an externally-tagged enum; pick exactly one variant per entry:

```json
{ "Arith256":         { "addr", "a_addr", "b_addr", "c_addr", "dh_addr", "dl_addr", "step", "a":[u64;4], "b":[u64;4], "c":[u64;4] } }
{ "Arith256Mod":      { ..., "module":[u64;4] } }
{ "Secp256k1Add":     { "addr", "p1_addr", "p2_addr", "step", "p1":[u64;8], "p2":[u64;8] } }
{ "Secp256k1Dbl":     { "addr", "step", "p1":[u64;8] } }
{ "Bn254CurveAdd":    { ... like Secp256k1Add ... } }
{ "Bn254CurveDbl":    { ... like Secp256k1Dbl ... } }
{ "Bn254ComplexAdd":  { "addr", "f1_addr", "f2_addr", "step", "f1":[u64;8], "f2":[u64;8] } }
{ "Bn254ComplexSub":  { ... same shape as ComplexAdd ... } }
{ "Bn254ComplexMul":  { ... same shape as ComplexAdd ... } }
{ "Secp256r1Add":     { ... like Secp256k1Add ... } }
{ "Secp256r1Dbl":     { ... like Secp256k1Dbl ... } }
```

### ArithEq384

```json
{ "Arith384Mod":         { ..., "a":[u64;6], "b":[u64;6], "c":[u64;6], "module":[u64;6] } }
{ "Bls12_381CurveAdd":   { "addr", "p1_addr", "p2_addr", "step", "p1":[u64;12], "p2":[u64;12] } }
{ "Bls12_381CurveDbl":   { "addr", "step", "p1":[u64;12] } }
{ "Bls12_381ComplexAdd": { "addr", "f1_addr", "f2_addr", "step", "f1":[u64;12], "f2":[u64;12] } }
{ "Bls12_381ComplexSub": { ... same shape ... } }
{ "Bls12_381ComplexMul": { ... same shape ... } }
```

### Add256

```json
{
  "step_main": <u64>,
  "addr_main": <u32>,
  "addr_a": <u32>, "addr_b": <u32>, "addr_c": <u32>,
  "cin": <u64>,
  "a": [<u64>; 4],
  "b": [<u64>; 4]
}
```

### MemAlign

```json
{
  "addr": <u32>,
  "is_write": <bool>,
  "width": <u8>,                 /* 1, 2, 4, or 8 */
  "step": <u64>,
  "value": <u64>,
  "mem_values": [<u64>; 2]
}
```

The number of trace rows consumed depends on `(is_write, addr%8 + width > 8)`:
2 rows for read-aligned, 3 for read-cross/write-aligned, 5 for write-cross.
Multiple inputs are packed into one AIR instance until the trace fills up.

### DMA — 12 keys, 4 input shapes

DMA is special: 12 distinct AIRs sit on top of only 4 underlying input shapes.
The JSON key picks the AIR; the input shape is implied by the key.

| Keys                                                                                                                                            | Input shape         |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| `Dma`, `DmaMemCpy`, `DmaInputCpy`                                                                                                                 | `DmaInput`          |
| `DmaPrePost`, `DmaPrePostMemCpy`, `DmaPrePostInputCpy`                                                                                            | `DmaPrePostInput`   |
| `Dma64Aligned`, `Dma64AlignedMemCpy`, `Dma64AlignedInputCpy`, `Dma64AlignedMemSet`, `Dma64AlignedMem`                                              | `Dma64AlignedInput` |
| `DmaUnaligned`                                                                                                                                    | `DmaUnalignedInput` |

Input shapes:

```json
DmaInput: { "src": <u32>, "dst": <u32>, "op": <u8>, "encoded": <u64>, "count_bus": <u32>, "step": <u64> }

DmaPrePostInput: {
  "src": <u32>, "dst": <u32>, "step": <u64>, "encoded": <u64>,
  "src_values": [<u64>; 2], "dst_pre_value": <u64>, "op": <u8>
}

Dma64AlignedInput: {
  "src": <u32>, "dst": <u32>, "is_last_instance_input": <bool>, "op": <u8>,
  "trace_offset": <u32>, "skip_rows": <u32>, "rows": <u32>,
  "step": <u64>, "encoded": <u64>, "src_values": [<u64>...]
}

DmaUnalignedInput: {
  "src": <u32>, "dst": <u32>, "is_last_instance_input": <bool>, "is_mem_eq": <bool>,
  "trace_offset": <u32>, "skip": <u32>, "count": <u32>,
  "step": <u64>, "encoded": <u64>, "src_values": [<u64>...]
}
```

Valid `op` values for DMA: `0xd0` memcpy, `0xd1` memcmp, `0xd2` inputcpy,
`0xd6` xmemcpy, `0xd7` xmemcmp, `0xd9` xmemset.

For unit-test runs the `Dma64Aligned*` and `DmaUnaligned` paths are invoked
with `segment_id = 0` and `is_last_segment = true` — i.e. a single segment.
Multi-segment scenarios aren't exposed via the CLI yet.

## Worked example: `example.json`

[`example.json`](example.json) in this directory exercises every supported
SM in a single run. Field values are conservative defaults — they pass per-AIR
constraints but don't represent meaningful computations. Substitute your own
values to test specific SM behaviors.

Run it:

```bash
cargo-zisk unit-test --inputs cli/examples/unit-test/example.json -v
```

Expected output (abridged):

```
✓ All constraints for Instance #0 of Binary were verified
✓ All constraints for Instance #0 of BinaryAdd were verified
✓ All constraints for Instance #0 of BinaryExtension were verified
✓ All constraints for Instance #0 of Arith were verified
✓ All constraints for Instance #0 of Keccakf were verified
✓ All constraints for Instance #0 of Sha256f were verified
✓ All constraints for Instance #0 of Poseidon2 were verified
✓ All constraints for Instance #0 of Blake2br were verified
✓ All constraints for Instance #0 of ArithEq were verified
✓ All constraints for Instance #0 of ArithEq384 were verified
✓ All constraints for Instance #0 of Add256 were verified
✓ All constraints for Instance #0 of MemAlign were verified
✓ All constraints for Instance #0 of Dma were verified
✓ All constraints for Instance #0 of SpecifiedRanges were verified
✓ All constraints for Instance #0 of VirtualTable0 were verified
```

## Extending the example

A few patterns worth knowing:

- **Multiple inputs per SM.** Append more entries to the array — the planner
  packs them into chunks sized by `Trace::NUM_ROWS` for that AIR.
- **Multiple instances of the same AIR.** Submit enough inputs that they
  exceed one chunk; the planner registers additional AIR instances
  automatically. Useful for stress-testing the SM's chunk-boundary handling.
- **Negative testing.** Submit a deliberately bad input (e.g. corrupt `a`,
  `b`, or `state`); the failing per-AIR constraint will be reported with
  row coordinates so you can pinpoint the offending column.

## Programmatic API (Rust tests)

For Rust test code, `UnitTestProver` exposes a typed builder — no JSON, no
temp files, no string column names. Inputs are constructed as the same
typed structs the SM uses internally; trace-row hooks fire post-witness
and operate on the typed row struct.

### Where tests live

Tests for a state machine live **in that state machine's own crate**, in
its `tests/` directory. One file per crate is the canonical pattern:

```
state-machines/binary/tests/unit_test.rs
precompiles/arith_eq/tests/unit_test.rs
precompiles/keccakf/tests/unit_test.rs
…
```

The SM author owns the tests; an external developer adding a new SM in
their own crate uses the exact same pattern with no central registry to
update. Each test crate just needs `zisk-prover-backend` as a dev-dep:

```toml
# in <sm-crate>/Cargo.toml
[dev-dependencies]
zisk-prover-backend = { workspace = true }
```

### Anatomy of a test file

The shared `UnitTestProver` singleton (one per process — `MPI_Init` is
one-shot) and the "skip if no proving key" guard are encapsulated in
`zisk_prover_backend::testing::with_prover`. A typical test file is:

```rust
use zisk_prover_backend::{
    inputs::{BinaryInput, KeccakfInput},
    rows::BinaryTraceRow,
    testing::with_prover,
    BinarySm, KeccakfSm,
};

#[test]
#[ignore]
fn binary_or_passes() {
    with_prover(|prover| {
        prover.verify_input()
            .input::<BinarySm>(BinaryInput { op: 15, a: 5, b: 3 })
            .input::<KeccakfSm>(KeccakfInput::default())
            .run()
            .expect("constraints should hold");
    });
}

#[test]
#[ignore]
fn binary_hook_injection_is_caught() {
    with_prover(|prover| {
        // Inputs + hook — the hook fires for every row of the matching
        // AIR instance and operates on the typed `BinaryTraceRow`.
        let result = prover.verify_input()
            .input::<BinarySm>(BinaryInput { op: 15, a: 5, b: 3 })
            .hook::<BinarySm>(|input_idx, _clock, row: &mut BinaryTraceRow<_>| {
                if input_idx == 0 {
                    row.set_use_first_byte(true);  // any post-hoc mutation
                }
            })
            .run();
        assert!(result.is_err(), "constraint violation should propagate");
    });
}
```

### Running the suite

```bash
# A single crate's tests:
cargo test -p precomp-arith-eq --test unit_test --release \
    -- --ignored --test-threads=1

# Every crate's tests at once:
cargo test --workspace --release -- --ignored --test-threads=1
```

`--ignored` is required — the tests are marked `#[ignore]` because they
need `~/.zisk/provingKey` and spin up `ProofMan` (~3s each).
`--test-threads=1` is **mandatory** — `proofman-starks-lib-c` is not
re-entrant within a single process, and `with_prover` serialises access
through a `Mutex` to enforce this.

Notable properties:

- **No JSON in test code.** Inputs are typed; the macro-generated
  `set_<col>` / `get_<col>` row methods are visible to the IDE and
  type-checked at compile time.
- **One terminal `.run()`** regardless of whether hooks were registered.
- **Hooks are independent of inputs.** Inputs flow through the registered
  SM's `compute_witness`; hooks fire post-witness and mutate the trace
  buffer in place. Either is optional.
- **Same wire format internally.** The session serialises typed inputs
  through the SM's `Serialize` impl — exactly the JSON shape the CLI
  accepts. The CLI and the builder share one source of truth (the
  `UnitTestSm` trait registry).

### Adding a new SM to the framework

Each SM owns its own `UnitTestSm` impl, **inside its own crate**. The
trait lives in `zisk-common` (a leaf crate everyone depends on), so SM
crates implement it without depending on `executor`. The executor only
imports the markers and lists them in a registry.

Three small things to do per new SM:

1. **In your SM crate's `lib.rs`**, after the production re-exports,
   invoke the `unit_test_sm!` macro. The shorthand form (most SMs) drops
   the `compute` closure entirely — the macro auto-generates the
   packed/non-packed branch:

   ```rust
   use zisk_common::unit_test_sm;
   use zisk_pil::{NewTrace, NewTraceRow, NewTraceRowPacked, NEW_AIR_IDS};

   unit_test_sm! {
       NewSm => {
           name: "New",
           air: NEW_AIR_IDS[0],
           input: NewInput,
           manager: NewSM<F>,            // the inner SM directly
           row: NewTraceRow<F>,
           row_packed: NewTraceRowPacked<F>,
           chunk_size: |_| NewTrace::<usize>::NUM_ROWS,
       }
   }
   ```

   The full form (with explicit `compute: |sm, sctx, inputs, buf, packed| {...}`)
   is for SMs whose witness call has a non-standard shape — extra `sctx`
   prefix, segment args, custom `used_rows`, etc. See
   [`state-machines/mem/src/lib.rs`](../../../state-machines/mem/src/lib.rs)
   for examples.

2. **In the executor**, append `&NewSm` to `REGISTRY` in
   [`executor/src/unit_test_targets/mod.rs`](../../../executor/src/unit_test_targets/mod.rs).

3. **In the same file**, add one line to `build_manager_registry` so
   the executor can extract the inner SM Arc from the bundle:

   ```rust
   if let Some(m) = bundle.new_sm() {
       map.insert(NEW_AIR_IDS[0], erase(m.inner_sm()));
   }
   ```

   (Step 3 is the only place the executor needs to know about the
   bundle layout. The trait impls themselves never reference
   `StaticSMBundle`.)

That's it. The new SM is immediately usable from the CLI (via its
`name()`) and from Rust tests
(`prover.verify_input().input::<NewSm>(input)`). No edits to
`ZiskExecutorTest`, the dispatcher, the trait file, or any test code.

### Worked test

[`precompiles/arith_eq/tests/unit_test.rs`](../../../precompiles/arith_eq/tests/unit_test.rs)
is the canonical example: an honest baseline + a hook that injects a
single bad chunk and asserts the per-AIR identity catches it.

```bash
cargo test -p precomp-arith-eq --test unit_test --release -- \
    --ignored --test-threads=1
```

## Architecture (for reference)

- CLI: [`cli/src/commands/unit_test.rs`](../../src/commands/unit_test.rs)
- Backend: [`prover-backend/src/prover/unit_test.rs`](../../../prover-backend/src/prover/unit_test.rs) — builds a `ProofMan` whose witness component is `ZiskExecutorTest`.
- Executor: [`executor/src/executor_test.rs`](../../../executor/src/executor_test.rs) — parses JSON, plans AIR instances, dispatches `compute_witness` per AIR id.
- SM bundle is shared with the production path via [`build_sm_bundle`](../../../executor/src/utils.rs); adding a new SM there automatically makes it available to unit-test mode (you still need to add a JSON key + dispatch arm to `executor_test.rs`).
