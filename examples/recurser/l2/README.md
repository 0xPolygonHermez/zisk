# Recurser example: folding an L2's block-range proofs

This example simulates an **L2 settlement layer**. A minimal leaf guest proves
one contiguous block range by attesting its settlement values
([`BlocksInfoStruct`](common/src/lib.rs)) into the proof's public values. The
**recurser** then folds three such proofs into a single proof that spans the
whole range — the "batch of batches" an L2 posts on-chain — while *checking*
that the ranges are contiguous.

```
[100,200)   [200,300)   [300,400)          leaf proofs (one per range)
     \         /            |
      (A + B) ------------ (AB + C)          recurser folds, checking A.end == B.start
                              |
                       [100, 400)            one proof for the whole range
```

## Testing it locally

The recurser folds STARK proofs, so it needs the **recursive** proving key in
`~/.zisk/provingKey/` — the base setup alone is not enough. Generate it once:

```bash
cargo-zisk setup --recursive
```

(Without it, the aggregation setup step bails with *"Global info file not found.
Run `setup --recursive` first."*)

Then run the example (in-process embedded prover — no coordinator needed):

```bash
cargo run --release -p recurser-l2-host
```

The fold is bracketed by the prover's `timer_start_info!` /
`timer_stop_and_log_info!` macros, so the log stream shows when the aggregation
starts and how long it took — making a slow or failing fold easy to spot. Then
the folded proof is verified, followed by the two rejection checks:

```text
>>> AGG_L2_ABC
Folded three ranges into [100, 400).
<<< AGG_L2_ABC (NNNNms)
Verifying the final folded proof...
Final folded proof verified successfully.
Testing invalid folds...
Non-contiguous fold correctly rejected.
Foreign programVK correctly rejected by the allow-list.
```

### Optional: wrap to PLONK

Set `ZISK_L2_PLONK=1` to also wrap the final folded proof into a PLONK/SNARK and
verify it. It's off by default because it's heavy (BN128 setup) and needs the
`provingKeySnark` artifacts:

```bash
ZISK_L2_PLONK=1 cargo run --release -p recurser-l2-host
```

Notes:
- Use `--release` — proving in debug is extremely slow.
- The first run also builds the guest ELFs and runs the circom aggregation setup,
  so it takes a while; later runs reuse the cached artifacts.
- To just compile-check the host without building the guest (fast iteration):
  `SKIP_GUEST_BUILD=1 cargo check -p recurser-l2-host`.

## What the pieces are

| Path | Role |
|------|------|
| [`guest/`](guest/src/main.rs) | Minimal leaf: reads the ABI-encoded struct on stdin and commits it verbatim as publics. No computation — the point is the fold. |
| [`foreign/`](foreign/src/main.rs) | A *different* guest (different programVK), used to show the allow-list rejecting a leaf it doesn't permit. |
| [`common/`](common/src/lib.rs) | The `sol!` `BlocksInfoStruct` + the field→slot map, shared by guest and host. |
| [`guest/aggregations/l2.toml`](guest/aggregations/l2.toml) | The aggregation definition: allow-list, `n-publics-agg = 64`, and the aggregate circuit. |
| [`guest/aggregations/circuits/aggregate.circom`](guest/aggregations/circuits/aggregate.circom) | `AggregatePublics`: checks contiguity and merges the two ranges. |
| [`host/`](host/src/main.rs) | Proves the three segments, folds them, decodes the collapsed publics, and checks the two rejection cases. |

The aggregation uses a **leaf allow-list** (`programs = ["recurser_l2_guest"]`),
so the recurser only accepts proofs from *this* guest. The host proves this two
ways at the end:

- a **non-contiguous** fold (skip a segment) fails the `AggregatePublics` stitch, and
- a proof from the **`foreign` guest** (a different programVK, not on the
  allow-list) makes the circuit unsatisfiable, so the fold is rejected before its
  publics even matter.

---

## How publics fit into ZisK

A ZisK proof carries a fixed **64-slot public array** (each slot a `u32`; unused
slots are zero). A guest fills it by committing bytes (`ziskos::io::commit` /
`commit_slice`), packed little-endian, 4 bytes per slot. The host reads it back
with `proof.get_publics()` — `.public_u64()`, `.read_abi::<T>()`, or `.read::<T>()`.

This example uses ABI: the guest commits `struct.abi_encode()` and the host reads
`read_abi::<BlocksInfoStruct>()`. All fields are static ABI types (32 bytes each),
so the encoding is a flat 8×32 bytes = all 64 slots (field *i* → slots `[i*8, i*8+8)`),
hence `n-publics-agg = 64`. (Dynamic fields like `bytes`/`T[]` wouldn't pack flatly.)

On the recursion path the layout is `[flag(1) | program VK(4) | user publics(64)]`;
`AggregatePublics` only touches the 64 user publics (`ZISK_PUBLICS()` in circom).
You declare how many you populate via `n-publics-agg`; the scaffolding zero-fills
the rest of the output for you. Inputs stay full 64-wide.

### The stitch

[`aggregate.circom`](guest/aggregations/circuits/aggregate.circom) folds A (older)
into B (newer), checking each field slot-wise:

- `A.endBlock == B.startBlock` — contiguous ranges,
- `A.globalExitRoot == B.oldGlobalExitRoot`, `A.accountRoot == B.oldAccountRoot` —
  B's pre-state equals A's post-state.

The output takes older values from A, newer from B, so `(A+B)+C` collapses
`[100,200)+[200,300)+[300,400)` into `[100,400)`.
