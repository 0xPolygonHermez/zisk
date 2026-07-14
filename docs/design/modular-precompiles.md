# Modular Precompiles — Design (draft)

Status: draft · Goal: add a self-contained precompile without hand-editing ZisK core.
Motivating case: PR #1152 (babyjubjub) touches ~13 files across 6 crates.

## Goals / Non-goals
1. Declare the precompile *set* declaratively, outside the executor.
2. Add precompiles from local folder(s) outside the built-in tree.
3. A `cargo-zisk-dev new-precompile` scaffold.
4. Derive circuit facts (AIR ids, trace structs) from the PIL; derive op/table ids from the precompile's own PIL consts.

**Non-goals:** runtime `.so` loading (single global pilout + statically-generated trace rows + order-assigned AIR ids ⇒ rebuild required); native ASM-emulator support for user precompiles (Tier 2 — Rust emulator only, see below).

## Constraints that shape the workflow
- **The precompile set = proof-system identity.** A new AIR flows into `vadcop_final`, whose root is hardcoded in `zisk-contracts/ZiskVerifier.sol`. Changing the set (even removing one) changes the vk, invalidates existing proofs, and needs setup regen (~4–12h) + verifier redeploy. ⇒ **canonical build** (fixed set, distributed setup, deployed verifier) vs **custom build** (user set → self-hosted setup + verifier).
- **Two emulators.** Rust (`emulator/src/emu.rs`, function-pointer dispatch) is codegen-friendly. ASM (`core/src/zisk_rom_2_asm.rs` + C shim `emulator-asm/src/emu.c`) is ~80 hand-written lines/precompile at 4 hardcoded levels. **Tier 1** = built-in (Rust+ASM). **Tier 2** = user (Rust only); `sync` auto-emits the ASM `panic!` stub (as babyjubjub did).
- **The pilout drops named scalars.** Its `SymbolType` covers only circuit objects; `const int OP_… = 0xEE` / `…_TABLE_ID = 5003` are compile-time **inlined** and unrecoverable by name. The pilout *does* yield AIR names → `*_AIR_IDS`, `NUM_ROWS`, `PILOUT_HASH`, trace-row structs (already extracted by pil-helpers).
- **PIL composition today:** one `airgroup Zisk {}` block holds all instantiations; `require`s and `const`s are top-level; precompile PILs declare only `airtemplate`s. So a precompile contributes to **two** insertion points, not one (see step 6).

### Source-of-truth split
| Scalar | Home | Consumed by |
|---|---|---|
| Table id (5003) | precompile PIL (`precompile.pil`) | PIL only — no Rust seam |
| Op id (0xEE) | precompile PIL (`precompile.pil`), single-sourced | PIL + Rust `define_ops!` + bus discriminant |
| Syscall id, cost, in/out sizes, op_type | manifest (`zisk-precompile.toml`) | Rust/host only |
| AIR ids, NUM_ROWS, row structs | full-`zisk.pil` pilout | generated `traces.rs` (already automated) |

---

# Workflow

## A. Author writes a self-contained precompile

**1 — Scaffold.** `cargo-zisk-dev new-precompile babyjubjub` emits the crate skeleton:
```
precompiles/babyjubjub/          (or any folder, in- or out-of-tree)
  zisk-precompile.toml           # manifest (Rust/host scalars + wiring)
  pil/precompile.pil             # entry: consts + require(s) + airgroup Zisk { instances }
  pil/babyjubjub.pil             # the AIR (airtemplate BabyJubJub, BabyJubJubLtTable)
  src/lib.rs                     # SM via existing zisk_precompile! macro
  src/helpers.rs                 # pure arithmetic fn (emulator + witness gen)
```

**2 — Fill in the PIL and helper.** Write the AIR in `babyjubjub.pil` and the arithmetic in `helpers.rs`. `precompile.pil` is the composition entry — it owns the ids, pulls in the AIR, and contributes the precompile's instantiations to the Zisk airgroup:
```
// pil/precompile.pil
const int OP_BABYJUBJUB_ADD      = 0xEE;
const int BABYJUBJUB_LT_TABLE_ID = 5003;

require "babyjubjub/pil/babyjubjub.pil";   // airtemplate BabyJubJub, BabyJubJubLtTable

airgroup Zisk {                            // merged into the single Zisk airgroup
    BabyJubJub(N: 2**18);
    virtual BabyJubJubLtTable();
}
```

**3 — Declare the manifest** (`zisk-precompile.toml`) — see [example](#example-manifest--babyjubjub) below. Rust/host scalars are authoritative here; op/table ids are mirrored only for a drift check.

**4 — Enable it** in the workspace registry `zisk-precompiles.toml` — see [example](#registry--zisk-precompilestoml). Order is significant (drives AIR ids → vk).

## B. `cargo-zisk-dev sync` generates every seam

**5 — Parse & validate.** Read all manifests; line-parse each `precompile.pil` for its `const int`s; assert PIL↔manifest agreement; error on any duplicate opcode / syscall id / table id (**explicit ids, fail on collision**).

**6 — Compose the PIL.** The author's `precompile.pil` is self-contained (consts + `require`(s) + an `airgroup Zisk {}` block). `sync` folds it into the build's `zisk.pil` — the author's file is identical either way:

- **Mode A — current (no pil2 change):** `sync` parses `precompile.pil`, lifts its consts/requires to top level and splices its `airgroup Zisk {}` members into the one real block.
- *Mode B — deferred:* once pil2 gains partial-airgroup support (merge multiple `airgroup Zisk {}` blocks), `sync` just injects one top-level `require`. Not modifying pil2 now.

Either way, order = registry order ⇒ AIR ids deterministic.

**7 — Generate the Rust/guest seams** (the 13 hand-edits → generated files; the 6 core enum files become `include!` of a generated file):

| Seam | Generated from |
|---|---|
| `define_ops!` row + `opc_*` fn (clone of `opc_secp256k1_add`) | manifest + `*_ids.pil` |
| `ZiskOperationType` + `*_OP_TYPE_ID`, cost, `zisk_rom_2_asm` **panic stub** | manifest |
| `CSR_PRECOMPILED` + syscall const | manifest |
| `ExtOperationData` (generic `Precompiled` payload — no per-precompile code; see Phase 0.1) | — |
| `register_precompiles!` + `air_classifier` | registry |
| guest `syscalls/<name>.rs` (+ optional hints branch) | manifest `[guest]` |

**8 — Compile & extract circuit facts.** `compile-pil` (`zisk.pil` → `zisk.pilout`) then regenerate `pil/src/pil_helpers/traces.rs` (existing path) → real `*_AIR_IDS`, `NUM_ROWS`, `PILOUT_HASH`, row structs.

## C. Build & prove

**9 — Setup (custom build only).** Because the AIR set changed, regenerate the proving/verifying key and self-deploy the verifier. `sync` must **loudly** warn: vk changed, existing proofs invalid. (Canonical builds skip this — fixed set, distributed setup.)

**10 — Prove.** Tier-2 precompiles run under the Rust emulator (`--emulator`); the ASM path panics with a clear message.

---

# Reference

## Example manifest — babyjubjub
`zisk-precompile.toml`. Rust/host scalars authoritative; op/table id mirrored for a drift guard; AIR ids **not** declared (derived in step 8).
```toml
[precompile]
name = "BabyJubJub"          # type-name stem: BabyJubJubSM/*Manager/*Trace, AIR name, BABY_JUB_JUB_AIR_IDS
                             # op_type (ZiskOperationType + *_OP_TYPE_ID) defaults to `name`;
                             # set explicitly only when they differ — keccakf→"Keccak",
                             # sha256f→"Sha256", big_int→"BigInt".

[precompile.op]
name        = "babyjubjub_add"   # ZiskOp::BabyJubJubAdd, define_ops! string
opcode      = 0xEE               # MUST equal OP_BABYJUBJUB_ADD in babyjubjub_ids.pil
syscall_id  = 0x81C              # definitions/src/syscall.rs (Rust-only)
cost        = "BABYJUBJUB_COST"  # 1424
input_size  = 144                # bytes read from memory (indirection ptrs + operands): 2*8 + 2*64.
                                 #   marks op as precompiled + sizes mem-trace, AND derives the
                                 #   operation-bus payload (no separate [bus] section needed):
                                 #   OPERATION_BUS_*_DATA_SIZE(words) = PRECOMPILED_BUS_DATA_SIZE(5) + input_size/8 = 23.
output_size = 64                 # bytes written back to memory (result point x3,y3)

[precompile.helper]              # pure-math fn in precompiles-helpers, called by two GENERATED sites:
fn = "babyjubjub_add"            #   core opc_* (emulation) + guest wrapper host branch (#[cfg(not(zisk_guest))]).
                                 #   fn(p1: &[u64;8], p2: &[u64;8], p3: &mut [u64;8]).
                                 #   NOTE: the SM's witness math is separate (its own executor emitting trace cols).

[precompile.guest]               # generates ziskos/entrypoint/src/syscalls/babyjubjub.rs
params = [
  { name = "p1", type = "point256", mutable = true  },  # result overwrites p1
  { name = "p2", type = "point256", mutable = false },
]
hints = true                     # emit #[cfg(feature="hints")] output re-emit branch

[precompile.pil]
entry = "babyjubjub/pil/precompile.pil"   # consts + require(s) + airgroup Zisk { instances }
trace = "BabyJubJubTrace"                 # main AIR → BABY_JUB_JUB_AIR_IDS (from generated traces.rs)
```

## Registry — zisk-precompiles.toml
```toml
[precompiles]
# A bare name resolves to the default dir `precompiles/<name>`; an item with a
# path points at that folder (in- or out-of-tree) and takes its name from that
# folder's zisk-precompile.toml.
# ORDER IS SIGNIFICANT: drives zisk.pil instantiation order → AIR ids → vk.
enabled = [
  "keccakf",                       # KECCAK_OP_TYPE_ID, rank_assign = true
  "sha256f",                       # SHA256_OP_TYPE_ID
  "poseidon",                      # POSEIDON_OP_TYPE_ID
  "blake2",                        # BLAKE2_OP_TYPE_ID
  "arith_eq",                      # ARITH_EQ_OP_TYPE_ID (arith256, secp256*, bn254*)
  "arith_eq_384",                  # ARITH_EQ_384_OP_TYPE_ID (bls12-381)
  "big_int",                       # BIG_INT_OP_TYPE_ID (Add256)
  "../my-precompiles/babyjubjub",  # explicit folder (PR #1152)
]
```

## Rust codegen & crate wiring
Mechanism: **committed generated files + `include!`** (mirrors `traces.rs`) — deterministic, reviewable diffs, no compile-time magic. `sync` regenerates them.

**Self-containment:** each precompile is its own crate(s) and declares its **own** `[dependencies]` (`ark-*`, `num-bigint`, …). Nothing is hand-added to shared crates.

**Two Rust homes per precompile:**
- **math (helper)** — pure result fn (own deps). Consumed by emulation (`core` opc_*) and the guest wrapper's host build. **Must not depend on `core`** (see cycle rule).
- **SM** — the `zisk_precompile!` machinery *plus its own richer witness math* (executor emitting trace columns — separate from the helper). May depend on `core`/`zisk_pil`/`helpers`; only `executor` depends on it.

**Cycle rule:** the spine is `core → precompiles-helpers`. Cargo deps are whole-crate, so if `helpers` depends on a crate that (transitively) depends on `core`, you get `core → helpers → <crate> → core`. Therefore the **math crate stays leaf** (own deps only, never `core`), and `precompiles-helpers` becomes a **thin generated aggregator** that path-depends on each enabled precompile's math and re-exports it — so `core` keeps calling `precompiles_helpers::<fn>` unchanged. (Built-in math can stay in `helpers` during transition; new precompiles bring their own.)

**Generated files (committed, `include!`d) — pure data, no new deps on the precompile:**
| Generated file | `include!`d into |
|---|---|
| `define_ops!` rows + `opc_*` fns (load per `[bus]` layout → call `precompiles_helpers::<fn>` → write) | `core/src/zisk_ops.rs` |
| `ZiskOperationType` variants + `*_OP_TYPE_ID`, costs, `zisk_rom_2_asm` panic arms | `core` |
| `ExtOperationData` variants + DATA_SIZE + `TryFrom` arms | `common` |
| syscall consts | `definitions` |
| `CSR_PRECOMPILED` table | `transpilers/riscv` |
| `register_precompiles!` list + `use` imports | `executor` |
| guest syscall wrappers | `ziskos/entrypoint` |

**Cargo wiring (sync-managed, between markers):**
- workspace `Cargo.toml` `members` += precompile folder(s)
- `precompiles-helpers` `[dependencies]` += each enabled precompile's **math** crate (path) + generated re-export module
- `executor` `[dependencies]` += each enabled precompile's **SM** crate

**Build order:** `sync` (parse manifests + `precompile.pil` consts → generate `include!` files → edit Cargo/PIL markers) → `cargo build`. No hand-edits to `core`/`common`/`definitions`.

## Phasing (incremental)
0. **No-op codegen of current built-ins** — generate today's seams into files; diff must be zero behavior change.
1. Goal 1 — registry feeds generated `register_precompiles!`.
2. Goal 3 — `new-precompile` scaffold.
3. Goal 4 — `*_ids.pil` parser + wire pilout-derived facts.
4. Goal 2 — external folders + include-path injection.

## Phase 0 — first move: Rust seams for the current precompiles, no PIL touched

Prove the manifest→codegen pipeline against today's 7 built-ins (keccakf, sha256f, poseidon, blake2, arith_eq, arith_eq_384, big_int), generating the Rust/host seams that are currently hand-written — **zero behavior change**. No `.pil`, `traces.rs`, setup, or `precompiles-helpers` restructuring, so AIR ids and the proof system are untouched. Generated `opc_*` still call the existing `precompiles_helpers::<fn>` (helpers stays as-is this phase).

**No-op oracle (invariant after every step):** `cargo build` + existing tests green, and each generated file is content-identical (post-`rustfmt`) to the hand-written seam it replaces. A CI check re-runs codegen and fails on any `git diff`.

**Steps**
1. **Manifests for the 7 built-ins.** Back-fill every field from current source (`define_ops!`, `syscall.rs`, `data_bus_operation.rs`, `register_precompiles.rs`, `zisk_ops_costs.rs`) and cross-check. Schema must handle multi-op (arith_eq owns 11 ops sharing one AIR/op_type).
2. **Codegen tool.** `cargo-zisk-dev sync --rust-only`: reads `zisk-precompiles.toml` + manifests → emits generated `include!` files. No PIL, no Cargo edits (built-ins already wired).
3. **Migrate the cleanly-separable seams** (risk order; per seam: generate → replace hand code with `include!` → build+test+diff):
   a. `register_precompiles!` (executor) — fully precompile-derived; easiest, highest-value proof.
   b. `SYSCALL_*_ID` consts (definitions) — pure block.
   c. `CSR_PRECOMPILED` array (transpilers/riscv) — pure block; **order-sensitive** (indexed by `csr - START`).
   d. guest syscall wrappers (ziskos/entrypoint) — one file per precompile.
4. **Harder seams (small refactor, still no PIL):**
   - `ExtOperationData` + DATA_SIZE + `TryFrom` (common) — do NOT generate; make it a **generic `Precompiled` payload** so no per-precompile code exists (see Phase 0.1).
   - `define_ops!` rows + `opc_*` fns (core). **Gotcha:** you cannot `include!` inside `define_ops!{…}` (macro args aren't macro-expanded). Decide first: (i) generate the *entire* invocation, sourcing base-ISA ops from a static `base-ops.toml` the generator also reads (gives an exact no-op diff), or (ii) refactor `define_ops!` to concat a base list + a generated precompile list.
   - Leave `ZiskOperationType` + `*_OP_TYPE_ID` **hand-written** (mixed with non-precompile types); codegen only references it.
5. **CI guard:** “precompile seams up to date” — re-run codegen, `git diff --exit-code`.

**Out of scope for Phase 0:** PIL composition (Mode A), `traces.rs`, helpers→aggregator, external folders, `new-precompile`, ASM tier.

**Done when:** the 7 built-ins are fully described by manifests, their Rust seams are generated with an empty behavior diff, and the Rust side of an 8th in-tree precompile (PIL still added by hand) needs only a manifest + a registry line.

## Phase 0.1 — `data_bus_operation.rs` simplification (execution plan)

**Approach chosen:** don't macro-generate the per-op boilerplate — **delete the structure that forces it.** Measurements: the `OPERATION_BUS_*_DATA_SIZE` consts have 0 external users; the only external variant match is the central `zisk_precompile!` collector macro (`state-machines/arith` matches only the generic `OperationData`); the typed `From<&OperationXData>` impls already live inside the precompile crates. And `len = PRECOMPILED + input_data.len()` equals the exact per-op `DATA_SIZE` for every precompile — so the payload carries its own length; the 30 per-op variants/consts/arms are redundant.

**End state:** `ExtOperationData` = `OperationData([D;4])` (kept) + `Precompiled(PrecompiledData<D>)` where `PrecompiledData { len, data: [D; MAX_OPERATION_DATA_SIZE] }`. Adding a precompile touches **0 lines** here (beats a macro's 1 line/op). ~1200 → ~200 lines.

**Invariant:** behavior no-op — verified by build + the arith_eq guest suite (arith256/secp256k1/bn254/secp256r1) + one end-to-end arith_eq proof (witness identical).

**Perf guard (baseline at `c1e6bc70f`, zec-reth / mainnet_25228625, 3 runs):** the bus hot path lives in the **Rust `execute` COMPUTE_MINIMAL_TRACE** = **~8163–8411 ms** (±~3%), total exec ~9.9 s, RSS 3.78 GB, steps 338,632,173. After each step, re-run `cargo-zisk execute` 3× and flag a regression only if COMPUTE_MINIMAL_TRACE exceeds the band by >3%. Note: `--asm` runs native assembly (COMPUTE_MINIMAL_TRACE ~732 ms) and bypasses `data_bus_operation.rs` — use it as a correctness cross-check (steps must match), not the perf metric.

**Pre-flight**
- P0. Exhaustive sweep: grep every `ExtOperationData`, `Operation*Data` alias, and `OPERATION_BUS_*_DATA_SIZE` across all crates **incl. tests**. Confirm the consumer surface = {central collector macro, per-precompile `From` impls, `arith_full` `OperationData`, `regular_counters` getters}. Any other specific-variant match → add to 3's site list.
- P0b. Capture baseline test output as the no-op oracle.

**Step 1 — collapse getters (pure no-op, no consumer change) — ✅ DONE**
- Added `ExtOperationData::payload(&self) -> &[D]` (one 29-arm match, `Variant(d) => d`, `&[D;N]`→`&[D]` coercion, `#[inline(always)]`). Rewrote `get_op/op_type/a/b` as `data.payload()[IDX]`. Net −87 match arms.
- Verified: `zisk-common` checks clean; rebuilt `cargo-zisk`. Rust `execute` COMPUTE_MINIMAL_TRACE 8406/8070/8419 ms (in the 8163–8411 band), `--asm` 737 ms (≈732 baseline), steps 338,632,173 on both — no regression, behavior identical.

**Step 2 — collapse `write_instruction_payload` (no-op, returns `&[u64]`) — ✅ DONE**
- Kept the 4 fixed-size hash arms (with their `debug_assert`s); replaced the ~200 lines of per-op ArithEq/ArithEq384/BigInt/Dma arms with **one guarded arm** `ArithEq | ArithEq384 | BigInt | Dma if inst.input_size > 0 => { len = PRECOMPILED + input_data.len(); … }`. The `input_size > 0` guard preserves the original `_ => OperationData` fallback for input-less family ops. Net −~185 lines.
- Verified: `zisk-common` clean; rebuilt. Rust COMPUTE_MINIMAL_TRACE over 9 runs 8130–8663 ms (mean ~8340, in band; the two high samples were compile-warmth), `--asm` 735 ms, steps 338,632,173 — no regression, behavior identical.

**Step 3 — generic payload — ✅ DONE**
- Landed: `ExtOperationData` → `OperationData([D;4])` + `Precompiled(PrecompiledData{len,[D;MAX]})`; `payload()` 2 arms; `from_instruction` + `TryFrom` collapsed to precompiled-vs-not (~600 lines gone); collector macro now uniform `<$input>::from(payload)`; each precompile's input type owns its decode — mono-op `From<&[u64]>`, and `ArithEqInput`/`ArithEq384Input` gained an op-demux `from(&[u64])`. Invocations unchanged. Per-op consts/aliases + unused macro metavars left as dead `pub` items (cleanup deferred).
- Verified: workspace builds clean; steps 338,632,173 identical (Rust + `--asm`). Perf: Step-3 COMPUTE_MINIMAL_TRACE ~8460–8660 vs a **same-machine concurrent baseline ~8587–8840** (re-measured via `git stash` because the machine had drifted +~400ms since the original 8326 baseline) → **no regression** (equal-or-slightly-faster). Lesson: isolate perf deltas against a concurrently-measured baseline, not a stale one.

_Original sub-step outline (executed as one compiler-guided pass):_
- 3a. Add `PrecompiledData<D>` + `Precompiled(..)` variant (keep old variants for now); `payload()`/`len()` for it.
- 3b. Rewrite `from_instruction` → precompiled builds `Precompiled{len,data}`, else `OperationData`; add transitional `debug_assert!(len == <old per-op DATA_SIZE>)`.
- 3c. Rewrite `TryFrom<&[D]>` → `len==4 ? OperationData : Precompiled(copy)` (optional strict op→len check).
- 3d. Central `zisk_precompile!` collector: demux by `get_op(data)` (macro has the op list) → `SubInput::from(payload)`.
- 3e. Per-precompile `*Input::from`: `&OperationXData<u64>` → `&[u64]` (bodies unchanged; `payload[..N].try_into()` if keeping elided bounds). Files: `arith_eq_input.rs` (~11) + others found in P0.
- 3f. Delete old per-op variants, type aliases, `OPERATION_BUS_*_DATA_SIZE` consts + the transitional assert. Compiler confirms no users.
- Verify after each sub-step (build); after 3f: full baseline + one real arith_eq proof.

**Type-safety restoration (done):** the interim `From<&[u64]>` lost the compile-time payload width. Restored via a `FromBusPayload` trait in `zisk_common`: each precompile keeps a fixed-size `from(&[u64; N])` (compile-time width + const-index bounds) plus a thin `from_bus_payload(&[u64])` that narrows via `try_into().expect(...)` (fail-fast on a wrong-width payload); multi-op aggregates demux by `payload[OP]` and narrow per-arm. The collector calls `<$input as FromBusPayload>::from_bus_payload(...)`, so a missing impl is a clear trait error. Verified no-op (steps 338,632,173, timings in-band).

**Cleanup (done):** removed the now-dead per-op consts/aliases (kept `DMA_ENCODED`, `DMA_MEMCMP_COUNT_BUS`, redefined `MAX_OPERATION_DATA_SIZE = PRECOMPILED + 35`); merged `write_instruction_payload`'s 4 hash arms into the unified precompiled arm; dropped the `ops = [...]` param from the `zisk_precompile!` macro (façade + explicit) and all 7 invocations, and updated the macro docs. Rebuilt clean (no warnings); steps 338,632,173 identical on Rust + `--asm`, COMPUTE_MINIMAL_TRACE in-band.

**Rollback:** steps 1,2 independently revertable; 3a–3e keep old variants so the tree compiles throughout; 3f (the irreversible delete) last, only after green.

**Done when:** no per-op precompile symbols remain in `data_bus_operation.rs`; a new precompile needs 0 edits here; all baseline tests + the arith_eq proof match pre-refactor.

## Phase 0.2 — decouple `zisk-core` (generated, not runtime)

**Framing:** the `ZiskOp` / `OpType` / `ZiskOperationType` enums are closed compile-time sets the emulator dispatches on — so core stays *compile-time* (add a precompile ⇒ regenerate + rebuild, no runtime loading). But a closed enum can be **generated**: the goal is **zero hand-written precompile content in `core/src/`** — a new precompile edits no core file; `sync` regenerates `include!`d files and adds a dep.

**Coupling today (all in core):** `zisk_ops.rs` (`OpType` enum + its `From`/`Display`/`FromStr`; the `define_ops!` rows; ~23 `opc_*/op_*/ops_*` fns), `zisk_inst.rs` (`ZiskOperationType` + `*_OP_TYPE_ID`), `zisk_ops_costs.rs` (cost consts), `zisk_rom_2_asm.rs` (47 ASM arms + externs), plus `helpers.rs`/`operations/*` DMA calls. (The `CSR_PRECOMPILED` + syscall id live in `transpilers/riscv` + `definitions`, outside core — handled by the same `sync`.)

**Generated-file map** (`include!` at item position expands whole constructs, so entire invocations/enums are pulled in):
| core/src source (kept, generic) | includes → generated file (from manifests) |
|---|---|
| `zisk_ops.rs`: `define_ops!` macro def + `precompiled_load_data`/stats infra | `ops_table.rs` = the whole `define_ops! { base-ISA rows + precompile rows }` + generated `opc_*/op_*/ops_*` fns |
| `zisk_ops.rs`: (nothing per-op) | `op_type.rs` = `OpType` enum + `From/Display/FromStr` impls |
| `zisk_inst.rs` | `op_types.rs` = `ZiskOperationType` enum + `*_OP_TYPE_ID` consts |
| `zisk_ops_costs.rs` | `costs.rs` = cost consts |
| `zisk_rom_2_asm.rs`: generic `emit_precompile_asm(ctx, code, &layout)` | `asm_dispatch.rs` = per-op arms calling the emitter (Phase B) |

**Cycle avoidance (the subtle part):** generate `opc_*` **into core** (they need core's `InstContext` + `precompiled_load_data`), calling each precompile's **leaf math crate** (`babyjubjub_math::add`). `core → leaf-math` is acyclic only because the math crate has no `core` dep. `sync` adds a path dep per enabled precompile between markers in `core/Cargo.toml`. (Putting `opc_*` in the precompile crate would cycle: `core → precompile → core`.)

**Base-ISA split:** one `define_ops!` invocation can't be half hand-written + half `include!`d, so base ISA ops become a static `base-ops` table the generator always emits. Core keeps zero op rows.

**Phasing (each step a no-op, verified against the same benchmark + steps + `--asm`):**
- **0.2a — extract-in-place (no-op):** move today's hand-written content (`define_ops!` rows, `OpType`/`ZiskOperationType`/consts/costs, `opc_*` fns) into generated `include!`d files with **zero behavior change**. Proves the codegen path against the built-ins; core source ends with only infra + macros. `cargo expand` diff empty.
- **0.2b — manifest-fed:** the generated files are produced from the precompile manifests + a `base-ops` table; `sync` wires the leaf-math path deps. A new precompile now adds only its manifest + crate — no core edit.
- **0.2c — ASM (own phase, or defer):** refactor the 47 `zisk_rom_2_asm.rs` arms into the data-parameterized `emit_precompile_asm` + generated dispatch + generated `emu.c` shims. Until then, **Rust-only tier**: `sync` emits the `panic!` arm for external precompiles (as babyjubjub did).

**Residual constraints (inherent, not coupling defects):** still compile-time (rebuild); base ISA becomes data; core's dep graph grows per precompile (leaf-math path deps); the enabled *set* still defines `OP_TYPE_ID` → proof-system identity (the vk point).

**Net:** core's *source* becomes fully precompile-agnostic; the enums stay closed and compiled-in but generated. "Fully decoupled" = generated, not runtime.

### Decision: full plugin model (Option 2) via a reserved op-type-id band
For a true plugin (add a precompile = folder + manifest, **zero core hand-edits**), `ZiskOperationType` must be generated too — but its `#[repr(u32)]` discriminants are ABI (`OP_TYPE_ID`), and the current order interleaves base + precompile, so naive append would shift base ids. Resolution:

- **Base op-types stay hand-written at low ids** (vanilla, never perturbed by precompiles).
- **Precompiles get ids from a reserved band** (e.g. `≥ 0x1000`), each **declared in its own manifest** (`op_type_id = 0x1007`) so the id is a *stable per-precompile property*, not positional — removing one never renumbers others. Same scheme for `opcode` (reserved `u8` band).
- The generator emits explicit discriminants for the band and **validates at codegen**: `op_type_id`/`opcode`/syscall-id/table-id are in-band, unique, no clash with base; valid idents; manifest complete; PIL↔manifest match; registry resolves. Codegen becomes a hard-failing validation gate.
- **Verified safe:** `OP_TYPE_ID` is used only as an opaque tag (`data[OP_TYPE] == …_OP_TYPE_ID`), never as a dense array index, so a sparse high band is fine (grep confirmed — no `op_type as usize` indexing, no `NUM_OP_TYPES` arrays).
- **Cost:** one-time vk regen on adoption (current precompile op-type ids move into the band). Inherent, since the enabled set already defines the vk.

`ZiskOp`/`define_ops!`, costs, `register_precompiles!`, and the PIL are **append-safe** (opcodes are explicit in each row; enum discriminant isn't the ABI) → those generate as no-ops (opcodes preserved). Only the op-type band move changes the vk.

## Open questions
- Can `vadcop` aggregation be made config-agnostic to avoid vk churn when the set changes?
- Add partial-airgroup support to pil2 (merge multiple `airgroup Zisk {}` blocks) to enable Mode B — check if already present; if not, scope the compiler change.
- Opcode space is `u8` and `0xe0–0xff` is nearly full — reserve a Tier-2 range, or a shared "external" opcode demuxed by syscall id?
- How does a Tier-2 (Rust-only) precompile degrade under distributed/coordinator proving?
```
