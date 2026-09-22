# ZisK Sail model

A formal description of the ZisK instruction set in [Sail](https://github.com/rems-project/sail),
the ISA specification language used for the official RISC-V model
([`sail-riscv`](https://github.com/riscv/sail-riscv)).

The point of writing the ISA in Sail is that one description generates
several things that today are written by hand and can drift apart:

```
                     ┌─→ Lean 4 definitions   (for proofs about the AIRs)
  model/*.sail ────→ ├─→ C emulator           (an independent reference to
                     │                          differential-test ziskemu against)
                     └─→ Rocq / Isabelle      (if ever wanted)
```

## Status

**Early.** 4 of 127 opcodes are modeled — `copyb`, `add`, `eq`, `ltu`, which
are the four the `ziskasm` doubler example uses. The step function, the
instruction record, and the source/store/jump machinery are complete.

The memory model and the register file are deliberately **not** modeled yet:
`rX`, `wX`, `read_mem` and `write_mem` are declared without definitions. That
keeps the instruction semantics precise and typechecked while being honest
about what is still missing. Filling them in is the next milestone.

## Layout

| Path | What it is |
|------|-----------|
| `model/zisk_prelude.sail` | Bitvector comparison operators and `zero_extend`/`zeros` wrappers. Sail's standard library leaves these to each model; `sail-riscv` defines the same set. |
| `model/zisk_types.sail` | Architectural state (`PC`, `Zc`, `Zflag`, `Step`) and the source/store enums. |
| `model/zisk_ops.sail` | One `union clause` + one `function clause` per opcode: `(c, flag) = op(a, b)`. Mirrors `core/src/zisk_ops.rs`. |
| `model/zisk_inst.sail` | The `zisk_inst` record, mirroring `ZiskInst` in `core/src/zisk_inst.rs`. |
| `model/zisk_step.sail` | The single execution step, mirroring `Emu::step` in `emulator/src/emu.rs`. |
| `check_ops.py` | Guard: fails if the model names an opcode that no longer exists in Rust. |
| `lean/` | A lake package that typechecks both generated Lean trees — one library points at `build/lean/out` (this model), the other at `build/pil` (the AIR constraints) — plus `lean/Proofs/`, the hand-written proofs about them. A proof relating the two belongs here. |

## Building

Needs Sail 0.19+ (`opam install sail`). This section is the per-target
reference; for the whole path from a fresh clone, with the output each command
produces, see [Verifying ZisK from a fresh clone](#verifying-zisk-from-a-fresh-clone).

```sh
make check   # typecheck the model
make lean    # generate Lean definitions into build/lean/out
make ops     # check opcode names against core/src/zisk_ops.rs
make pil     # generate Lean definitions of the AIR constraints into build/pil
```

Two more targets typecheck the generated Lean, through the lake package in
`lean/`. They need a Lean toolchain — install
[elan](https://lean-lang.org/install/):

```sh
make pil-build    # elaborate the AIR constraints  (all 54 AIRs build, ~85s)
make proofs       # check lean/Proofs against them
make lean-build   # elaborate the model            (9 of 12 modules build)
```

`make lean-build` does not pass yet, and the reason is the gap below: `rX`,
`wX`, `read_mem`, `write_mem` and `rom_fetch` are declared without definitions,
so Sail emits references to Lean identifiers that do not exist and
`Out.ZiskStep` fails to elaborate with 9 `unknown identifier` errors. The other
nine modules — the types, the opcodes, the prelude — are fine.

`make pil` is the other half of step 5 below: it reads the compiled
`pil/zisk.pilout` and emits the Main AIR's constraints as Lean, so the proof
has a machine-generated definition on both sides instead of a hand-transcribed
one. It needs `pil/zisk.pilout`, which is a build artifact — see
[`tools/pilout-constraints/README.md`](../tools/pilout-constraints/README.md).

`MODEL` in the Makefile is **order-sensitive**: Sail has no module system, so
definitions must precede their use. Prelude first, step last.

## Verifying ZisK from a fresh clone

> **Disclaimer — none of this verifies ZisK yet.** What the run below exercises
> is the *scaffolding* of a verification: the ISA model typechecks, both sides
> of the intended proof are generated from what ZisK actually ships, the AIR
> constraints elaborate as Lean propositions, and proofs about them go through.
> The theorem that would earn the phrase "formally verified" — that the model's
> `Out.Functions.zisk_step` implies the AIR's `Pil.Zisk.Main.holds` — does not
> exist, and cannot be stated until the model side elaborates, which is step 1
> of [Next steps](#next-steps). One of the commands below is *expected to
> fail*, and it is the one that measures how far away that is. Delete this
> disclaimer when step 5 lands.

Nothing below is assumed already built, and every figure quoted is what the
command printed against the pilout this branch was last regenerated on: 54
AIRs, 4066 constraints.

### What you need

| Tool | For | Install |
|------|-----|---------|
| Rust | the workspace and the constraint extractor | [rustup](https://rustup.rs); `rust-toolchain.toml` pins the version |
| Node + npm | the pil2 compiler, pulled at the version proofman pins | any recent LTS (v24 here) |
| Python 3 | `check_ops.py` | any 3.x |
| Sail 0.19+ | the ISA model | `opam install sail` |
| elan | Lean 4 | [lean-lang.org/install](https://lean-lang.org/install/). No `elan default` needed: `lean/lean-toolchain` picks the toolchain and elan fetches it on the first `lake build`. |

### The run

**0. Clone, and check out this branch.** The model lives on `feature/sail`
until it merges.

```sh
git clone git@github.com:0xPolygonHermez/zisk.git && cd zisk
git checkout feature/sail
```

**1. Compile the PIL.** Everything on the constraint side reads
`pil/zisk.pilout`, a gitignored build artifact, so it has to be built once.
This is by far the longest step; all the rest together take under two minutes.

```sh
tools/test-env/setup_build.sh --compile-pil
```

It generates the fixed data first, and ends with:

```
==> compile-pil
==> pil-helpers (regenerating pil/src/pil_helpers/traces.rs)
done. pil/zisk.pilout and pil/src/pil_helpers/ regenerated.
```

The remaining commands run from `sail/`.

**2. Typecheck the model.** Silent on success — Sail prints nothing when it has
nothing to say.

```sh
make check
```

**3. Check the opcode names against Rust.**

```sh
make ops
```

```
Rust ops: 127   Sail clauses: 4   modeled: 4/127 (3%)

UNMODELED -- 123 Rust ops with no Sail clause:
  add256, add_u_w, add_w, and, andn, arith256, ...

OK: no stale opcodes.
```

The UNMODELED list is informational and is meant to be long today; the last
line is the one that matters. A **STALE** entry is the real failure — it means
the model describes a machine ZisK no longer ships.

**4. Generate Lean from the model.** Writes twelve modules into
`build/lean/out/`.

```sh
make lean
```

**5. Generate Lean from the AIR constraints.** `--all` takes every AIR; the
default, `--air Main`, is all the proofs in step 7 need.

```sh
make pil PIL_AIR=--all
```

```
wrote build/pil/Pil/Main.lean (609 constraints)
  6764 expressions reachable of 9488, 596 named intermediates, 181 witness columns
...
wrote build/pil/Pil.lean
```

55 files under `build/pil/Pil/`, one per AIR plus the prelude, in under a
second.

**6. Elaborate the constraints.** The first run also fetches the Lean
toolchain.

```sh
make pil-build
```

```
✔ [57/58] Built Pil
Build completed successfully.
```

About 86 s from cold, seconds incrementally. Each of the 4066 constraints is
now a Lean `Prop` that Lean has checked is well formed: the columns it names
exist, and its indices are within the extents the pilout declares.

**7. Check the proofs.**

```sh
make proofs
```

```
✔ [5/6] Built Proofs
Build completed successfully.
```

Under a second. [`lean/Proofs/Main.lean`](lean/Proofs/Main.lean) holds five
theorems proved from the generated Main AIR and nothing else; it also records
where the abstraction stops.

**8. Elaborate the model — this one is expected to fail.**

```sh
make lean-build
```

```
✖ [10/12] Building Out.ZiskStep
error: ../build/lean/out/Out/ZiskStep.lean:23:20: unknown identifier 'rX'
... 9 errors ...
make: *** [Makefile:82: lean-build] Error 1
```

Nine errors, one per use of the five functions the model declares without
defining (`rX`, `wX`, `read_mem`, `write_mem`, `rom_fetch`). The other nine
modules do elaborate. This is exactly the gap described in step 1 of
[Next steps](#next-steps), and it is why the correspondence theorem cannot yet
be stated. When this command passes, the disclaimer above is a step closer to
deletable.

Optionally, the extractor's own tests: `cargo test -p zisk-pilout-constraints`,
13 tests.

### What a clean run establishes

- The ISA model is well typed, and every opcode it names still exists in
  `core/src/zisk_ops.rs`.
- The 4066 constraints of the pilout ZisK ships are well-formed propositions
  over a commutative ring, in Lean, **generated rather than transcribed**, each
  carrying the `.pil` file and line it came from.
- Those definitions are usable for proof, not merely well formed: five
  theorems about the Main AIR go through by `grind` alone.

### And what it does not

- **No correspondence.** Nothing yet relates the model to the AIR. That
  relation is the verification, and it is unwritten.
- **123 of 127 opcodes are unmodeled**, as are memory and the register file.
- **Lookup and permutation arguments are out of scope.** The extractor reads
  the AIR's algebraic constraints; the pilout's 4211 hints — where range checks
  such as the booleanity of a `bits(1)` column live — are not extracted, so a
  `bits(1)` column is not known here to be boolean.
- **The AIR is checked, not the prover.** Nothing here says the witness
  computation fills the columns these constraints describe, nor anything about
  the soundness of the STARK backend.

## Why `check_ops.py` exists

The ZisK opcode table is referenced **by name, as a string**, from places no
single compiler checks together: the `.zisk` assembly sources under `ziskasm/`,
and now this model. A rename in `zisk_ops.rs` breaks them silently.

This is not hypothetical. Renaming `blake2` → `blake2b` upstream produced a
*textually clean* merge into `feature/ziskasm` that still compiled — the
breakage only surfaced when the `zisklib` assemble test ran. `make ops` makes
that class of failure immediate:

- **STALE** — the model names an op that no longer exists in Rust. Hard error:
  the model is describing a machine we do not ship.
- **UNMODELED** — an op in Rust with no Sail clause. Informational while the
  model is being filled in; `--strict` turns it into an error once coverage is
  meant to be complete.

Run it in CI alongside the typecheck.

## Notes on modeling ZisK versus RISC-V

A few things differ from `sail-riscv` in ways worth knowing before adding
opcodes.

**There is no `encdec` mapping.** A ZisK instruction is never decoded from a
bit pattern — the transpiler and the assembler emit it already decoded, and the
ROM stores it as a record keyed by address. So each opcode needs *one* clause
here, where a RISC-V instruction needs three (`ast`, `encdec`, `execute`).

**There is no hardwired-zero register.** RISC-V models `x0` as always-zero in
the register accessor. ZisK does not need to: both `ZiskInstBuilder::src_a` and
the assembler rewrite `r0` to `SRC_IMM 0` at encode time, so `a_src = A_REG`
implies the index is non-zero.

**The source enums are split.** `a_src` and `b_src` are separate types so that
the asymmetry — `STEP` is a-only, `IND` is b-only — is a *type error* rather
than the runtime `panic!` the Rust emulator uses for the same condition.

**`ind_width` is existentially typed.** It is `{1, 2, 4, 8}`, not a plain
integer, so the legal indirect widths are enforced statically. One consequence
to watch for: each *projection* of the field yields a fresh type variable, so
two reads of `inst.ind_width` cannot be proven equal. Bind it once with a `let`
and use that binding — see `store_c` in `model/zisk_step.sail`.

## Next steps

1. **Memory and registers.** Define `read_mem`/`write_mem`/`rX`/`wX`/`rom_fetch`
   so the model becomes executable — and so it elaborates at all, since these
   five are what `make lean-build` trips over. Note that ZisK registers are
   memory-mapped at `SYS_ADDR` but the first 32 live in the main trace, and
   indices 32–63 are "virtual registers" backed by memory — that distinction
   needs a decision.

   The plumbing is already there on the Lean side: `SequentialState` in the
   generated `Out/Sail/Sail.lean` carries `mem : Std.HashMap Nat (BitVec 8)`,
   and `$include <concurrency_interface/emulator_memory.sail>` declares the
   `read_mem#`/`write_mem#` externs that reach it. So the work is the modeling
   decisions, not the memory representation. `rom_fetch` is the odd one out: it
   returns a `zisk_inst` record, not bytes, so it needs a ROM representation of
   its own.
2. **Run the doubler.** With memory in place, `sail -c` produces an emulator
   that can execute `ziskasm/examples/doubler-min`. That is the first real
   cross-check against `ziskemu`.
3. **Fill in the opcode table**, tracked by `make ops`.
4. **An `assembly` mapping.** Sail mappings are bidirectional, so one
   definition yields both an assembler and a disassembler — which would
   subsume the round-trip test listed as a TODO in `ziskasm/README.md`
   (currently `src/assembler.rs` and `ZiskInst::to_zisk_asm` are two
   hand-written implementations of the same relation).
5. **Lean proofs** relating the two generated definitions: `Out.Functions.zisk_step`
   from this model, and `Pil.Zisk.Main.holds` from
   [`pilout-constraints`](../tools/pilout-constraints/README.md), which reads
   the constraints out of the compiled pilout. Neither side is transcribed by
   hand, so neither can drift from what ZisK ships without the generator
   noticing. The constraint side already elaborates (`make pil-build`); what is
   still missing is the model side (step 1 above) and a mapping from a
   `zisk_inst` plus machine state into a `Pil.Zisk.Main.Row`.

   That mapping is where the model's `ind_width`-style typing meets the AIR's
   field columns, and it is a mapping into a *slot*, not a row: Main packs four
   instructions per row, so every column is indexed (`(t i).pc 2`) and one
   `zisk_step` corresponds to one of the four. A `.pil` line such as
   `main.pil:224` therefore yields four constraints, one per slot.
