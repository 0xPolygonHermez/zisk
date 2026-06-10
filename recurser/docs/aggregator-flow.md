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
- runs three publics-handling sub-templates: `PreparePublics` (optional,
  default identity), `CheckPublics` (optional, default no-op),
  `AggregatePublics` (required) — see §5/§6/§7,
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
| **Aggregator-level** (prover-supplied) | `private_inputs[K]` | side inputs threaded into all three sub-templates (§5/§6/§7) |
| | `rootCRecurserAgg[4]` | this aggregator's *own* VK — committed at the next level |
| **Hardcoded** (baked into the circuit at setup) | `programVKs[P][4]` | the **leaf allowlist** — VKs of all registered programs |
| | `rootCVadcopFinalZisk[4]` | verification key for ZisK proofs |
| **Out** | `publics[64]` | aggregated user publics |
| | `programVK[4]` | new chain identity, per §8 (Output programVK) |

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

`K` is the number of private inputs, a CLI / Rust API parameter (default 0).
`P` is the number of registered programs. `programVKs[]` and
`rootCVadcopFinalZisk` come from CLI / external setup data.

> ⚠ **Zero-pad unused slots.** The publics array is always 64 long. If
> your app uses fewer than 64 publics, the leftover high-index slots have
> to be zero in every leaf proof — otherwise the prover can put arbitrary
> values there and they propagate through the fold tree unchecked. The
> default `CheckPublics` is a no-op (§6) and won't catch this. Either
> zero-pad in the producer circuit, or supply a custom `CheckPublics` that
> adds `a_publics[i] === 0` and `b_publics[i] === 0` for the unused range.

---

## 2. Pipeline

The aggregator runs a linear sequence of stages:

| # | Stage | What happens | Detail |
|---|---|---|---|
| 1 | **Classify** A and B independently | Each proof's `programVK` is tested against the registered-program allowlist. Match ⇒ leaf, no match ⇒ aggregated. | §3 |
| 2 | **Pick `rootC`** per proof | Leaf ⇒ `rootCVadcopFinalZisk`. Aggregated ⇒ that proof's own `programVK`. | §4 |
| 3 | **Verify both STARK proofs** | Each proof's inner STARK verifier runs with its picked `rootC` and the proof data. | §4 |
| 4 | **PreparePublics** *(optional)* | Normalisation applied only to leaf publics (aggregated publics pass through unchanged). Defaults to identity passthrough. | §5 |
| 5 | **CheckPublics** *(optional)* | Stitching constraints between A's and B's publics. Defaults to no-op (no constraints). | §6 |
| 6 | **AggregatePublics** *(required)* | User-supplied combination of the two payloads. | §7 |
| 7 | **Pick output `programVK`** | One of four cases based on each side's leaf/aggregated status. | §8 |
| 8 | **Emit combined proof** | `(publics, programVK)` — the next fold-level's input. | — |

Each row corresponds to a contiguous block in
[aggregator.circom.tera](https://github.com/0xPolygonHermez/zisk/blob/feature/recurser/recurser/templates/aggregator.circom.tera).

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
  aggregator, so `rootC` is the proof's own `programVK` — which by §10's
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

## 5. PreparePublics (optional, default identity)

A normalisation hook applied to each proof's publics *before* the
consistency check and the aggregation. It's the spot to rewrite a leaf
proof's raw publics — hash them, re-encode, derive new values from
`private_inputs`, etc. — the first time the proof enters the recursion.
Aggregated proofs were already normalised at a prior level, so they skip
this step:

| Proof type | Publics used downstream |
|---|---|
| leaf (`isRegisteredProgram = 1`) | prepared |
| aggregated (`isRegisteredProgram = 0`) | raw |

The aggregator muxes between the two automatically, so the prepare step
only fires on leaf proofs (the first time they're folded); the normalised
form then propagates unchanged through every later level.

### Default behaviour

If you don't supply anything, the recurser uses a built-in identity body
([templates/prepare_publics.circom](https://github.com/0xPolygonHermez/zisk/blob/feature/recurser/recurser/templates/prepare_publics.circom))
that copies `publics` to `recurser_publics` unchanged.

### Custom override

Supply your own body via `--prepare-publics-template <path>` on the CLI or
`CircomTemplates::prepare_publics = Some(body)` in the Rust API. Required
signature:

```circom
template PreparePublics(nPublics, nPrivateInputs) {
    signal input publics[nPublics];
    signal input private_inputs[nPrivateInputs];
    signal output recurser_publics[nPublics];

    // ... your derivation logic ...
}
```

The aggregator calls it as:

```circom
signal <preparedA>[nPublics] <==
    PreparePublics(nPublics, nPrivateInputs)(<rawA>, privateInputs);
```

(and analogously for B). `nPublics` is fixed to 64 at the `Main(…)`
instantiation; `nPrivateInputs` flows in from the CLI option. The body
can do anything Circom supports — hashes, decompositions, range checks.

---

## 6. CheckPublics (optional, default no-op)

Stitching constraints between A's and B's publics — e.g. "A's `endBlock`
equals B's `startBlock`". A failure aborts the fold, so two proofs that
aren't end-to-end can't be combined.

This is also where you constrain unused publics slots to zero (see the
zero-pad warning in §1). If your app uses 32 publics, add
`a_publics[i] === 0` and `b_publics[i] === 0` for `i = 32..64`.

### Default behaviour

If you don't supply anything, the recurser uses a built-in no-op body
([templates/check_publics.circom](https://github.com/0xPolygonHermez/zisk/blob/feature/recurser/recurser/templates/check_publics.circom)) that
emits no `===` constraints. Useful when your publics don't need any
stitching — but the no-op default also means the zero-pad rule (§1) isn't
enforced, so either zero-pad in the producer circuit or supply a custom
`CheckPublics`.

### Custom override

Supply your own body via `--check-publics-template <path>` on the CLI or
`CircomTemplates::check_publics = Some(body)` in the Rust API. Required
signature:

```circom
template CheckPublics(nPublics, nPrivateInputs) {
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];
    signal input private_inputs[nPrivateInputs];

    // ... your === constraints ...
}
```

Call site:

```circom
CheckPublics(nPublics, nPrivateInputs)(<preparedA>, <preparedB>, privateInputs);
```

`<preparedA>` and `<preparedB>` are the post-`PreparePublics` payloads from
§5. There's no output signal — the template only emits `===` constraints
that fail when the stitching is invalid. `private_inputs` is forwarded too,
so checks can constrain publics against side data.

---

## 7. AggregatePublics (required)

Combines A's and B's `publics[nPublics]` arrays into one same-size output
that the next fold level consumes. Each output slot is some function of
the matching slots in A and B.

Supply the body via `--aggregate-publics-template <path>` on the CLI or
`CircomTemplates::aggregate_publics` in the Rust API. Required signature:

```circom
template AggregatePublics(nPublics, nPrivateInputs) {
    signal output aggregated_publics[nPublics];
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];
    signal input private_inputs[nPrivateInputs];

    // ... your combination logic ...
}
```

The aggregator calls it as:

```circom
signal aggregatedPublics[nPublics] <==
    AggregatePublics(nPublics, nPrivateInputs)(<preparedA>, <preparedB>, privateInputs);
```

Every element of `aggregated_publics` must be driven by `<==` inside the
body or Circom errors. `private_inputs` is forwarded too, so the
combination logic can mix in side data.

A common pattern is a per-slot pick of A's value or B's value: e.g.
`startBlock` inherits from A and `endBlock` from B, so the combined proof
attests the segment `[A.start, B.end]`. Sums, hashes, and conditional
combinations all work — it's plain Circom.

An inherit-from-A example lives at
[tests/fixtures/aggregate_publics.circom](https://github.com/0xPolygonHermez/zisk/blob/feature/recurser/recurser/tests/fixtures/aggregate_publics.circom).

---

## 8. Output programVK

The output `programVK` becomes the next fold level's input `programVK`.
There are four cases:

| A type | B type | Output `programVK` | Why |
|---|---|---|---|
| leaf | leaf | `rootCRecurserAgg` | First fold of this chain — stamp the aggregator's own identity |
| leaf | agg | B's `programVK` | B's chain is already committed; A's leaf is absorbed into it |
| agg | leaf | A's `programVK` | Mirror of the above — A's chain dominates |
| agg | agg | shared VK | Both chains committed; §9 forces them to match |

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

## 9. Immutability check

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

## 10. Recursive invariant

> From level 1 onward, every proof in a chain carries the same `programVK` —
> the `rootCRecurserAgg` of that chain's level-1 fold.

```
   Level 0 (leaves)
       programVK = program's own VK          (∈ registered list)
            │
            ▼  fold (both leaves → §8 row 1)
   Level 1
       programVK = rootCRecurserAgg_lvl1     (∉ registered ⇒ "agg" from now on)
            │
            ▼  fold (any input is "agg" → §8 rows 2/3/4)
   Level 2
       programVK = rootCRecurserAgg_lvl1     ← inherited; locked by §9
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
  `rootCRecurserAgg_lvl1` by §8's first row.
- **Level k ≥ 2.** At least one input is aggregated, and by induction its
  `programVK` is `rootCRecurserAgg_lvl1`. §8 picks an aggregated input's
  `programVK` as the output (rows 2/3/4), so the output `programVK` is
  also `rootCRecurserAgg_lvl1`.
- **Chains can't be mixed.** Whenever both inputs at level ≥ 2 are
  aggregated, §9 forces `programVK_A == programVK_B`. Two proofs from
  different chains have different `rootCRecurserAgg_lvl1` values, so the
  equality fails and the fold is rejected.

---

## 11. Failure modes

| Stage | Triggers when… |
|---|---|
| STARK verify (vA, vB) | malformed witness or wrong VK; mismatched `rootC` ⇒ FRI / Merkle checks reject |
| CheckPublics (§6) | stitching constraint broken |
| Immutability check (§9) | folding two aggregated proofs from different chains |
| Binary check on `isRegisteredProgram` | malicious witness tries to set `isRegisteredProgram_X` non-binary |

Every check is in-circuit, so a passing proof is sound.

---

## 12. Usage

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
bodies, different `n-private-inputs`). The id is a content-addressed
blake3 hash of the circuit inputs — `program_vks`, `n_private_inputs`,
and the `prepare_publics` / `check_publics` / `aggregate_publics` template
bodies, together with the vadcop_final proving-key VK — so identical
inputs always resolve to the same id and any change produces a fresh one.
It's computed automatically and logged at startup; there's no manual
override.

The recurser doesn't nest under the source ZisK pilout name
(`provingKey/<name>/...`) because the artifacts here are
aggregator-scoped, not ZisK-program-scoped.

> ⚠ **Domain-size constraint.** The recurser-aggregator's own STARK
> `n_bits` (decided by `plonk2pil` from the generated R1CS) must equal
> `vadcop_final.starkStruct.nBits`. The aggregator's output proof has to be
> re-verifiable by the next fold level, which is the same circuit — so
> different domain sizes would panic the prover. The setup checks this
> after `plonk2pil` and bails with a message. If it fires, either shrink
> the recurser circuit (simpler `PreparePublics` / `CheckPublics` /
> `AggregatePublics`, fewer `private_inputs`) or rebuild `vadcop_final`
> with a larger `nBits`.

### CLI (`cargo-zisk`)

```text
cargo-zisk setup-recurser-aggregator \
    --program-elf <ELF> [--program-elf <ELF> ...] \
    --aggregate-publics-template <FILE.circom> \
    [--setup-dir <SETUP_DIR>] \
    [--output-dir <OUTPUT_DIR>] \
    [--prepare-publics-template <FILE.circom>] \
    [--check-publics-template <FILE.circom>] \
    [--n-private-inputs <N>] \
    [--proving-key <DIR>] \
    [--cache-dir <DIR>]
```

| Flag | Required | Description |
|---|---|---|
| `--setup-dir` | no | ZisK setup directory to read from (contains `provingKey/<name>/vadcop_final/`). Defaults to `~/.zisk` |
| `--output-dir` | no | Where to write the recurser-aggregator artifacts. Must differ from `--setup-dir`. Defaults to `./build` |
| `--recurser-id` | no | Identifier for this setup. Artifacts land under `<output-dir>/provingKey/recurser/<recurser-id>/`. Defaults to a random hex placeholder logged at startup |
| `--program-elf` | yes (1+) | Guest program ELF(s) to register as recurser leaves. Each is resolved to its program VK via `rom_merkle_setup` and baked into the allowlist. Order fixes the `programVKs[]` index — keep it stable across re-setups |
| `--aggregate-publics-template` | yes | User's `AggregatePublics` Circom body (§7) |
| `--prepare-publics-template` | no | Custom `PreparePublics` body. Omit for the built-in identity default (§5) |
| `--check-publics-template` | no | Custom `CheckPublics` body. Omit for the built-in no-op default (§6) |
| `--n-private-inputs` | no (default `0`) | Side-input count threaded into the three sub-templates |
| `--proving-key` | no | Path to the proving key used to build the `ProofCtx` rom-setup runs against. Defaults to the standard ZisK location |
| `--cache-dir` | no | rom-setup cache directory (`<elfHash>_<pkHash>_…verkey.bin` artifacts). Defaults to `~/.zisk/cache` |

VK derivation is cache-aware: if `rom_merkle_setup` finds the matching
`*.verkey.bin` already in `--cache-dir`, it just reads it (no recompute).
First-time ELFs run the ROM-merkle pass, which is cheaper than full
`program-setup` because it skips assembly generation.

The flag names above (modulo `--program-elf`) match
[`SetupRecurserAggregatorOptions`](https://github.com/0xPolygonHermez/zisk/blob/feature/recurser/recurser/src/setup/command.rs)
one-for-one. The lib API takes program VKs inline as
`Vec<[String; 4]>` — callers that already have the VKs can skip the
ELF-resolution step entirely. The subcommand lives at
[`cli/src/commands/setup_recurser_aggregator.rs`](https://github.com/0xPolygonHermez/zisk/blob/feature/recurser/cli/src/commands/setup_recurser_aggregator.rs).

### Rust API

```rust
use recurser::setup::{run_setup_recurser_aggregator, SetupRecurserAggregatorOptions};

let opts = SetupRecurserAggregatorOptions {
    setup_dir: "/path/to/zisk/setup".to_string(),            // reads provingKey/<name>/vadcop_final/
    output_dir: "/path/to/recurser/output".to_string(),      // writes recurser-aggregator artifacts here
    recurser_id: None,                                        // None ⇒ random hex placeholder
    program_vks: vec![                                        // 4 decimal-string Goldilocks limbs per program;
        ["1234".into(), "5678".into(), "9abc".into(), "def0".into()],   // typically derived from ELFs via
    ],                                                        // rom_merkle_setup in the calling code.
    n_private_inputs: 0,
    prepare_publics_template: None,            // None ⇒ built-in identity (§5)
    check_publics_template: None,              // None ⇒ built-in no-op (§6)
    aggregate_publics_template: "/path/to/aggregate_publics.circom".to_string(),  // required (§7)
};
run_setup_recurser_aggregator(&opts)?;
```

For finer-grained control — say, when embedding the generator into a
larger pipeline — call [`gen_aggregator`](https://github.com/0xPolygonHermez/zisk/blob/feature/recurser/recurser/src/templates.rs) directly
with a [`CircomTemplates`](https://github.com/0xPolygonHermez/zisk/blob/feature/recurser/recurser/src/templates.rs) value. `prepare_publics`
and `check_publics` are `Option<String>` (None means use the built-in
default); `aggregate_publics` is a required `String`.

---

## 13. Open questions

### 1. Zero-enforcement responsibility for fewer-than-64-publics apps

§1 covers the input side: if your app uses fewer than 64 publics, the
unused range `[nUsed..64)` has to be zero on the producer-side input,
either enforced in the ZisK producer or via a custom `CheckPublics`.

That leaves two more spots inside the fold:

- **`PreparePublics` output.** The body can compute arbitrary functions
  from `publics[]` into `recurser_publics[]`, so even if its inputs at
  `[nUsed..64)` are zero, the outputs may not be — e.g. when
  `PreparePublics` is a hash.
- **`AggregatePublics` output.** `aggregated_publics[]` becomes the next
  level's input `publics[]`. If the unused range isn't zero here, the next
  level's zero-pad invariant breaks immediately.

Best guess, not enforced anywhere yet: yes, the user is responsible for
zeroing the unused range in both `PreparePublics` and `AggregatePublics`.
Once we confirm, the doc should say so explicitly.

Alternative: add a setup-time `nUsedPublics` parameter (≤ 64). The
aggregator template would auto-constrain `a_sv_publics[i] === 0` and
`b_sv_publics[i] === 0` on input and auto-zero `aggregated_publics[i] <== 0`
on output for `i ∈ [nUsedPublics, 64)`, and pass `nUsedPublics` to the user
templates so they only see the meaningful range. The user never touches
the zero-pad rule.
