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

### Running the same steps from the CLI

`cargo run` does this in-process, but the recurser also has a `cargo-zisk` CLI
path that reads the *same* `hash.toml` — `resolve_recurser` derives the identical
`recurser_id`, so a setup done either way is found by the other.

The three leaves use these secret vectors ([`common/src/lib.rs`](common/src/lib.rs)),
and the folds carry their running sums as free inputs (element-wise; the values
stay well under the Goldilocks modulus, so no field reduction happens):

| Vector | value (`[u64; 12]`) |
|--------|----------------------|
| `va` | `1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12` |
| `vb` | `100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111` |
| `vc` | `1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008, 1009, 1010, 1011` |
| `va + vb` | `101, 103, 105, 107, 109, 111, 113, 115, 117, 119, 121, 123` |

The leaf guest reads its `[u64; 12]` with `ziskos::io::read` (bincode), so a
hand-typed `inline://` input wouldn't match the encoding. The `gen-inputs`
helper binary writes the exact input bytes to `a.bin` / `b.bin` / `c.bin`.

Run from this example dir (paths are relative to the toml), using the
`cargo-zisk` built from this repo:

```bash
export CARGO_ZISK="$(git rev-parse --show-toplevel)/target/release/cargo-zisk"
cd examples/recurser/hash

# 1. Build the guest ELF (zkVM target) via the host's build.rs → build_program,
#    which the allow-list needs. The guest is its own isolated workspace, so
#    this is the way to build it — not a plain `-p recurser-hash-guest`.
cargo build --release -p recurser-hash-host
ELF=guest/target/elf/riscv64ima-zisk-zkvm-elf/release/recurser_hash_guest

# 2. Write the leaf inputs (a.bin/b.bin/c.bin) with the exact bincode the guest
#    expects. The workspace target dir is examples/target (this is its own
#    workspace), so the built helper lives there.
cargo run --release -p recurser-hash-host --bin gen-inputs-hash

# 3. Set up the recurser.
$CARGO_ZISK setup --aggregation guest/aggregations/hash.toml

# 4. Prove each leaf.
$CARGO_ZISK prove -e "$ELF" -i file://a.bin -o a.proof
$CARGO_ZISK prove -e "$ELF" -i file://b.bin -o b.proof
$CARGO_ZISK prove -e "$ELF" -i file://c.bin -o c.proof

# 5. Fold A + B. Each side's free input is its own secret vector; the aggregate
#    binds Poseidon1(free) === the proof's digest, then outputs Poseidon1(va+vb).
$CARGO_ZISK aggregate --aggregation guest/aggregations/hash.toml \
  --proof-a a.proof --proof-b b.proof --output ab.proof \
  --free-inputs-a "1,2,3,4,5,6,7,8,9,10,11,12" \
  --free-inputs-b "100,101,102,103,104,105,106,107,108,109,110,111"

# 6. Fold AB + C. AB is an aggregated proof; its digest is Poseidon1(va+vb), so
#    its free input is the running sum va+vb. C's free input is vc.
$CARGO_ZISK aggregate --aggregation guest/aggregations/hash.toml \
  --proof-a ab.proof --proof-b c.proof --output abc.proof \
  --free-inputs-a "101,103,105,107,109,111,113,115,117,119,121,123" \
  --free-inputs-b "1000,1001,1002,1003,1004,1005,1006,1007,1008,1009,1010,1011"
```

`abc.proof` now commits `Poseidon1(va + vb + vc)`. The host adds what the CLI
doesn't: it **computes** those running sums (here you type them in) and **checks**
each folded digest against a native `Poseidon1(sum)`. To watch the binding fail,
rerun step 5 with a `--free-inputs-a` that doesn't hash to A's digest (bump the
first value to `2`): the fold becomes unsatisfiable and `aggregate` errors.

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
| [`guest/`](guest/src/main.rs) | Reads a `[u64; 12]` in one `read`, hashes via the shared `hash12`, commits the 4-element digest. |
| [`common/`](common/src/lib.rs) | Layout constants, `hash12` / `add_vecs`, and `secret_vectors` (the leaf values), shared by guest, host, and `gen-inputs`. `hash12` uses the precompile on the guest, native Rust on the host. |
| [`guest/aggregations/hash.toml`](guest/aggregations/hash.toml) | allow-list, `n-publics-agg = 4`, `free-inputs = 12`, normalize + aggregate circuits. |
| [`guest/aggregations/circuits/normalize.circom`](guest/aggregations/circuits/normalize.circom) | u32-limbs → field-elements reassembly. |
| [`guest/aggregations/circuits/aggregate.circom`](guest/aggregations/circuits/aggregate.circom) | sum the free vectors, `Poseidon(4)` → folded digest. |
| [`host/`](host/src/main.rs) | Proves the leaves, folds, checks each digest against native `Poseidon1(sum)`. |
