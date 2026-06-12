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
- runs the publics-handling circuits: per-program-group `NormalizePublics`
  (optional) and `AggregatePublics` (required) — see §5/§6,
- combines the two publics arrays into one,
- stamps a new `programVK` on the output so that a chain's identity
  propagates unchanged once it's been committed.

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
| **Aggregator-level** (prover-supplied) | `freeInputsA[K]`, `freeInputsB[K]` | per-proof side inputs consumed by `NormalizePublics` (§5); `K` = worst case across groups |
| | `rootCRecurserAgg[4]` | this aggregator's *own* VK — committed at the next level |
| **Hardcoded** (baked into the circuit at setup) | `programVKs[P][4]` | the **leaf allowlist** — VKs of all registered programs |
| | `rootCVadcopFinalZisk[4]` | verification key for ZisK proofs |
| **Out** | `publics[64]` | aggregated user publics |
| | `programVK[4]` | new chain identity, per §7 (Output programVK) |

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

`K` is the worst-case free-input count across normalization groups (0 when
there are none), fixed by the definition (`free-inputs` per group in the
TOML, or `normalize_with(..., n)` in the Rust API). `P` is the number of
registered programs. `programVKs[]` is derived from the definition's
programs at setup; `rootCVadcopFinalZisk` comes from the proving key.

> ⚠ **Zero-pad unused slots.** The publics array is always 64 long. If
> your app uses fewer than 64 publics, the leftover high-index slots have
> to be zero in every leaf proof — otherwise the prover can put arbitrary
> values there and they propagate through the fold tree unchecked. Nothing
> catches this by default. Either zero-pad in the producer circuit, or add
> `a_publics[i] === 0` and `b_publics[i] === 0` constraints for the unused
> range to your `AggregatePublics` body (§6).

---

## 2. Pipeline

The aggregator runs a linear sequence of stages:

| # | Stage | What happens | Detail |
|---|---|---|---|
| 1 | **Classify** A and B independently | Each proof's `programVK` is tested against the registered-program allowlist. Match ⇒ leaf, no match ⇒ aggregated. | §3 |
| 2 | **Pick `rootC`** per proof | Leaf ⇒ `rootCVadcopFinalZisk`. Aggregated ⇒ that proof's own `programVK`. | §4 |
| 3 | **Verify both STARK proofs** | Each proof's inner STARK verifier runs with its picked `rootC` and the proof data. | §4 |
| 4 | **NormalizePublics** *(optional, per group)* | Each leaf's publics run through its program group's circuit; aggregated proofs and ungrouped leaves pass through raw. | §5 |
| 5 | **AggregatePublics** *(required)* | User-supplied stitching constraints between A's and B's publics plus the combination of the two payloads. | §6 |
| 6 | **Pick output `programVK`** | One of four cases based on each side's leaf/aggregated status. | §7 |
| 7 | **Emit combined proof** | `(publics, programVK)` — the next fold-level's input. | — |

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
  aggregator, so `rootC` is the proof's own `programVK` — which by §9's
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

## 5. NormalizePublics (optional, per program group)

A normalisation hook applied to a leaf proof's publics *before* the
aggregation — the spot to rewrite raw publics (hash them, re-encode,
derive new values from that proof's free inputs) the first time the
proof enters the recursion. Normalization is declared per *group* of
registered programs: each group supplies its own circuit and side-input
count, and programs not covered by any group keep their publics unchanged.

| Proof type | Publics used downstream |
|---|---|
| leaf of a grouped program | that group's `NormalizePublics` output |
| leaf of an ungrouped program | raw |
| aggregated (`isRegisteredProgram = 0`) | raw |

In-circuit, the membership flags from §3 select the path: every group's
circuit is instantiated on both sides (circuits are static), and a
sum-of-masks mux picks at most one normalized result per proof — groups
are disjoint, so the selector weights sum to 1. Aggregated proofs were
already normalised at a prior level, so they pass through untouched and
the normalised form propagates unchanged up the whole tree.

### The circuit body

Each group's file defines this exact template name (the generator renames
it to `NormalizePublics_<g>` at injection so groups coexist):

```circom
template NormalizePublics(nPublics, nFreeInputs) {
    signal input publics[nPublics];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[nPublics];

    // ... your derivation logic ...
}
```

The aggregator instantiates each group `g` on both sides as:

```circom
signal normA_g[nPublics] <==
    NormalizePublics_g(nPublics, n_g)(aPublics, <leading n_g slots of freeInputsA>);
```

`nPublics` is fixed to 64 at the `Main(…)` instantiation. Each side's
`freeInputs` array is sized to the worst case across groups; a group
consuming fewer sees only its leading slice. The body can do anything
Circom supports — hashes, decompositions, derived values.

> ⚠ **No `===` constraints in `NormalizePublics`.** Circuits are static:
> every group's circuit runs on *every* proof's publics (aggregated proofs,
> other groups' leaves — with zeroed free inputs), and only the mux discards
> the unwanted results. An assertion inside a normalize body would therefore
> fire on inputs it was never meant to see and abort valid folds. Constraints
> belong in `AggregatePublics` (§6), which sees only the selected payloads.

### Free inputs are per proof

The prover supplies side inputs per *proof*, not per fold: in the SDK,
`proof.with_free_inputs(vec![...])` pairs a leaf with the data its
group's circuit consumes, while plain `&Proof` (aggregated proofs,
ungrouped leaves) carries none. The two arrays travel independently to
the circuit as `freeInputsA` / `freeInputsB`.

---

## 6. AggregatePublics (required)

Combines A's and B's `publics[nPublics]` arrays into one same-size output
that the next fold level consumes. Each output slot is some function of
the matching slots in A and B.

This is also where stitching constraints between A's and B's publics live —
e.g. "A's `endBlock` equals B's `startBlock`". A failed constraint aborts
the fold, so two proofs that aren't end-to-end can't be combined. And it's
where you constrain unused publics slots to zero (see the zero-pad warning
in §1): if your app uses 32 publics, add `a_publics[i] === 0` and
`b_publics[i] === 0` for `i = 32..64`.

Supply the body via the definition TOML's `aggregate-publics` key (§11) or
`CircomTemplates::aggregate_publics` in the Rust API. Required signature:

```circom
template AggregatePublics(nPublics) {
    signal output aggregated_publics[nPublics];
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];

    // ... your === stitching constraints ...
    // ... your combination logic ...
}
```

The aggregator calls it as:

```circom
signal aggregatedPublics[nPublics] <==
    AggregatePublics(nPublics)(ziskPublicsA, ziskPublicsB);
```

`ziskPublicsA` and `ziskPublicsB` are the post-normalization payloads from
§5. Every element of `aggregated_publics` must be driven by `<==` inside
the body or Circom errors. Free inputs are a normalization-only concern —
they flow into `NormalizePublics` (§5), not here; anything `AggregatePublics`
needs from side data should be baked into the normalized publics.

A common pattern is a per-slot pick of A's value or B's value: e.g.
`startBlock` inherits from A and `endBlock` from B, so the combined proof
attests the segment `[A.start, B.end]`. Sums, hashes, and conditional
combinations all work — it's plain Circom.

An inherit-from-A example lives at
[tests/fixtures/aggregate_publics.circom](../tests/fixtures/aggregate_publics.circom);
a full chain-fold example (stitch constraint + digest propagation) at
[test-artifacts/programs/aggregations/circuits/aggregate_publics.circom](../../test-artifacts/programs/aggregations/circuits/aggregate_publics.circom).

---

## 7. Output programVK

The output `programVK` becomes the next fold level's input `programVK`.
There are four cases:

| A type | B type | Output `programVK` | Why |
|---|---|---|---|
| leaf | leaf | `rootCRecurserAgg` | First fold of this chain — stamp the aggregator's own identity |
| leaf | agg | B's `programVK` | B's chain is already committed; A's leaf is absorbed into it |
| agg | leaf | A's `programVK` | Mirror of the above — A's chain dominates |
| agg | agg | shared VK | Both chains committed; §8 forces them to match |

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

## 8. Immutability check

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

## 9. Recursive invariant

> From level 1 onward, every proof in a chain carries the same `programVK` —
> the `rootCRecurserAgg` of that chain's level-1 fold.

```
   Level 0 (leaves)
       programVK = program's own VK          (∈ registered list)
            │
            ▼  fold (both leaves → §7 row 1)
   Level 1
       programVK = rootCRecurserAgg_lvl1     (∉ registered ⇒ "agg" from now on)
            │
            ▼  fold (any input is "agg" → §7 rows 2/3/4)
   Level 2
       programVK = rootCRecurserAgg_lvl1     ← inherited; locked by §8
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
  `rootCRecurserAgg_lvl1` by §7's first row.
- **Level k ≥ 2.** At least one input is aggregated, and by induction its
  `programVK` is `rootCRecurserAgg_lvl1`. §7 picks an aggregated input's
  `programVK` as the output (rows 2/3/4), so the output `programVK` is
  also `rootCRecurserAgg_lvl1`.
- **Chains can't be mixed.** Whenever both inputs at level ≥ 2 are
  aggregated, §8 forces `programVK_A == programVK_B`. Two proofs from
  different chains have different `rootCRecurserAgg_lvl1` values, so the
  equality fails and the fold is rejected.

---

## 10. Failure modes

| Stage | Triggers when… |
|---|---|
| STARK verify (vA, vB) | malformed witness or wrong VK; mismatched `rootC` ⇒ FRI / Merkle checks reject |
| AggregatePublics (§6) | stitching constraint broken |
| Immutability check (§8) | folding two aggregated proofs from different chains |
| Binary check on `isRegisteredProgram` | malicious witness tries to set `isRegisteredProgram_X` non-binary |

Every check is in-circuit, so a passing proof is sound.

---

## 11. Usage

The definition is authored once (a TOML next to the guest programs) and
consumed twice: by `build_program` at host-build time (for the SDK path)
and by the `cargo-zisk setup-recurser-aggregator` CLI at setup time.

### Prerequisites

The setup pipeline reads `provingKey/<name>/vadcop_final/` (verkey,
starkinfo, verifierinfo) from the *setup* directory. That folder is produced
by ZisK's `final` setup stage and has to exist before you run the
aggregator setup. The recurser writes its own artifacts to a *separate*
output directory — input and output paths must differ, so one ZisK setup
can feed any number of recurser configurations without cross-contaminating
the proving-key tree.

Output layout (under `--output-dir`, default `./build`):

```
provingKey/recurser/<recurser-id>/    recurser_aggregator.{dat,exec} + witness library
circom/                                recurser_aggregator.circom + vadcop_final stark verifier
build/                                 recurser_aggregator.{r1cs,fixed.bin,...}
pil/                                   recurser_aggregator.pil
```

The `<recurser-id>` segment lets a single output directory hold multiple
coexisting setups (different program-VK allowlists, different template
bodies, different normalization groups). The id is a content-addressed
blake3 hash of the circuit inputs — `program_vks`, the normalization
groups (member indices, body hash, side-input count each), and the
`aggregate_publics` body, together with the vadcop_final proving-key VK —
so identical inputs always resolve to the same id and any change produces
a fresh one. It's computed automatically and logged at startup; there's
no manual override.

The recurser doesn't nest under the source ZisK pilout name
(`provingKey/<name>/...`) because the artifacts here are
aggregator-scoped, not ZisK-program-scoped.

> ⚠ **Domain-size constraint.** The recurser-aggregator's own STARK
> `n_bits` (decided by `plonk2pil` from the generated R1CS) must equal
> `vadcop_final.starkStruct.nBits`. The aggregator's output proof has to be
> re-verifiable by the next fold level, which is the same circuit — so
> different domain sizes would panic the prover. The setup checks this
> after `plonk2pil` and bails with a message. If it fires, either shrink
> the recurser circuit (simpler or fewer `NormalizePublics` groups, a
> simpler `AggregatePublics`, fewer `free_inputs`) or rebuild
> `vadcop_final` with a larger `nBits`.

### The definition (build-time)

An aggregation program is *defined* next to the guest programs and built by
the same `cargo build` that compiles them — `build_program` in the host's
build.rs discovers `programs/aggregations/<name>.toml`, validates it
(guest names against the just-built ELFs, circuit files declare the
expected templates, groups disjoint — all build errors), and generates the
builder expression behind [`load_aggregation_program!`] (env
`ZISK_AGG_<name>`), with circuits and member ELFs embedded.

```toml
# programs/aggregations/chain.toml — circuits beside it
programs = ["chain_segment"]            # guest names, same as load_program!
aggregate-publics = "circuits/aggregate_publics.circom"

[[normalize]]
template = "circuits/normalize.circom"   # defines `template NormalizePublics(nPublics, nFreeInputs)`
free-inputs = 1
programs = ["chain_segment"]
```

### The setup (machine-time)

The CLI consumes the same TOML, resolving guest names against the built
ELFs (so run the guests' `cargo build` first):

```text
cargo-zisk setup-recurser-aggregator --aggregation programs/aggregations/chain.toml
```

| Flag | Required | Description |
|---|---|---|
| `--aggregation` | yes | The definition TOML under `<programs>/aggregations/` |
| `--release` | no | Resolve guest ELFs from the release profile instead of debug |
| `--setup-dir` | no | ZisK setup directory to read from (contains `provingKey/<name>/vadcop_final/`). Defaults to `~/.zisk` |
| `--output-dir` | no | Where to write the recurser-aggregator artifacts. Must differ from `--setup-dir`. Defaults to `./build` |
| `--proving-key` | no | Path to the proving key used to build the `ProofCtx` rom-setup runs against. Defaults to the standard ZisK location |
| `--cache-dir` | no | rom-setup cache directory (`<elfHash>_<pkHash>_…verkey.bin` artifacts). Defaults to `~/.zisk/cache` |

VK derivation is cache-aware: if `rom_merkle_setup`
finds the matching `*.verkey.bin` already in `--cache-dir`, it just reads
it (no recompute). The subcommand lives at
[`cli/src/commands/setup_recurser_aggregator.rs`](../../cli/src/commands/setup_recurser_aggregator.rs);
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

// Leaves carry their group's free inputs; aggregated proofs are plain refs.
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
directly (`new(guests, circuit)` + `normalize_with(...)` + `build()`), or
go lower with
[`run_setup_recurser_aggregator`](../src/setup/command.rs) /
[`gen_recurser`](../src/templates.rs).