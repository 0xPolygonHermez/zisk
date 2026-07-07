# Recurser example: private-vector hash folding

Each leaf knows **12 secret `u64` values** and proves only their Poseidon1 hash —
the preimage stays private. The recurser folds proofs by **element-wise summing**
the secret vectors and exposing `Poseidon1(sum)` as the folded public.

```
secret va ──prove──▶ H(va)          secret vb ──prove──▶ H(vb)
                 \                          /
                  fold(free: va, vb) ─▶ H(va+vb)        secret vc ─▶ H(vc)
                              \                              /
                               fold(free: va+vb, vc) ─▶ H(va+vb+vc)
```

Run it (needs the recursive proving key — see the [l2 example](../l2/README.md#testing-it-locally)):

```bash
cargo run --release -p recurser-hash-host
```

It proves three leaves, folds them, and at each step checks the exposed digest
equals the host's own `Poseidon1(sum)` — then verifies the final folded proof.

## What this example teaches

It exercises the two recurser features the [l2 example](../l2/README.md) doesn't:

1. **`NormalizePublics` as a real transform** — Poseidon works over Goldilocks
   field elements (`u64`), but ZisK publics are `u32` slots. The leaf commits its
   4-element digest as 8 `u32` limbs; `NormalizePublics` reassembles them into
   4 field elements so the fold works over the native representation
   (`n-publics-agg = 4`). See [circuits/normalize.circom](guest/aggregations/circuits/normalize.circom).
2. **Free inputs** — the 12-element secret vector is carried into each fold as a
   *free input* (`free-inputs = 12`), off the public output. See
   [circuits/aggregate.circom](guest/aggregations/circuits/aggregate.circom),
   which sums the two vectors and hashes them **in-circuit** with the
   `Poseidon` custom template — the same Poseidon1 the guest computes (both go
   through `fields::poseidon1_hash`, which uses the `syscall_poseidon1`
   precompile on the guest), so the digests agree.

## How the digest fits into ZisK publics

| Layer | Representation |
|-------|----------------|
| Guest commit | 4 field elements → each as 2 little-endian `u32` limbs → **8 `u32` slots** |
| `NormalizePublics` | reassembles `slot[2k] + slot[2k+1]·2³²` → **4 field elements** in slots `[0,4)` |
| `AggregatePublics` | folds over those 4 field elements; the recursion path carries full `u64` per slot, so a >32-bit element round-trips |

### Reading it back on the host (a gotcha)

The host reads leaf and folded proofs *differently*:

- A **leaf** commits `u32` limbs → `public_u64()` + pairwise reassembly (`leaf_digest`).
- A **folded** proof's digest is 4 full field elements in slots `[0,4)`, which
  exceed 32 bits — `public_u64()` truncates them. Read `publics_full()` instead
  (`[program_vk(4) | user(64)]`) and take user slots `[0,4)` (`folded_digest`).

## Binding the free inputs (soundness)

A free input is host-supplied, so `AggregatePublics` binds each side's vector to
its proof's committed digest:

```
Poseidon1(free_inputs_a) === a.digest      // and same for b
```

This holds at every level (leaf: `H(v)`; aggregated: `H(sum)`), so the prover
can't fold a vector that doesn't match the proof. It's the example's `===` stitch
— the analogue of l2's contiguity check. The host's final negative test feeds a
tampered vector and confirms the fold is rejected.

## The pieces

| Path | Role |
|------|------|
| [`guest/`](guest/src/main.rs) | Reads 12 private `u64`s in one array, hashes via the shared `hash12`, commits the 4-element digest. |
| [`common/`](common/src/lib.rs) | Layout constants + `hash12` / `add_vecs`, shared by guest and host (`hash12` uses the precompile on the guest, native Rust on the host). |
| [`guest/aggregations/hash.toml`](guest/aggregations/hash.toml) | allow-list, `n-publics-agg = 4`, `free-inputs = 12`, normalize + aggregate circuits. |
| [`guest/aggregations/circuits/normalize.circom`](guest/aggregations/circuits/normalize.circom) | u32-limbs → field-elements reassembly. |
| [`guest/aggregations/circuits/aggregate.circom`](guest/aggregations/circuits/aggregate.circom) | sum the free vectors, `Poseidon(4)` → folded digest. |
| [`host/`](host/src/main.rs) | Proves the leaves, folds, checks each digest against native `Poseidon1(sum)`. |
