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

1. **Leaf proofs** — direct executions of a ZisK program. The proof
   carries the vadcop_final VK as `programVK` and is verified using
   `rootCVadcopFinalZisk` (the ZisK proof verification key, hardcoded
   at setup) as `rootC`. Its reserved slot 0 (`is_vadcop_final_proof`)
   is set to **1** by proofman.
2. **Aggregated proofs** — outputs of a prior fold. The proof carries
   this aggregator's own VK as `programVK` and is verified using that
   same VK as `rootC`. Its reserved slot 0 is **0** (forced by the
   aggregator at output time).

The aggregator reads `is_vadcop_final_proof` from slot 0 of each input
proof's publics to classify it, then muxes `rootC` accordingly. One
circuit, both proof types.

Each input is classified independently, so one fold can mix any
combination — leaf+leaf, leaf+agg, agg+leaf, agg+agg. The aggregator:

- verifies both input proofs with the right `rootC`,
- runs the optional `NormalizePublics` circuit on leaves (§5),
- runs `AggregatePublics` on both proofs' (post-normalize) publics (§6),
- stamps a new `programVK` on the output so a chain's identity
  propagates unchanged once committed,
- forces its own output slot 0 (`is_vadcop_final_proof`) to 0.

---

## 1. Inputs / outputs

Every input proof has a `publics[OUTPUT_LEN]` buffer laid out as
described in §2. The aggregator consumes two proofs and emits one.

```
   Proof A: publics[OUTPUT_LEN] ─┐
                                  ├──► Aggregator ──► Proof publics[OUTPUT_LEN]
   Proof B: publics[OUTPUT_LEN] ─┘
```

| Source | Item | Description |
|---|---|---|
| **Per proof** (prover-supplied) | `publics[OUTPUT_LEN]` | flag at slot 0, VK at [1..5), user inputs at [5..69) |
| | STARK data | commits, FRI evals, siblings, nonce — verified internally |
| **Aggregator-level** (prover-supplied) | `freeInputsA[n_free]` | side A's single free-input array (only present if `n_free > 0`) |
| | `freeInputsB[n_free]` | side B's single free-input array (only present if `n_free > 0`) |
| | `rootCRecurserAgg[4]` | this aggregator's own VK — committed at the next level |
| **Hardcoded** (baked into the circuit at setup) | `rootCVadcopFinalZisk[4]` | verification key for ZisK vadcop_final proofs |
| **Out** | `publics[OUTPUT_LEN]` | flag=0, aggregated VK, aggregated user publics |

---

## 2. Publics layout

The fold-layer publics buffer is one slot wider than the SNARK-layer
`state-machines/publics.json` (which is flag-free): the fold layer carries the
`is_vadcop_final_proof` flag at slot 0, ahead of the VK/inputs that
`publics.json` describes.

| Slot(s) | Name | Description |
|---|---|---|
| `0` | `is_vadcop_final_proof` | 1 = raw ZisK vadcop_final leaf, 0 = aggregator output |
| `[1..5)` | `programVK` / `rom_root` | 4-limb verification key (`verificationKey: true`) |
| `[5..69)` | user inputs | the 64 ZisK user publics |

Named constants in the generated circuit (defined in the `Main` template):
- `VK_BASE = 1`
- `PUBLICS_BASE = VK_BASE + PROGRAM_VK_LEN = 5`
- `OUTPUT_LEN = PUBLICS_BASE + nPublics = 69`
- `PROGRAM_VK_LEN = 4`

`nPublics` is fixed to 64 (the ZisK user-publics count). `OUTPUT_LEN` is
69 total. The `ZISK_PUBLICS()` circom function (defined by the scaffolding)
returns 64 so that `AggregatePublics` can size its arrays without taking
`nPublics` as a template parameter.

> **Slot-0 flag contract.** The vadcop_final circuit emits
> `signal output is_vadcop_final_proof <== 1` — a circom main output, which
> becomes STARK public slot 0 (ordered ahead of the input publics) on every
> genuine leaf. The `final_compressed` / `recursion_final` layers strip this
> slot, so it never reaches the flag-free SNARK-layer `publics.json` / on-chain
> hash. This aggregator (the fold layer) reads slot 0 to classify leaf vs
> aggregated.

> **Unused output slots are zero-filled for you.** The user publics array is
> always 64 long, but `AggregatePublics` outputs only the `n-publics-agg` slots
> your app uses; the scaffolding zero-fills the `[n-publics-agg, 64)` tail on the
> fold *output* (§6), so those slots can't carry arbitrary values up the tree —
> no producer zero-padding or manual output `=== 0` required. Note this covers
> outputs only: the *inputs* `a_publics` / `b_publics` are still full 64-wide and
> unconstrained in the tail, so if your body reads an input slot `>= n-publics-agg`
> (e.g. into a stitch constraint), constrain it yourself.

---

## 3. Classification: leaf or aggregated?

The aggregator reads `is_vadcop_final_proof` from the reserved slot 0 of
each proof's publics:

```circom
signal {binary} isFinalA <== a_sv_publics[0];
signal {binary} isFinalB <== b_sv_publics[0];
// Self-defending: enforce boolean, don't merely trust the {binary} tag.
isFinalA * (1 - isFinalA) === 0;
isFinalB * (1 - isFinalB) === 0;
```

The boolean constraint is self-defending: the circuit does not merely rely
on proofman's `{binary}` annotation to survive, it enforces it. The mux
and rootC selection below are only sound for `sel ∈ {0,1}`.

| Value | Proof type | Source |
|---|---|---|
| **1** | LEAF — raw ZisK vadcop_final proof | Emitted by the vadcop_final circuit (`signal output is_vadcop_final_proof <== 1`) |
| **0** | AGGREGATED — output of this aggregator | Forced to 0 by the aggregator at output |

A and B are classified independently.

> The `is_vadcop_final_proof` flag is what drives leaf-vs-aggregated selection
> (rootC, normalize mux, output VK). It is independent of the optional leaf
> allow-list described in §3a: the flag says *how* to verify a proof; the
> allow-list (when configured) says *which* leaf programs are permitted.

---

## 3a. Optional leaf allow-list (access control)

By default the recurser is **VK-agnostic**: any valid ZisK vadcop_final leaf is
accepted. A definition may optionally pin an allow-list of guest programs, and
then only those programs' proofs are accepted as leaves.

When the allow-list is non-empty the circuit bakes the members' 4-limb
`programVK`s and enforces membership on every proof that claims to be a leaf
(`isFinal = 1`); an unregistered leaf makes the circuit unsatisfiable:

```circom
// Emitted only when `programs` is non-empty (guarded by the n_programs > 0 branch).
var programVKs[n_programs][4] = [ ... baked limbs ... ];

signal {binary} isRegisteredProgramA <== 1 - noMatchA[n_programs - 1];  // 1 - ∏(1 - eq[k])
signal {binary} isRegisteredProgramB <== 1 - noMatchB[n_programs - 1];

// A claimed leaf MUST be a registered program (aggregated proofs, isFinal=0, are unaffected).
isFinalA * (1 - isRegisteredProgramA) === 0;
isFinalB * (1 - isRegisteredProgramB) === 0;
```

Membership uses an `IsEqualVK()` helper (element-wise limb equality via
`IsZero`) and the product form `1 - ∏(1 - eq[k])` so circom 2.1's `{binary}`
tag survives. Soundness assumes the baked `programVKs[]` have no duplicates —
the SDK/CLI enforce uniqueness when deriving them.

The allow-list is committed into `recurser_id` (§10) and its order is
significant. An empty allow-list emits none of this machinery (byte-identical
to the VK-agnostic baseline).

---

## 4. rootC selection

Every STARK verifier needs a `rootC`. The recurser picks it per proof:

- **Leaf proofs** (`isFinal = 1`): `rootC = rootCVadcopFinalZisk`.
- **Aggregated proofs** (`isFinal = 0`): `rootC = programVK` (the proof's
  own identity — by §9's invariant this equals the prior level's
  `rootCRecurserAgg`).

```circom
vA.rootC <== MultiMux1(4)([programVK_A, rootCVadcopFinalZisk], isFinalA);
vB.rootC <== MultiMux1(4)([programVK_B, rootCVadcopFinalZisk], isFinalB);
```

One mux per proof. One circuit, both proof types.

---

## 5. NormalizePublics (optional, single circuit, all leaves)

An optional normalisation hook applied to a leaf proof's publics
*before* aggregation. When configured, a single `NormalizePublics`
circuit runs on every leaf — there are no per-program groups and no
member subsets. Aggregated proofs pass through raw.

| Proof type | `isFinal` | Publics used downstream |
|---|---|---|
| leaf | 1 | `NormalizePublics` output |
| aggregated | 0 | raw publics (unchanged) |

The mux (when normalize is configured), shown here in its algebraically-equivalent
inline form — the generated circuit splits it into `selNorm`/`selRaw` intermediate
signals (circom 2.1 forbids nonlinear terms in a single `<==`):

```circom
// conceptual; generated form uses selNormA/selRawA intermediates
ziskPublicsA[i] <== isFinalA * normA[i] + (1 - isFinalA) * aPublics[i];
ziskPublicsB[i] <== isFinalB * normB[i] + (1 - isFinalB) * bPublics[i];
```

When `normalize` is `None` (not configured), the circuit emits the raw
passthrough directly — no mux is generated, output is byte-identical to
the no-normalize baseline.

### Conditional template signature

The template signature depends on the single declared free-input count `n_free`:

- `n_free == 0` → `template NormalizePublics()`, called as
  `NormalizePublics()(aPublics)` — no free inputs or outputs.
- `n_free > 0` → `template NormalizePublics(nFreeInputs)`, which
  MUST emit a `free_outputs[nFreeInputs]` output (see below).

Publics arrays are sized via `ZISK_PUBLICS()` (a file-scope function the
generator emits), so `NormalizePublics` takes no publics-width param —
matching `AggregatePublics` (§6).

The generator asserts the user-authored body's `template NormalizePublics(...)` arity
matches the declared count at build time (`InvalidTemplates` error on mismatch), and
— when `n_free > 0` — that it declares `signal output free_outputs`.

### The circuit body (n_free > 0 example)

```circom
template NormalizePublics(nFreeInputs) {
    signal input publics[ZISK_PUBLICS()];
    signal input free_inputs[nFreeInputs];
    signal output recurser_publics[ZISK_PUBLICS()];
    signal output free_outputs[nFreeInputs];

    // ... your derivation logic ...
}
```

The body can do anything Circom supports — hashes, decompositions,
derived values. `recurser_publics` is `ZISK_PUBLICS()` wide (same as input);
`free_outputs` is `n_free` wide and feeds `AggregatePublics` (§6).

> ⚠ **No `===` constraints in `NormalizePublics`.** The circuit runs on
> every proof (including aggregated proofs, which `isFinal=0` gates out
> via the mux). An assertion inside the normalize body would fire on
> inputs it was never meant to see. Constraints belong in `AggregatePublics`
> (§6), which sees only the muxed payloads.

### Free inputs — one array per side

There is a **single** free-input count `n_free` and one free-input array per
side, emitted only when `n_free > 0`:

```circom
signal input freeInputsA[n_free];
signal input freeInputsB[n_free];
```

Their meaning depends on whether the side is a leaf or an aggregated proof:

- **Leaf** (`isFinal = 1`): `freeInputsX` are the raw free inputs fed to
  `NormalizePublics`, whose `free_outputs` then feed `AggregatePublics`.
- **Aggregated** (`isFinal = 0`): normalize is muxed out, so `freeInputsX`
  are already the accumulated free values (a prior fold's `free_outputs`) and
  flow directly to `AggregatePublics`.

A free-value mux mirrors the publics mux and picks the source per side:

```circom
aggFreeX[i] <== isFinalX * normFreeOutX[i] + (1 - isFinalX) * freeInputsX[i];
```

`aggFreeA` / `aggFreeB` are what `AggregatePublics` receives. On the wire, the
proofman API carries one array per side (`free_inputs_a`, `free_inputs_b`), each
`n_free` wide — no concatenation or splitting.

> ⚠ **Exact width, no padding.** Each free array must be supplied at exactly
> `n_free` values (0 when `n_free == 0`). The backend fills the witness buffer
> positionally — `proof_a`, `proof_b`, `freeInputsA`, `freeInputsB`,
> `rootCRecurserAgg` — appending each free array by its supplied length with no
> zero-padding. A wrong length shifts `rootCRecurserAgg` and shears the witness,
> so both under- and oversupply are rejected up front (`validate_prove_inputs`,
> and the SDK's `aggregate_proofs`). A plain `&Proof` supplies an empty array,
> valid only when `n_free == 0`; otherwise use `.with_free_inputs(..)`.

---

## 6. AggregatePublics (required)

Combines A's and B's post-normalize `ziskPublicsX` arrays into the output.
Inputs are ZisK's fixed 64 user-publics slots, exposed by `ZISK_PUBLICS()`.
The **output** is only `nPublicsAgg` wide — the number of slots the
aggregation actually populates, set by the recurser's required
`n-publics-agg` config and threaded in as the trailing template
parameter. The scaffolding zero-fills the remaining
`[nPublicsAgg, ZISK_PUBLICS())` tail **outside** this template, so the
body never writes a padding loop and a prover cannot inject values into unused
slots. Each output slot is some function of the matching slots in A and B.

This is also where stitching constraints between A's and B's publics live
— e.g. "A's `endBlock` equals B's `startBlock`". A failed constraint
aborts the fold.

### Conditional template signature

`nPublicsAgg` is always the trailing param; `nFreeInputs` precedes it
only when `n_free > 0`:

- `n_free == 0` → `template AggregatePublics(nPublicsAgg)`, called as
  `AggregatePublics(nPublicsAgg)(ziskPublicsA, ziskPublicsB)`.
- `n_free > 0` → `template AggregatePublics(nFreeInputs, nPublicsAgg)`,
  called with `aggFreeA` / `aggFreeB` (the muxed free values) too.

The generator asserts arity matches at build time.

### Required template body (n_free > 0 example)

```circom
template AggregatePublics(nFreeInputs, nPublicsAgg) {
    signal output aggregated_publics[nPublicsAgg];
    signal input a_publics[ZISK_PUBLICS()];
    signal input b_publics[ZISK_PUBLICS()];
    signal input free_inputs_a[nFreeInputs];
    signal input free_inputs_b[nFreeInputs];

    // ... your stitching constraints and combination logic ...
}
```

`ZISK_PUBLICS()` is defined by the scaffolding above the injected body, so the
input width is not a template parameter; the output width `nPublicsAgg`
is passed in by the generator. Inputs bind positionally to the aggregator's
call in the order listed. Every slot of the `nPublicsAgg`-wide
`aggregated_publics` must be driven (the tail beyond it is generator-owned).

An inherit-from-A example lives at
[tests/fixtures/aggregate_publics.circom](../tests/fixtures/aggregate_publics.circom);
a minimal chain-fold (stitch constraint only, `n_free = 0`, `n-publics-agg = 2`) at
[test-artifacts/programs/aggregations/circuits/aggregate_publics_simple.circom](../../test-artifacts/programs/aggregations/circuits/aggregate_publics_simple.circom);
and a richer chain-fold (stitch + a NormalizePublics digest check, `n_free = 1`) at
[test-artifacts/programs/aggregations/circuits/aggregate_publics.circom](../../test-artifacts/programs/aggregations/circuits/aggregate_publics.circom).

---

## 7. Output programVK

The output `programVK` becomes the next fold level's input `programVK`.
There are four cases driven by `isFinalA` and `isFinalB`:

| A type | B type | Output `programVK` | Why |
|---|---|---|---|
| leaf | leaf | `rootCRecurserAgg` | First fold of this chain — stamp the aggregator's own identity |
| leaf | agg | B's `programVK` | B's chain is committed; A's leaf is absorbed into it |
| agg | leaf | A's `programVK` | A's chain dominates |
| agg | agg | shared VK | Both chains committed; §8 forces them to match |

In Circom, three mutually-exclusive selectors sum to 1:

```circom
signal selectAgg <== isFinalA * isFinalB;          // both leaves     → rootCRecurserAgg
signal selectAVK <== 1 - isFinalA;                // A aggregated    → A.programVK
signal selectBVK <== isFinalA * (1 - isFinalB);   // A leaf, B agg   → B.programVK

for (var i = 0; i < 4; i++) {
    // conceptual; generated form uses aggTerm/aTerm/bTerm intermediates
    // (circom 2.1 forbids the selector*value products inside one `<==`)
    aggregatedPublics[VK_BASE + i] <==
        selectAgg * rootCRecurserAgg[i]
      + selectAVK * programVK_A[i]
      + selectBVK * programVK_B[i];
}
```

For any `(isFinalA, isFinalB) ∈ {0,1}²`, the selectors sum to 1
algebraically.

---

## 8. Immutability check (both-aggregated VK match)

Once a chain commits to a `programVK`, that VK propagates upward unchanged.
When both inputs are aggregated (`isFinal = 0` on both sides), their
`programVK`s must match:

```circom
signal bothAggregated <== (1 - isFinalA) * (1 - isFinalB);
for (var i = 0; i < 4; i++) {
    bothAggregated * (programVK_A[i] - programVK_B[i]) === 0;
}
```

`bothAggregated` is 1 iff both inputs are aggregated. When it's 0 the
constraint is vacuously satisfied. When it's 1, element-wise equality
is enforced. This check uses direct limb equality inline; it is distinct
from the optional allow-list's `IsEqualVK` helper (§3a), which is only
emitted when a `programs` list is configured.

---

## 9. Recursive invariant

> From level 1 onward, every proof in a chain carries the same `programVK` —
> the `rootCRecurserAgg` of that chain's level-1 fold.

```
   Level 0 (leaves)
       is_vadcop_final_proof = 1   (emitted by the vadcop_final circuit)
       programVK = vadcop_final VK
            │
            ▼  fold (both leaves → §7 row 1)
   Level 1
       is_vadcop_final_proof = 0   (forced by aggregator)
       programVK = rootCRecurserAgg_lvl1
            │
            ▼  fold (at least one input has isFinal=0 → §7 rows 2/3/4)
   Level 2+
       is_vadcop_final_proof = 0   (propagated)
       programVK = rootCRecurserAgg_lvl1   (locked by §8)
```

Why this holds:

- **Level 1.** Both inputs are leaves (`isFinal=1`), so the output is
  `rootCRecurserAgg_lvl1` by §7's first row, and `is_vadcop_final_proof`
  output is forced to 0.
- **Level k ≥ 2.** At least one input is aggregated, so §7 propagates an
  aggregated side's `programVK`. By §8, both aggregated sides must match.
  By induction the propagated value is `rootCRecurserAgg_lvl1`.

---

## 10. `recurser_id`

The `recurser_id` is a content-addressed blake3 hash of:

```
hash(zisk_vk,
     program_vks[]  (the optional leaf allow-list, in order; empty when VK-agnostic),
     optional_normalize { blake3(body) },
     aggregate { blake3(body) },
     n_free)
```

The recurser is identified by its circuits (normalize body + aggregate body +
the shared free-input count `n_free`), the shared vadcop_final VK, and its
optional leaf allow-list (§3a). Because `program_vks` is committed, two
otherwise-identical setups that differ only in their allow-list resolve to
*different* ids and separate on-disk directories; the empty allow-list is the
VK-agnostic default. Order within the allow-list is significant. Identical
inputs always resolve to the same id; any change produces a fresh one. The id
is computed automatically and logged at startup; there is no manual override.

---

## 11. Failure modes

| Stage | Triggers when… |
|---|---|
| Boolean check on `isFinal` | malicious witness sets `isFinalA/B` non-binary |
| STARK verify (vA, vB) | malformed witness or wrong VK; mismatched `rootC` → FRI / Merkle checks reject |
| AggregatePublics (§6) | stitching constraint broken |
| Immutability check (§8) | folding two aggregated proofs from different chains |

Every check is in-circuit, so a passing proof is sound. The boolean
check on `isFinal` is the first gate — it runs before rootC selection.

---

## 12. Usage

The definition is authored once (a TOML next to the guest programs) and
consumed twice: by `build_program` at host-build time (for the SDK path)
and by the `cargo-zisk setup --aggregation` CLI at setup time.

### Prerequisites

The setup pipeline reads `provingKey/<name>/vadcop_final/` (verkey,
starkinfo, verifierinfo) from the *setup* directory. That folder is produced
by ZisK's `final` setup stage and must exist before running aggregator
setup. The recurser writes its own artifacts to a *separate* output
directory — input and output paths must differ.

Output layout (under the output directory, `~/.zisk/recurser` via the CLI/SDK):

```
provingKey/recurser/<recurser-id>/    recurser_aggregator.{dat,exec} + witness library
circom/                                recurser_aggregator.circom + vadcop_final stark verifier
build/                                 recurser_aggregator.{r1cs,fixed.bin,...}
pil/                                   recurser_aggregator.pil
```

The `<recurser-id>` segment lets a single output directory hold multiple
coexisting setups (different template bodies, different free-input counts).

> ⚠ **Domain-size constraint.** The recurser-aggregator's own STARK
> `n_bits` must equal `vadcop_final.starkStruct.nBits`. The setup checks
> this after `plonk2pil` and bails with a message. If it fires, shrink
> the recurser circuit (simpler normalize or aggregate body, fewer free
> inputs) or rebuild `vadcop_final` with a larger `nBits`.

### The definition (build-time)

An aggregation program is defined next to the guest programs and built by
the same `cargo build`. `build_program` in the host's build.rs discovers
`programs/aggregations/<name>.toml`, validates it, and generates the
builder expression behind [`load_aggregation_program!`].

All keys are flat (kebab-case); there is no `[normalize]` table. Unknown keys
are rejected.

```toml
# programs/aggregations/chain.toml — circuits beside it

# Required: the AggregatePublics circom body.
aggregate-publics = "circuits/aggregate_publics.circom"

# Required: number of publics slots the aggregation populates. AggregatePublics
# outputs an `n-publics-agg`-wide array; the scaffolding zero-fills the
# rest of the 64-slot buffer (§6). Must be in 1..=64.
n-publics-agg = 6

# Optional: single free-value width per side (default 0). Both NormalizePublics
# and AggregatePublics must declare the matching arity (§5, §6).
free-inputs = 1

# Optional: single normalize circom body, applied to all leaves. Defines
# `template NormalizePublics([nFreeInputs])`. Omit the key to skip.
normalize-publics = "circuits/normalize.circom"

# Optional: leaf allow-list (§3a). Guest program names, resolved against the
# guest workspace to derive their VKs. Absent/empty = VK-agnostic. Order is
# significant and is committed into recurser_id.
programs = ["chain_segment"]
```

`programs = [...]` doubles as the build step's guest list (to produce ELFs and
derive each member's `zisk_vk`) **and** the leaf allow-list. It IS committed
into `recurser_id` and IS emitted as the in-circuit membership check (§3a). Omit
it (or leave it empty) for the VK-agnostic default.

### The setup (machine-time)

```text
cargo-zisk setup --aggregation programs/aggregations/chain.toml
```

Reads the vadcop_final setup from `~/.zisk` and writes recurser artifacts
to `~/.zisk/recurser/<recurser-id>`.

| Flag | Required | Description |
|---|---|---|
| `--aggregation` | yes | The definition TOML under `<programs>/aggregations/` |
| `--proving-key` | no | Path to a precomputed proving key |

On the `--aggregation` path there is no profile flag: allow-list guest ELFs are
auto-detected (release profile first, then debug), so either build satisfies the
setup. `--release` only governs ELF resolution on the non-aggregation setup path.

### Rust API (SDK)

```rust
use zisk_sdk::{load_aggregation_program, AggregationProgram, ProofExt};

static AGG: AggregationProgram = load_aggregation_program!("chain");

client.setup(&AGG).run()?.await?;

// Leaves carry their free inputs; aggregated proofs are plain refs.
let ab = client
    .aggregate_proofs(&AGG, pa.with_free_inputs(vec![4]), pb.with_free_inputs(vec![4]))
    .run()?
    .await?;
let root = client.aggregate_proofs(&AGG, &ab, &cd).run()?.await?;
```

For dynamic composition without the build pipeline, construct an
[`AggregationProgram`](../../sdk/src/recurser.rs)
directly, or go lower with
[`run_setup_recurser_aggregator`](../src/setup/command.rs) /
[`gen_recurser`](../src/templates.rs).
