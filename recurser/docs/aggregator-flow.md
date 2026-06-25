---
title: Recurser Aggregator — flow
tags: recurser, circom, zisk
---

# Recurser Aggregator

> Folds two STARK proofs into one. Applied repeatedly down a binary
> recursion tree, it collapses N program executions into a single final
> proof.

---

## 0. Why this exists

One aggregator circuit has to handle two kinds of input proof:

1. **Leaf proofs** — direct executions of a registered ZisK program. The
   proof carries its program's own VK as `programVK`, and the recurser
   verifies it using `rootCVadcopFinalZisk` (the verification key for ZisK
   proofs, hardcoded at setup) as `rootC`.
2. **Aggregated proofs** — outputs of a prior fold. The proof carries the
   previous aggregator's own VK as `programVK`, and the recurser verifies
   it using that same VK as `rootC`. So for aggregated proofs,
   `programVK == rootC`: the proof carries inside itself the key needed
   to verify it.

The aggregator muxes its inner STARK verifier's `rootC` between the
ZisK-proof VK (when the input is a leaf) and the proof's own `programVK`
(when the input is aggregated). One circuit, both proof types.

Each input is classified independently, so one fold can mix any
combination — leaf+leaf, leaf+agg, agg+leaf, agg+agg. The aggregator:

- verifies both input proofs with the right `rootC`,
- runs the `AggregatePublics` circuit (required) over both proofs' raw
  publics (plus any free inputs) — see §5,
- combines the two publics arrays into one,
- stamps a new `programVK` on the output so that a chain's identity
  propagates unchanged once it's been committed.

> **Single publics layout.** Every accepted guest program must emit the
> *same* publics layout. The aggregator folds each proof's publics through
> raw, directly into `AggregatePublics`, so it has no per-program rewrite
> step — aggregating programs with different publics layouts is intentionally
> unsupported.

---

## 1. Inputs / outputs

Every input proof has two top-level pieces: `publics` and `programVK`.
The aggregator consumes two proofs and emits one.

```
   Proof A: (publics, programVK) ─┐
                                   ├──► Aggregator ──► Proof (publics, programVK)
   Proof B: (publics, programVK) ─┘
```

| Source | Item | Description |
|---|---|---|
| **Per proof** (prover-supplied) | `publics[64]` | the 64 user publics |
| | `programVK[4]` | the proof's identity — see §3 (Classification) |
| | STARK data | commits, FRI evals, siblings, nonce — verified internally |
| **Aggregator-level** (prover-supplied) | `freeInputsA[K]`, `freeInputsB[K]` | per-proof side inputs consumed by `AggregatePublics` (§5); `K` = the aggregate stage's free-input count |
| | `rootCRecurserAgg[4]` | this aggregator's *own* VK — committed at the next level |
| **Hardcoded** (baked into the circuit at setup) | `programVKs[P][4]` | the **leaf allowlist** — VKs of all registered programs |
| | `rootCVadcopFinalZisk[4]` | verification key for ZisK proofs |
| **Out** | `publics[64]` | aggregated user publics |
| | `programVK[4]` | new chain identity, per §6 (Output programVK) |

The user publics count is fixed at 64 — that's the ZisK publics layout.
The `vadcop_final` STARK proof has 64 user publics plus a 4-element
program-VK slot, so the recurser's aggregator is hardcoded to the same
shape. Changing it would mean re-generating ZisK setup.

> **Publics layout — VK first.** Inside a proof's `public_values` blob (and
> the STARK verifier's `publics[68]`), the 4-limb `programVK` occupies the
> *leading* slots `[0..4)` and the 64 user publics follow at `[4..68)`. This
> matches ZisK's `state-machines/publics.json` (`rom_root` at `initialPos 0`
> with `verificationKey: true`, `inputs` at `initialPos 4`) and the Rust side
> (`common/src/proof.rs`, `recurser/src/prove/validate.rs`). The aggregator
> circuit reads and re-emits the VK from these leading slots, so its output
> proof re-verifies one fold up.

`K` is the aggregate stage's free-input count (0 when there are none),
fixed by the definition (`aggregate-free-inputs` in the TOML, or
`.aggregate_free_inputs(n)` in the Rust API). `P` is the number of
registered programs. `programVKs[]` is derived from the definition's
programs at setup; `rootCVadcopFinalZisk` comes from the proving key.

> ⚠ **Zero-pad unused slots.** The publics array is always 64 long. If
> your app uses fewer than 64 publics, the leftover high-index slots have
> to be zero in every leaf proof — otherwise the prover can put arbitrary
> values there and they propagate through the fold tree unchecked. Nothing
> catches this by default. Either zero-pad in the producer circuit, or add
> `a_publics[i] === 0` and `b_publics[i] === 0` constraints for the unused
> range to your `AggregatePublics` body (§5).

---

## 2. Pipeline

The aggregator runs a linear sequence of stages:

| # | Stage | What happens | Detail |
|---|---|---|---|
| 1 | **Classify** A and B independently | Each proof's `programVK` is tested against the registered-program allowlist. Match ⇒ leaf, no match ⇒ aggregated. | §3 |
| 2 | **Pick `rootC`** per proof | Leaf ⇒ `rootCVadcopFinalZisk`. Aggregated ⇒ that proof's own `programVK`. | §4 |
| 3 | **Verify both STARK proofs** | Each proof's inner STARK verifier runs with its picked `rootC` and the proof data. | §4 |
| 4 | **AggregatePublics** *(required)* | Consumes both proofs' raw publics plus their free inputs: user-supplied stitching constraints between A's and B's publics plus the combination of the two payloads. | §5 |
| 5 | **Pick output `programVK`** | One of four cases based on each side's leaf/aggregated status. | §6 |
| 6 | **Emit combined proof** | `(publics, programVK)` — the next fold-level's input. | — |

Each row corresponds to a contiguous block in
[aggregator.circom.tera](../templates/aggregator.circom.tera).

---

## 3. Classification: leaf or aggregated?

The aggregator compares each proof's `programVK` against the hardcoded
`programVKs[]` allowlist using an `IsEqualVK` helper (per-element `IsZero`
AND'd across the 4 elements), then folds the indicators into a 0/1
membership flag via the complement of a product:

```
eq_X[k]                =  IsEqualVK(programVK_X, programVKs[k])
noMatch_X              =  ∏_k (1 − eq_X[k])
isRegisteredProgram_X  =  1 − noMatch_X
```

Each `eq_X[k]` is `{binary}`, so `(1 − eq_X[k])` is `{binary}`, and the
running product `noMatch_X[k]` stays `{binary}` through every multiply —
which is what lets `isRegisteredProgram_X` flow into the `MultiMux1`
selector below without a circom tag error. (An equivalent
`Σ_k eq_X[k]` formulation would be cleaner on paper but addition strips
the binary tag in circom 2.1.)

Soundness of the formulation rests on `programVKs[]` containing no
duplicates — enforced at setup time by the CLI, not in-circuit. With
duplicates, two `eq_X[k]` could fire simultaneously for the same input
and the membership flag would still come out 1, but the registry would
have admitted the same program twice, which is meaningless.

| Type | `programVK` is… | `rootC` used to verify it | `isRegisteredProgram` |
|---|---|---|---|
| **LEAF** | one of the registered program VKs | `rootCVadcopFinalZisk` | **1** |
| **AGGREGATED** | a prior aggregator's `rootCRecurserAgg` | the proof's own `programVK` (so `programVK == rootC`) | **0** |

A and B are classified independently.

---

## 4. rootC selection

Every STARK verifier needs a `rootC` — the verification key for the proof's
constants polynomial (i.e. which circuit produced this proof). The recurser
picks it per proof based on the classification from §3:

- **Leaf proofs** are ZisK proofs, so `rootC = rootCVadcopFinalZisk` (the
  ZisK-proof VK, hardcoded at setup).
- **Aggregated proofs** were produced by an earlier level of this same
  aggregator, so `rootC` is the proof's own `programVK` — which by §8's
  invariant is the prior level's `rootCRecurserAgg`.

```
                  isRegisteredProgram = 1               isRegisteredProgram = 0
                  (leaf)                             (aggregated)
                       │                                   │
                       ▼                                   ▼
                rootCVadcopFinalZisk[4]              programVK[4]
                       └────────────── MultiMux1 ─────────┘
                                       │
                                       ▼
                                  vA.rootC  (or vB)
```

One mux per proof. One circuit, both proof types, no duplicate verifier.

---

## 5. AggregatePublics (required)

Combines A's and B's `publics[64]` arrays into one same-size output that
the next fold level consumes. The width is ZisK's fixed 64 user-publics
slots, so it is hardcoded — `AggregatePublics` takes only `nFreeInputs` as
a template parameter, not `nPublics`. Each output slot is some function of
the matching slots in A and B. Each proof's publics flow in *raw* — there
is no per-program rewrite step, so every accepted guest program must emit
the same publics layout (see the overview).

This is also where stitching constraints between A's and B's publics live —
e.g. "A's `endBlock` equals B's `startBlock`". A failed constraint aborts
the fold, so two proofs that aren't end-to-end can't be combined. And it's
where you constrain unused publics slots to zero (see the zero-pad warning
in §1): if your app uses 32 publics, add `a_publics[i] === 0` and
`b_publics[i] === 0` for `i = 32..64`.

Supply the body via the definition TOML's `aggregate-publics` key (§10) or
`CircomTemplates::aggregate_publics` in the Rust API. Required signature:

```circom
template AggregatePublics(nFreeInputs) {
    signal output aggregated_publics[64];
    signal input a_publics[64];
    signal input b_publics[64];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];

    // ... your === stitching constraints ...
    // ... your combination logic ...
}
```

The aggregator calls it as:

```circom
signal aggPublics[64] <==
    AggregatePublics(nFreeInputs)(ziskPublicsA, ziskPublicsB, freeInputsA, freeInputsB);
```

`ziskPublicsA` and `ziskPublicsB` are the two proofs' raw publics; every
element of `aggregated_publics` must be driven by `<==` inside the body or
Circom errors.

### Free inputs

Each proof carries `nFreeInputs` side inputs (`aggregate-free-inputs` in
the TOML / `.aggregate_free_inputs(n)` in the Rust API), supplied by the
prover and routed straight into `AggregatePublics` as `free_inputs_a` /
`free_inputs_b`. The canonical use case is hash-style publics: each side's
preimage is supplied as free inputs so `AggregatePublics` can check the
hash against the publics, recombine the two preimages, and re-hash into the
aggregated output. In the SDK, `proof.with_free_inputs(vec![...])` attaches
a proof's free inputs; plain `&Proof` carries none.

A common pattern is a per-slot pick of A's value or B's value: e.g.
`startBlock` inherits from A and `endBlock` from B, so the combined proof
attests the segment `[A.start, B.end]`. Sums, hashes, and conditional
combinations all work — it's plain Circom.

An inherit-from-A example lives at
[tests/fixtures/aggregate_publics.circom](../tests/fixtures/aggregate_publics.circom);
a full chain-fold example (stitch constraint + zero-filled tail) at
[test-artifacts/programs/aggregations/circuits/aggregate_publics.circom](../../test-artifacts/programs/aggregations/circuits/aggregate_publics.circom).

---

## 6. Output programVK

The output `programVK` becomes the next fold level's input `programVK`.
There are four cases:

| A type | B type | Output `programVK` | Why |
|---|---|---|---|
| leaf | leaf | `rootCRecurserAgg` | First fold of this chain — stamp the aggregator's own identity |
| leaf | agg | B's `programVK` | B's chain is already committed; A's leaf is absorbed into it |
| agg | leaf | A's `programVK` | Mirror of the above — A's chain dominates |
| agg | agg | shared VK | Both chains committed; §7 forces them to match |

In Circom this is a sum of masks with three mutually-exclusive selectors
that sum to 1:

```circom
signal selectAgg <== isRegisteredProgramA * isRegisteredProgramB;        // both leaves           → rootCRecurserAgg
signal selectAVK <== 1 - isRegisteredProgramA;                         // A aggregated (any B)  → A.programVK
signal selectBVK <== isRegisteredProgramA * (1 - isRegisteredProgramB);   // A leaf, B aggregated  → B.programVK

for (var i = 0; i < 4; i++) {
    outProgramVK[i] <== selectAgg * rootCRecurserAgg[i]
                      + selectAVK * programVK_A[i]
                      + selectBVK * programVK_B[i];
}
```

For any `(isRegisteredProgramA, isRegisteredProgramB) ∈ {0,1}²`, the
selectors sum to 1 algebraically, so exactly one of the three terms is
non-zero in any given fold.

---

## 7. Immutability check

Once a chain commits to a `programVK` (at its first fold), that VK has to
propagate upward unchanged. The check:

```circom
signal bothAggregated <== (1 - isRegisteredProgramA) * (1 - isRegisteredProgramB);
for (var i = 0; i < 4; i++) {
    bothAggregated * (programVK_A[i] - programVK_B[i]) === 0;
}
```

`bothAggregated` is 1 iff both inputs are aggregated, 0 otherwise. When
it's 0 the constraint is vacuously satisfied — mismatched leaves don't
violate anything, since leaves carry their program's own VK, not a chain
identity. When it's 1, the constraint forces element-wise equality of
`programVK_A` and `programVK_B`.

If the constraint fails, the prover was trying to fold two proofs from
different recursion chains. That's what we want to forbid.

---

## 8. Recursive invariant

> From level 1 onward, every proof in a chain carries the same `programVK` —
> the `rootCRecurserAgg` of that chain's level-1 fold.

```
   Level 0 (leaves)
       programVK = program's own VK          (∈ registered list)
            │
            ▼  fold (both leaves → §6 row 1)
   Level 1
       programVK = rootCRecurserAgg_lvl1     (∉ registered ⇒ "agg" from now on)
            │
            ▼  fold (any input is "agg" → §6 rows 2/3/4)
   Level 2
       programVK = rootCRecurserAgg_lvl1     ← inherited; locked by §7
            │
            ▼  fold
   Level 3, 4, …
       programVK = rootCRecurserAgg_lvl1     ← still
```

| Level | `programVK` is… | Classified as (when fed to the next level) |
|---|---|---|
| 0 (leaf) | the program's own VK | leaf |
| 1 | `rootCRecurserAgg_lvl1` (this aggregator's own VK) | aggregated |
| ≥ 2 | same `rootCRecurserAgg_lvl1` | aggregated |

Why this holds:

- **Level 1.** Both inputs are leaves, so the output is
  `rootCRecurserAgg_lvl1` by §6's first row.
- **Level k ≥ 2.** At least one input is aggregated, and by induction its
  `programVK` is `rootCRecurserAgg_lvl1`. §6 picks an aggregated input's
  `programVK` as the output (rows 2/3/4), so the output `programVK` is
  also `rootCRecurserAgg_lvl1`.
- **Chains can't be mixed.** Whenever both inputs at level ≥ 2 are
  aggregated, §7 forces `programVK_A == programVK_B`. Two proofs from
  different chains have different `rootCRecurserAgg_lvl1` values, so the
  equality fails and the fold is rejected.

---

## 9. Failure modes

| Stage | Triggers when… |
|---|---|
| STARK verify (vA, vB) | malformed witness or wrong VK; mismatched `rootC` ⇒ FRI / Merkle checks reject |
| AggregatePublics (§5) | stitching constraint broken |
| Immutability check (§7) | folding two aggregated proofs from different chains |
| Binary check on `isRegisteredProgram` | malicious witness tries to set `isRegisteredProgram_X` non-binary |

Every check is in-circuit, so a passing proof is sound.

---

## 10. Usage

The definition is authored once (a TOML next to the guest programs) and
consumed twice: by `build_program` at host-build time (for the SDK path)
and by the `cargo-zisk setup --aggregation` CLI at setup time.

### Prerequisites

The setup pipeline reads `provingKey/<name>/vadcop_final/` (verkey,
starkinfo, verifierinfo) from the *setup* directory. That folder is produced
by ZisK's `final` setup stage and has to exist before you run the
aggregator setup. The recurser writes its own artifacts to a *separate*
output directory — input and output paths must differ, so one ZisK setup
can feed any number of recurser configurations without cross-contaminating
the proving-key tree.

Output layout (under the output directory, `~/.zisk/recurser` via the CLI/SDK):

```
provingKey/recurser/<recurser-id>/    recurser_aggregator.{dat,exec} + witness library
circom/                                recurser_aggregator.circom + vadcop_final stark verifier
build/                                 recurser_aggregator.{r1cs,fixed.bin,...}
pil/                                   recurser_aggregator.pil
```

The `<recurser-id>` segment lets a single output directory hold multiple
coexisting setups (different program-VK allowlists, different template
bodies, different free-input counts). The id is a content-addressed
blake3 hash of the circuit inputs — `program_vks`, the `aggregate_publics`
body, and `aggregate_n_free_inputs`, together with the vadcop_final
proving-key VK — so identical inputs always resolve to the same id and any
change produces a fresh one. It's computed automatically and logged at
startup; there's no manual override.

The recurser doesn't nest under the source ZisK pilout name
(`provingKey/<name>/...`) because the artifacts here are
aggregator-scoped, not ZisK-program-scoped.

> ⚠ **Domain-size constraint.** The recurser-aggregator's own STARK
> `n_bits` (decided by `plonk2pil` from the generated R1CS) must equal
> `vadcop_final.starkStruct.nBits`. The aggregator's output proof has to be
> re-verifiable by the next fold level, which is the same circuit — so
> different domain sizes would panic the prover. The setup checks this
> after `plonk2pil` and bails with a message. If it fires, either shrink
> the recurser circuit (a simpler `AggregatePublics`, fewer free inputs) or
> rebuild `vadcop_final` with a larger `nBits`.

### The definition (build-time)

An aggregation program is *defined* next to the guest programs and built by
the same `cargo build` that compiles them — `build_program` in the host's
build.rs discovers `programs/aggregations/<name>.toml`, validates it
(guest names against the just-built ELFs, circuit files declare the
expected templates — all build errors), and generates the
builder expression behind [`load_aggregation_program!`] (env
`ZISK_AGG_<name>`), with circuits and member ELFs embedded.

```toml
# programs/aggregations/myagg.toml — circuits beside it
programs = ["seg_a", "seg_b"]           # guest names, same as load_program!
aggregate-publics = "circuits/aggregate_publics.circom"
aggregate-free-inputs = 1               # optional; per-proof free inputs into
                                        # AggregatePublics (default 0 — omit if unused,
                                        # as the bundled `chain` example does)
```

### The setup (machine-time)

The CLI consumes the same TOML, resolving guest names against the built
ELFs (so run the guests' `cargo build` first):

```text
cargo-zisk setup --aggregation programs/aggregations/myagg.toml
```

The command runs the setup through the SDK: it reads the vadcop_final setup
from `~/.zisk` and writes the recurser artifacts to
`~/.zisk/recurser/<recurser-id>` (the SDK-managed location `aggregate`
and the workers resolve against).

| Flag | Required | Description |
|---|---|---|
| `--aggregation` | yes | The definition TOML under `<programs>/aggregations/` |
| `--release` | no | Resolve guest ELFs from the release profile instead of debug |
| `--proving-key` | no | Path to a precomputed proving key. Defaults to the standard ZisK location |

VK derivation is cache-aware: if the matching `*.verkey.bin` is already in
the rom-setup cache (`~/.zisk/cache`), it is read back without recompute.
The subcommand lives at
[`cli/src/commands/user/embedded/recurser.rs`](../../cli/src/commands/user/embedded/recurser.rs);
the lib API ([`SetupRecurserAggregatorOptions`](../src/setup/command.rs))
takes program VKs and circuit bodies inline for callers that already have
them.

### Rust API (SDK)

With the definition above and `build_program` in build.rs, the whole thing
is one identifier — the mirror of `load_program!` for guest programs:

```rust
use zisk_sdk::{load_aggregation_program, AggregationProgram, ProofExt};

static AGG: AggregationProgram = load_aggregation_program!("chain");

client.setup(&AGG).run()?.await?;

// Proofs can carry free inputs for AggregatePublics; plain refs carry none.
let ab = client
    .aggregate_proofs(&AGG, pa.with_free_inputs(vec![4]), pb.with_free_inputs(vec![4]))
    .run()?
    .await?;
let root = client.aggregate_proofs(&AGG, &ab, &cd).run()?.await?;
```

The lazy `build()` behind the static derives each program's VK and computes
the content-addressed `recurser_id` on first use (proving-key dependent, so
it can't happen at compile time). For dynamic composition without the
build pipeline, construct an
[`AggregationProgram`](../../sdk/src/recurser.rs)
directly (`AggregationProgramBuilder::new(guests, aggregate_circuit)`
`.aggregate_free_inputs(n)` `.build()`), or go lower with
[`run_setup_recurser_aggregator`](../src/setup/command.rs) /
[`gen_recurser`](../src/templates.rs).