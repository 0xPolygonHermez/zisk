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

## What Sail is, and why the ISA is not written in Lean directly

**Sail is a language for writing down an instruction set**, developed by the
[REMS](https://www.cl.cam.ac.uk/~pes20/rems/) project at Cambridge. A Sail
model reads roughly like the pseudocode in an ISA manual — registers, memory
effects, a function per instruction — but it is typechecked rather than prose:
bitvectors carry their length in their type, and the arithmetic side conditions
that produces are discharged by an SMT solver (z3) while the model is checked.
Besides the RISC-V model linked above, there are Sail models of Arm-A and of
CHERI.

Sail is a *specification* language, not a logic: it has no theorems, no proofs,
and no tactics. What it has instead are backends. Version 0.19 emits C,
SystemVerilog, OCaml, Lean 4, Coq/Rocq, Lem (and through it Isabelle/HOL and
HOL4), SMT problems, and documentation — every one of them from the same
source text.

**Lean is where the proving happens.** `sail --lean` turns each Sail function
into a Lean definition — `source_a` below is the model's operand fetch, in the
`SailM` state monad the backend generates:

```lean
def source_a (inst : zisk_inst) : SailM (BitVec 64) := do
  match inst.a_src with
  | A_C => readReg Zc
  | A_REG => (pure (rX (Sail.BitVec.extractLsb inst.a_imm 4 0)))
  ...
```

Those definitions are one half of what this branch is building toward. The
other half is [`Pil.Zisk.Main.holds`](../tools/pilout-constraints/README.md),
the AIR constraints ZisK's prover actually enforces, generated out of the
compiled pilout. The verification is a hand-written Lean theorem relating the
two, and Lean — not Sail — is where it will live.

So the question is not Sail *or* Lean; it is whether the ISA side of that
theorem should be hand-written Lean or generated from Sail. Five reasons it is
Sail:

1. **A specification has to be validated, not just written.** A proof about the
   wrong description of ZisK is worth nothing, and no proof can tell you the
   description is wrong. Only running it can. `sail -c` compiles the model to a
   C emulator fast enough to execute real programs and be differential-tested
   against `ziskemu` — so the spec can be checked against the machine before
   anything is proved about it. Hand-written Lean gives no such check.
2. **One source, several artifacts, no drift.** That is the diagram above. The
   emulator, the Lean definitions and the documentation are all the same text
   compiled differently, so they cannot disagree. This is the same argument
   that makes the constraint side *generated* from the pilout rather than
   transcribed: the two generated definitions are what makes a proof about ZisK
   a proof about what ZisK ships.
3. **ISA-shaped types, checked by the compiler.** `ind_width` is `{1, 2, 4, 8}`
   in the model, so an illegal indirect width is a type error at compile time
   where the Rust emulator has a runtime `panic!` — and `a_src`/`b_src` being
   separate types makes the asymmetry between them unrepresentable. In Lean the
   same constraints become proof obligations to carry through every lemma.
4. **The spec is the top of the trust chain, so it has to be readable.**
   Nothing above it is verified; it is trusted because people read it and agree
   it describes ZisK. Sail is readable by a ZisK engineer or an external auditor
   who does not know Lean, which keeps that audience as wide as possible. A
   Lean specification narrows it to people fluent in Lean.
5. **ZisK is a RISC-V machine, and RISC-V is already in Sail.** The official
   model is `sail-riscv`. Sharing its language keeps the door open to comparing
   against it, or reusing parts of it, instead of reimplementing RISC-V
   semantics in Lean.

### What it costs

Three costs, since none of the above is free.

**The Sail → Lean backend joins the trusted base.** Nothing proves that the
Lean it emits means the same thing as the C it emits from the same source: the
no-drift argument above buys a common origin, not a proof that two backends
agree. The Lean backend is also the newest of them, and far less
exercised than the C or Coq output.

**The generated Lean is machine-shaped.** It is monadic and verbose, as the
snippet above shows, and it is not what anyone would write by hand to prove
things about. Proofs have to work against definitions this repository does not
control, and a backend change can move them.

**Two toolchains instead of one.** Sail through opam, Lean through elan, and a
model that has no module system — which is why `MODEL` in the Makefile is
order-sensitive.

## Status

**Early.** 4 of 127 opcodes are modeled — `copyb`, `add`, `eq`, `ltu`, which
are the four the `ziskasm` doubler example uses. The step function, the
instruction record, and the source/store/jump machinery are complete.

The register file, memory and the ROM are modeled, so every function the model
uses is defined and the generated Lean elaborates in full. No program is loaded
yet: the default ROM has no instructions, and running the model on a real
program is the next milestone.

## Layout

| Path | What it is |
|------|-----------|
| `model/zisk_prelude.sail` | Bitvector comparison operators and `zero_extend`/`zeros` wrappers. Sail's standard library leaves these to each model; `sail-riscv` defines the same set. |
| `model/zisk_types.sail` | Architectural state (`PC`, `Zc`, `Zflag`, `Step`) and the source/store enums. |
| `model/zisk_mem.sail` | The register file (`rX`, `wX` over `x1`–`x31`) and byte-addressed memory (`read_mem`, `write_mem`). See [Registers and memory](#registers-and-memory). |
| `model/zisk_ops.sail` | One `union clause` + one `function clause` per opcode: `(c, flag) = op(a, b)`. Mirrors `core/src/zisk_ops.rs`. |
| `model/zisk_inst.sail` | The `zisk_inst` record, mirroring `ZiskInst` in `core/src/zisk_inst.rs`. |
| `model/zisk_rom.sail` | The default ROM: `rom_fetch` with no instructions. A program supplies its own file, selected with `make ROM=…`. |
| `model/zisk_step.sail` | The execution step, mirroring `Emu::step` in `emulator/src/emu.rs`: `zisk_execute(inst)` runs one instruction, `zisk_step()` fetches it from the ROM first. |
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
make lean-build   # elaborate the model            (all 14 modules build, ~6s)
```

`make pil` is the other half of the correspondence proof (steps 3 and 6 of
[Next steps](#next-steps)): it reads the compiled
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
> `Out.Functions.zisk_execute` implies the AIR's `Pil.Zisk.Main.holds` — does
> not exist. Both sides now elaborate, so it can be stated, but it is not yet
> proved; that is steps 3 and 6 of [Next steps](#next-steps). Delete this
> disclaimer when step 6 lands.

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

**4. Generate Lean from the model.** Writes fourteen modules into
`build/lean/out/Out/`, plus Sail's Lean runtime under `Out/Sail/`.

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

**8. Elaborate the model.**

```sh
make lean-build
```

```
✔ [12/15] Built Out.ZiskMem
✔ [13/15] Built Out.ZiskStep
✔ [14/15] Built Out
Build completed successfully.
```

About 6 s from cold. Every function the model uses is now defined, so the
generated `Out.Functions.zisk_execute` is a complete Lean definition: one side
of the correspondence theorem, with no `sorry` and no axioms.

Optionally, the extractor's own tests: `cargo test -p zisk-pilout-constraints`,
13 tests.

### What a clean run establishes

- The ISA model is well typed, and every opcode it names still exists in
  `core/src/zisk_ops.rs`.
- The model's generated Lean elaborates, with registers, memory and the ROM
  defined, so it is a complete Lean definition of one ZisK step.
- The 4066 constraints of the pilout ZisK ships are well-formed propositions
  over a commutative ring, in Lean, **generated rather than transcribed**, each
  carrying the `.pil` file and line it came from.
- Those definitions are usable for proof, not merely well formed: five
  theorems about the Main AIR go through by `grind` alone.

### And what it does not

- **No correspondence.** Nothing yet relates the model to the AIR. That
  relation is the verification, and it is unwritten.
- **123 of 127 opcodes are unmodeled.**
- **The model has not run a program.** Nothing has checked it against
  `ziskemu` yet; that is step 2 of [Next steps](#next-steps).
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
implies the index is non-zero. `rX` and `wX` assert it rather than assume it.

**The source enums are split.** `a_src` and `b_src` are separate types so that
the asymmetry — `STEP` is a-only, `IND` is b-only — is a *type error* rather
than the runtime `panic!` the Rust emulator uses for the same condition.

**`ind_width` is existentially typed.** It is `{1, 2, 4, 8}`, not a plain
integer, so the legal indirect widths are enforced statically. One consequence
to watch for: each *projection* of the field yields a fresh type variable, so
two reads of `inst.ind_width` cannot be proven equal. Bind it once with a `let`
and use that binding — see `store_c` in `model/zisk_step.sail`.

## Registers and memory

`model/zisk_mem.sail` fixes three decisions, each taken from what the encoder
and the emulator already do rather than chosen freely.

**The register file is `x1`–`x31`.** ZisK maps registers at `REG_FIRST =
SYS_ADDR`, but an instruction never reaches the first 32 through memory. The
encoder (`ZiskInstBuilder::src_a`, `src_b`, `store` in
`core/src/zisk_inst_builder.rs`) turns register 0 into `SRC_IMM 0` or no store
at all, registers 1–31 into `SRC_REG`/`STORE_REG`, and the "virtual registers"
32–63 into `SRC_MEM`/`STORE_MEM` at `REG_FIRST + 8·i`. So registers 32–63 are
plain memory, above `REG_LAST`, and the register file is the 31 registers the
main trace holds: a `vector(32, bits(64))` whose slot 0 is never used.

**Memory is a little-endian byte map, apart from the registers.** `read_mem`
and `write_mem` go through Sail's `read_ram`/`write_ram` builtins, which
`rts.c` implements for the C emulator and the Lean runtime implements over
`SequentialState.mem`. The emulator keeps `x0`–`x31` in an array of their own,
and `Mem` `debug_assert`s that no memory access lands on an aligned register
address. The model asserts the stronger condition: no access overlaps
`[REG_FIRST, REG_LAST + 8)` at all.

**The ROM is per program.** An instruction is a record, not bytes, so
`rom_fetch` is a function from `pc` to `zisk_inst` rather than a memory read.
`model/zisk_rom.sail` is a ROM with no instructions, so the model builds with
no program at all, and a program supplies its own file (`make ROM=…`). The step
is split so that proofs need no ROM: `zisk_execute(inst)` runs one instruction,
and `zisk_step()` is `zisk_execute(rom_fetch(PC))`.

Two simplifications, both of which hold today but which the model does not
check:

- **No stack pointer.** The emulator adds `sp` to a `SRC_MEM`/`STORE_MEM`
  address when the instruction's `use_sp` bit is set. Nothing in the
  transpilers sets it, and `sp` starts at 0 and is never written, so the
  model leaves it out.
- **No memory regions.** ZisK's memory map has read-only and input regions;
  the model lets any address outside the register file be read and written.

And one difference between the backends: an unwritten byte reads as 0 in C,
but throws `OutOfMemoryRange` in Lean. A proof states which bytes its initial
memory defines; a differential run against `ziskemu` loads every section of
the program into memory first.

## Next steps

1. **Memory and registers — done.** `rX`, `wX`, `read_mem`, `write_mem` and
   `rom_fetch` are defined, as described in
   [Registers and memory](#registers-and-memory), and `make lean-build`
   elaborates the whole model.
2. **Run the doubler.** Generate a ROM file from
   `ziskasm/examples/doubler-min`, give the C backend a `main` that loads the
   program's data sections and steps until `is_end`, and build it with
   `sail -c`. Then diff its trace step by step against `ziskemu`: `pc`, `a`,
   `b`, `c` and `flag` at every step. This is the first time the specification
   is checked against what ZisK actually does, and nothing after this step is
   worth more than that check.
3. **A one-slot correspondence, as a pilot.** Write the mapping from a
   `zisk_inst` plus machine state into one slot of a `Pil.Zisk.Main.Row`, and
   prove that `zisk_execute` on an `add` satisfies slot 0's constraints. Range
   checks enter as hypotheses, and `F` is fixed to Goldilocks where a step
   needs field laws (see [`lean/Proofs/Main.lean`](lean/Proofs/Main.lean)).
   Doing this with four opcodes, before adding the other 123, shows whether the
   shape works while changing it is still cheap.

   The mapping is where the model's `ind_width`-style typing meets the AIR's
   field columns, and it maps into a *slot*, not a row: Main packs four
   instructions per row, so every column is indexed (`(t i).pc 2`) and one
   `zisk_execute` corresponds to one of the four. A `.pil` line such as
   `main.pil:224` therefore yields four constraints, one per slot.
4. **Extract the range checks.** Teach
   [`pilout-constraints`](../tools/pilout-constraints/README.md) to read the
   range-check lookups among the pilout's hints, so that facts like "`a_src_mem`
   is 0 or 1" come from the generated AIR rather than being assumed in step 3.
5. **Fill in the opcode table**, tracked by `make ops`, with each new opcode
   checked against `ziskemu` the way step 2 sets up. Turn on `--strict` once
   coverage is complete.
6. **The full correspondence.** Extend step 3 to all four slots and every
   opcode, up to `zisk_execute` implies `Pil.Zisk.Main.holds`. Neither side is
   transcribed by hand, so neither can drift from what ZisK ships without the
   generators noticing.
7. **An `assembly` mapping** (optional). Sail mappings are bidirectional, so
   one definition yields both an assembler and a disassembler — which would
   subsume the round-trip test listed as a TODO in `ziskasm/README.md`
   (currently `src/assembler.rs` and `ZiskInst::to_zisk_asm` are two
   hand-written implementations of the same relation).

Out of scope throughout: that the witness computation fills the columns the
constraints describe, and the soundness of the STARK backend.
