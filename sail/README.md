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

**Early.** 8 of 129 opcodes are modeled — `copyb`, `add`, `eq`, `ltu`, which
the `ziskasm` doubler's own code uses, and `flag`, `and`, `pubout`, `halt`,
which its launcher and the BIOS add. The step function, the instruction record,
and the source/store/jump machinery are complete.

The register file, memory and the ROM are modeled, so every function the model
uses is defined and the generated Lean elaborates in full. The model also runs:
compiled to C, it executes `ziskasm/examples/doubler-min` — BIOS, launcher and
program, 245 steps — and `make difftest` checks every step of it against
`ziskemu`. See [Differential testing](#differential-testing).

The first theorem relating the model to the AIR is proved: for one slot of
Main and the internal ops, the constraints force exactly the state
`zisk_execute` produces, for the instruction the program holds at that pc,
reached through the ROM lookup. See
[The pilot correspondence](#the-pilot-correspondence) and
[The ROM lookup](#the-rom-lookup).

## Layout

| Path | What it is |
|------|-----------|
| `model/zisk_prelude.sail` | Bitvector comparison operators and `zero_extend`/`zeros` wrappers. Sail's standard library leaves these to each model; `sail-riscv` defines the same set. |
| `model/zisk_types.sail` | Architectural state (`PC`, `Zc`, `Zflag`, `Step`, and `Zerror`/`Zerror_code` for a run that ends by `halt`) and the source/store enums. |
| `model/zisk_mem.sail` | The register file (`rX`, `wX` over `x1`–`x31`) and byte-addressed memory (`read_mem`, `write_mem`). See [Registers and memory](#registers-and-memory). |
| `model/zisk_ops.sail` | One `union clause` + one `function clause` per opcode: `(c, flag) = op(a, b)`. Mirrors `core/src/zisk_ops.rs`. |
| `model/zisk_inst.sail` | The `zisk_inst` record, mirroring `ZiskInst` in `core/src/zisk_inst.rs`. |
| `model/zisk_rom.sail` | The default ROM: `rom_fetch` with no instructions. A program supplies its own file, selected with `make ROM=…`. |
| `model/zisk_step.sail` | The execution step, mirroring `Emu::step` in `emulator/src/emu.rs`: `zisk_execute(inst)` runs one instruction, `zisk_step()` fetches it from the ROM first. |
| `model/zisk_main.sail` | The C emulator's `main`: initial state, the step loop, and the trace `make difftest` compares. Not part of the Lean build. |
| [`../tools/sail-difftest`](../tools/sail-difftest/src/main.rs) | Exports a program to the model — `rom.sail` and its initial memory — records the reference trace on `ziskemu`, and checks the program's committed ROM rows against the encoding the Lean proofs assume. |
| `check_ops.py` | Guard: fails if the model names an opcode that no longer exists in Rust. |
| `check_frops.py` | Checks every `add` row of the FROPS tables the setup commits against `c = a + b`; `make frops`. |
| `lean/` | A lake package that typechecks both generated Lean trees — one library points at `build/lean/out` (this model), the other at `build/pil` (the AIR constraints) — plus `lean/Proofs/`, the hand-written proofs about them. |
| `lean/Proofs/Slot.lean` | The pilot correspondence: one slot of Main against one step of the model. |
| `lean/Proofs/Rom.lean` | The ROM lookup: the slot's instruction derived from the program, through the bus. |
| `lean/Proofs/BinaryAdd.lean` | BinaryAdd computes `add`: the `c` it proves is `a + b` mod 2^64. |
| `lean/Proofs/OpBus.lean` | The 29 operation-bus provers that cannot prove `add`, each proved so. |
| `difftest/` | Programs `make difftest-all` runs besides the doubler: `halt/` ends through the BIOS's `halt`. |

## Building

Needs Sail 0.19+ (`opam install sail`). This section is the per-target
reference; for the whole path from a fresh clone, with the output each command
produces, see [Verifying ZisK from a fresh clone](#verifying-zisk-from-a-fresh-clone).

```sh
make check   # typecheck the model
make lean    # generate Lean definitions into build/lean/out
make ops     # check opcode names against core/src/zisk_ops.rs
make pil     # generate Lean definitions of the AIR constraints into build/pil
make frops   # check the FROPS tables' add rows (needs the PIL compile's tmp/fixed)
```

Two more targets typecheck the generated Lean, through the lake package in
`lean/`. They need a Lean toolchain — install
[elan](https://lean-lang.org/install/):

```sh
make pil-build    # elaborate the AIR constraints  (all 54 AIRs build, ~87s)
make proofs       # check lean/Proofs against them (~21s)
make lean-build   # elaborate the model            (all 14 modules build, ~6s)
```

And two that compile the model to C, which needs a C compiler and the GMP and
zlib headers Sail's runtime links against:

```sh
make emu ROM=…    # build the C emulator for one ROM into build/c/zisk_emu
make difftest     # run PROG on ziskemu and on the model, and diff every step
make difftest-all # the same for every program the model is difftested on
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
> The theorem that would earn the phrase "formally verified" — that any trace
> the AIRs accept is a run of the model's `Out.Functions.zisk_execute` — does
> not exist. What is proved is a pilot of it: one slot of Main, two opcodes,
> two sources, with the soundness of the ROM lookup argument assumed (see
> [The pilot correspondence](#the-pilot-correspondence)). Delete this disclaimer
> when step 6 of [Next steps](#next-steps) lands.

Nothing below is assumed already built, and every figure quoted is what the
command printed against the pilout this branch was last regenerated on: 54
AIRs, 4188 constraints.

### What you need

| Tool | For | Install |
|------|-----|---------|
| Rust | the workspace and the constraint extractor | [rustup](https://rustup.rs); `rust-toolchain.toml` pins the version |
| Node + npm | the pil2 compiler, pulled at the version proofman pins | any recent LTS (v24 here) |
| Python 3 | `check_ops.py` | any 3.x |
| Sail 0.19+ | the ISA model | `opam install sail` |
| A C compiler, GMP, zlib | the model's C emulator, for `make difftest` | e.g. `apt install build-essential libgmp-dev zlib1g-dev` |
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
Rust ops: 129   Sail clauses: 8   modeled: 8/129 (6%)

UNMODELED -- 121 Rust ops with no Sail clause:
  add256, add_u_w, add_w, andn, arith256, arith256_mod, ...

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
wrote build/pil/Pil/Main.lean (713 constraints)
  8092 expressions reachable of 11376, 711 named intermediates, 181 witness columns
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

About 87 s from cold, seconds incrementally. Each of the 4188 constraints is
now a Lean `Prop` that Lean has checked is well formed: the columns it names
exist, and its indices are within the extents the pilout declares.

**7. Check the proofs.**

```sh
make proofs
```

```
✔ [51/52] Built Proofs
Build completed successfully.
```

About 21 s from cold: elaborating the model, which `Proofs/Slot.lean` imports, and the op-bus exclusions.
[`lean/Proofs/Main.lean`](lean/Proofs/Main.lean) holds six theorems proved from
the generated Main AIR alone, and records where the abstraction stops;
[`lean/Proofs/Slot.lean`](lean/Proofs/Slot.lean) is the pilot correspondence.

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

**9. Run the model against ziskemu.**

```sh
make difftest
```

```
wrote build/difftest/doubler-min: 44 instructions, 8 opcodes (add, and, copyb, eq, flag, halt, ltu, pubout), 245 steps on ziskemu
RomHolds: the committed ROM row of all 44 instructions is RomRowEncodes
...
OK: 245 steps and the output agree with ziskemu
```

A few seconds, most of it the C compile. On a divergence it prints the first
lines of the diff and fails instead; the two traces are left in
`build/difftest/doubler-min/`. `make difftest-all` also runs `difftest/halt`,
which ends the run in error through the BIOS's `halt` (15 steps, exit code 3).

Optionally, the extractor's own tests: `cargo test -p zisk-pilout-constraints`,
15 tests.

### What a clean run establishes

- The ISA model is well typed, and every opcode it names still exists in
  `core/src/zisk_ops.rs`.
- The model's generated Lean elaborates, with registers, memory and the ROM
  defined, so it is a complete Lean definition of one ZisK step.
- The same model, compiled to C, runs `doubler-min` to the same result as
  `ziskemu`, agreeing with it at every one of the 245 steps.
- The 4188 constraints of the pilout ZisK ships are well-formed propositions
  over a commutative ring, in Lean, **generated rather than transcribed**, each
  carrying the `.pil` file and line it came from.
- Those definitions are usable for proof, not merely well formed: six
  theorems about the Main AIR go through by `grind` alone.
- For one slot of Main and the `copyb` and `flag` ops, with immediate or `c`
  sources, the constraints force the slot's `c` and `flag` and the next slot's
  `pc` to be exactly what `zisk_execute` computes, for the program's
  instruction at that pc, given a sound ROM lookup.
- The encoding of an instruction in the committed ROM that this relies on is
  the one the ROM setup code produces, checked row by row for every program
  `make difftest` runs.

### And what it does not

- **The correspondence is a pilot.** One slot of four, two opcodes, two
  sources, no stores, no `set_pc`. The rest is steps 4 to 6 of
  [Next steps](#next-steps).
- **Bus arguments are assumed sound, not derived.** The ROM lookup's guarantee
  (`LookupHolds`) and the ROM's contents (`RomHolds`) are hypotheses; the gsum
  columns and the global constraints that enforce a bus are not modeled.
- **121 of 129 opcodes are unmodeled**, and two programs have been run. The
  difftest covers what they execute: eight opcodes, and of the sources and
  stores, the ones they use.
- **The C backend is tested, the Lean backend is not.** Both are generated from
  the same Sail, but agreeing with `ziskemu` in C is evidence about the model,
  not a proof that the Lean definition says the same thing.
- **Only the ROM bus is used.** Every bus argument of every AIR is extracted
  (1629 of them, from the pilout's hints), but only the ROM lookup has been
  given a meaning and used in a proof. The operation bus that carries `add` and
  every other external op, the memory bus, and the range checks are next.
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

## Differential testing

`make difftest` runs one program on `ziskemu` and on the model's C emulator,
and diffs the two runs step by step. It is the check that the specification
says what ZisK actually does, which no proof can provide: a proof only shows
that the AIR agrees with the model, so the model has to be right first.

The pipeline:

1. [`tools/sail-difftest`](../tools/sail-difftest/src/main.rs) assembles
   `PROG` with the same code `ziskemu -z` uses, and writes three things into
   `build/difftest/<prog>/`. `rom.sail` holds every instruction of the ROM, BIOS
   included, as a `rom_fetch`. `mem/` holds the initial memory sections: the
   ROM's data and the input behind its 8-byte header, as
   `Emu::create_emu_context` lays them out. `ziskemu.trace` is the reference
   run, recorded by stepping `Emu` directly.
2. The model is compiled to C with that ROM and `model/zisk_main.sail`, and
   run with the memory sections loaded through `-b addr,file`.
3. The two traces are diffed.

Each line of the trace is a step: the pc it ran at, `c`, `flag`, and every
register the step changed. After the last step comes the output region. `a`
and `b` are not traced, because they are locals of `zisk_execute`, and exposing
them would mean adding state the Lean side would carry into every proof. A
wrong source still shows up, in `c` or in a register, at the step that uses
it. A wrong memory write shows up when the value is read back, or in the
output.

The exporter refuses what the model does not describe, rather than exporting
something the model would get subtly wrong: an instruction that uses the stack
pointer or takes extended arguments (the precompiles), or a source or store
kind with no Sail counterpart. An opcode with no Sail clause gets through the
exporter, then fails the Sail typecheck with the opcode's name.

It does catch what it should: changing `ltu` to answer wrongly for one value
makes `make difftest` fail at the first step where that value appears.

The same run checks the hypothesis the Lean proofs make about the ROM,
`RomHolds`: for every instruction, the row the ROM setup commits for it —
computed by `CustomRom::build_trace`, the function the setup itself calls — is
the `RomRowEncodes` of the instruction `rom.sail` exports, field by field, with
the opcode table and flag packing as the Lean side defines them. A mismatch
names the row and the field; giving `add` the wrong opcode in the Lean table's
mirror fails at the first `add` in the ROM.

## The pilot correspondence

[`lean/Proofs/Slot.lean`](lean/Proofs/Slot.lean) holds the first theorem that
relates the two generated definitions, `Out.Functions.zisk_execute` and
`Pil.Zisk.Main.everyRow`. In words, `slot1_internal` says: if slot 1 of a row
holds the instruction the model is about to execute, at the model's pc, and
slot 0's `c` is the model's `c`, then Main's constraints force slot 1's `c`
and `flag`, and slot 2's `pc`, to encode exactly the state `zisk_execute`
leaves. It holds for `copyb` and `flag`, with immediate or `c` sources, no
store, no `set_pc`, and a jump that does not wrap the address space. The proof
takes 9s and depends on no axioms beyond Lean's standard three.

**The direction is soundness.** The plan said "`zisk_execute` implies
`Main.holds`": a model step yields a trace the AIR accepts. The pilot proves
the other way round, that what the AIR accepts is what the model does, because
that is the property a proof system needs: an accepted proof must mean the
program ran correctly. The full theorem in step 6 is stated this way too.

Three things the pilot settled about the shape:

- **Slot 1, not slot 0.** Main packs four steps per row. Slot 1 reads its
  previous `c` from slot 0 and hands its pc to slot 2 inside one row, while
  slot 0 reads across a row boundary, or a segment boundary on the first row.
  The boundary cases belong to the full correspondence.
- **Main alone cannot prove `add`.** Main fixes `c` and `flag` only for the
  internal ops, `flag` (op 0) and `copyb` (op 1). For every other op it sends
  `(op, a, b, c, flag)` over the operation bus, and the AIR that computes the op
  constrains it; nothing in Main says `c = a + b`. So the pilot's opcodes are
  the internal ones, and `add` waits for the bus. The same holds for memory and
  register sources and stores, which go over the memory bus.
- **The ROM lookup was first a hypothesis.** `SlotHolds` says which
  instruction a slot holds, column by column. In the AIR that is the ROM
  lookup, which the pilot first assumed; it is now derived, see
  [The ROM lookup](#the-rom-lookup).

The encoding is the one Main uses: a 64-bit value is two 32-bit limbs, low
first; a jump offset is a signed field element; a pc is its value. Everything
holds over any commutative ring, so no field laws were needed.

Writing it needed one change to the extractor: `everyRow` now comes with an
accessor per constraint (`h.c46`), because `grind` cannot search a conjunction
713 constraints wide. See
[`tools/pilout-constraints/README.md`](../tools/pilout-constraints/README.md).

## The ROM lookup

[`lean/Proofs/Rom.lean`](lean/Proofs/Rom.lean) derives the pilot's
`SlotHolds` instead of assuming it. In the AIR, each of Main's four slots looks
its instruction columns up on bus 7890, and the Rom AIR proves the program's
rows there; `slot1_internal_rom` is the pilot restated on top of that, with
the instruction now `prog pc`, the program's.

The bus arguments come from the generator: every pil2 lookup, permutation and
range check leaves a `gsum_debug_data` hint naming its bus, its side, its
count and its tuple, and `pilout-constraints` emits each one as a `BusArg`
(see [its README](../tools/pilout-constraints/README.md#bus-arguments)). So the
lookup is stated over the generated `Main.bus87` and `Rom.bus0`, not over a
hand-copied list of columns.

What is assumed instead of `SlotHolds`:

- **`LookupHolds`**: the lookup argument is sound — every tuple Main assumes
  is a tuple the Rom proves, at a row with nonzero multiplicity. It is the
  guarantee of the gsum argument, which is not modeled; it also checks that
  the two arguments it pairs are on the same bus, one assuming and one proving.
- **`RomHolds prog`**: the Rom's committed `rom` columns encode the program.
  That commitment is fixed per program, and its root is a public of the proof,
  so this is a statement about the setup. `make difftest` checks it for every
  program it runs, against the setup's own code; see
  [Differential testing](#differential-testing).

The derivation needs two field facts, so it is stated for `F` a field of the
Goldilocks characteristic rather than any commutative ring: the Rom's
`is_data * multiplicity = 0` makes a row in use a code row only because a
nonzero multiplicity is invertible, and the flag bits come out of the packed
`rom_flags` only because a number below `2^16` is a distinct field element.
Each bit is first a bit by Main's booleanity constraints.

Writing it surfaced a fact for later: `halt` (op `0xff`) can never be proved.
Main's internal-op constraints with that opcode force both `flag = 1` and
`flag = 0`, so no accepted trace executes it — which is what a program that
fails should get.

## Who can prove an `add`

The ROM bus had one prover. The operation bus (5000), which carries every
external op from Main to the AIR that computes it, has forty: an `add` slot in
Main is correct only if *every* AIR that could prove a tuple with `op = 0x0a`
constrains its `c` to be `a + b`. So before proving `add`, the provers were
sorted by what their tuple's `op` can be, read from the generated bus
arguments (`op` is the tuple's first element):

| Provers | `op` | Can be `add`? |
|---|---|---|
| Add256, Blake2br, Blake2s, Blake3f, JumpDest, Keccakf, Sha256f | a constant (240, 234, 239, 238, 192, 241, 249) | no |
| Poseidon | `235 + sel_poseidon1` | no (235 or 236) |
| Arith256X, ArithBn254, ArithEq, ArithEq384, ArithSecp256K1, TwistedEdwards, Dma, Dma64Aligned\*, DmaUnaligned (and their Large variants) | a sum of selectors times opcodes | no: every selector is constrained boolean, and no 0/1 combination sums to 10 |
| Arith | its `op` column | no, given its table: the Arith table's `OP` is a mul/div code, `0xB0`–`0xBF` |
| BinaryExtension (2 variants) | its `op` column | no, given its table: shifts, sign extension and bit counts |
| **BinaryAdd, BinaryAddHi** (6 variants) | `10 + 78·sel_sh3add` | **yes**: `add`, or `sh3add` |
| **Binary** (3 variants) | `b_op + 16·mode32` | **yes**, with `b_op = 0x0a`, `mode32 = 0`: byte by byte, through the Binary table |
| **VirtualTableZisk0** | 8 of its 10 arguments | **yes**: the FROPS tables, precomputed results |

So an `add` has three kinds of prover, and each needs its own argument: the
dedicated adder's constraints, the general Binary AIR's byte-wise table
lookups, and a table of precomputed results.

**The exclusions are proved.** [`lean/Proofs/OpBus.lean`](lean/Proofs/OpBus.lean)
proves, for each of the 29 proving arguments in the first three rows of the
table, that its op is never 10, from the AIR's own constraints. One lemma does
it: an op of the form `c₀ + Σ cₖ·sₖ` with every `sₖ` a bit takes one of the
values `c₀ + Σ_{k∈S} cₖ`; `decide` checks that none is 10 (2048 subsets for
ArithEq), and each `sₖ` is a bit by the AIR's booleanity constraint, reached by
column name (`h.bool_sel_arith256`). All 29 check in about 3 seconds, on Lean's
standard three axioms. Arith and BinaryExtension, ruled out only by their
tables, are not among them.

**The FROPS tables are checked, not assumed.** They are fixed data the pilout
does not carry, compiled into `VirtualTableZisk0`. `make frops` runs
[`check_frops.py`](check_frops.py): for every argument the virtual table
proves on the operation bus, it evaluates the tuple exactly as the extracted
bus argument states it, at every row of the compiled fixed file the setup
commits (`tmp/fixed/VirtualTableZisk0.fixed`), and checks every `add` row:
`c = a + b mod 2^64`, `flag = 0`, every limb below `2^32`. All 15,604,641 `add`
rows pass, in about 25s, the same count as the source table
`binary_basic_frops_fixed.bin`. Six of the eight sub-tables hold only 32-bit
operands, their high limbs folded to constants by the compiler, which is why
the check reads the compiled table rather than the source file. Making the
checker expect `a + b + 1` fails every row.

Two "no, given its table" entries rest on the same kind of fixed data, the
Arith and BinaryExtension tables; `check_frops.py` is the place to check them
too.

**BinaryAdd adds — proved.** [`lean/Proofs/BinaryAdd.lean`](lean/Proofs/BinaryAdd.lean):
at a row where BinaryAdd proves a tuple whose op is `add`, on operands that are
the limbs of two 64-bit values `A` and `B`, the `c` limbs it proves are those of
`A + B`. Op 10 clears `sel_sh3add` (78 is not zero in the field), the carries
are bits by the AIR's constraints, and each `c` limb is two 16-bit chunks by
four range checks. The range checks enter as `RangeHolds`, the guarantee of a
sound range check, with the range read from the AIR's generated `ranges`
(bus 104, `[0, 65535]`) rather than written in the proof. The two limb
equations then become equations between naturals below the characteristic, and
the rest is arithmetic. It depends on Lean's standard three axioms.

The operands being the model's limbs is the job of the lookup that carries the
tuple from Main; the variants (`BinaryAddLarge`, `Huge`, and the `Hi` family)
are the same template with more lanes, still to be covered.

## Next steps

1. **Memory and registers — done.** `rX`, `wX`, `read_mem`, `write_mem` and
   `rom_fetch` are defined, as described in
   [Registers and memory](#registers-and-memory), and `make lean-build`
   elaborates the whole model.
2. **Run the doubler — done.** `make difftest` runs
   `ziskasm/examples/doubler-min` on both emulators and diffs every step; see
   [Differential testing](#differential-testing). It took four more opcodes
   than the program's own four, because the BIOS and the launcher run too.
3. **A one-slot correspondence, as a pilot — done.** See
   [The pilot correspondence](#the-pilot-correspondence): slot 1, `copyb` and
   `flag`, immediate and `c` sources.
4. **Extract the buses — in progress.** Teach
   [`pilout-constraints`](../tools/pilout-constraints/README.md) to read the
   lookup and permutation arguments among the pilout's hints, so the facts the
   pilot assumes or cannot reach come from the generated AIR. In order of what
   they unlock:
   - the **ROM lookup** — done: every bus argument is extracted, and
     `SlotHolds` is derived from the ROM lookup; see
     [The ROM lookup](#the-rom-lookup);
   - the **operation bus**, which ties an external op in Main to the AIR that
     computes it, so `add`, `eq`, `ltu` and the rest can be proved. Started:
     the census of who can prove an `add` is done and the FROPS rows are
     checked, BinaryAdd is proved to add, and the 29 provers that cannot
     prove `add` are proved not to (see
     [Who can prove an `add`](#who-can-prove-an-add)); what remains is
     BinaryAdd's other variants, Binary's byte-wise lookups, the Arith and
     BinaryExtension tables, and the `add` pilot on top;
   - the **memory bus**, for memory and register sources and stores;
   - the **range checks**, for limbs known to be below `2^32`.
5. **Fill in the opcode table**, tracked by `make ops`, with each new opcode
   checked by `make difftest` on a program that executes it
   (`make difftest PROG=… INPUT=…`). Turn on `--strict` once coverage is
   complete.
6. **The full correspondence.** Extend the pilot to every slot, the row and
   segment boundaries, every source, store and opcode, up to: a trace every
   AIR accepts is a run of `zisk_execute`. Neither side is transcribed by hand,
   so neither can drift from what ZisK ships without the generators noticing.
7. **An `assembly` mapping** (optional). Sail mappings are bidirectional, so
   one definition yields both an assembler and a disassembler — which would
   subsume the round-trip test listed as a TODO in `ziskasm/README.md`
   (currently `src/assembler.rs` and `ZiskInst::to_zisk_asm` are two
   hand-written implementations of the same relation).

Out of scope throughout: that the witness computation fills the columns the
constraints describe, and the soundness of the STARK backend.
