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
| `check_tables.py` | Checks the virtual tables the setup commits against what the proofs assume: every FROPS row of a modeled op (`add`, `ltu`, `eq`, `and`) carries its result, the Arith and BinaryExtension tables never serve one, the Binary table's rows for them compute their byte, and the precompiles' fixed ops on the memory bus are 1 or 2; `make tables`. |
| `gen_membus.py` | Generates `lean/Proofs/MemBus.lean` (every argument on the memory bus) and `lean/Proofs/MainRegs.lean` (Main's register pairs); `make membus`. |
| `gen_provers.py` | Generates `lean/Proofs/Provers.lean` from the AIRs' operation-bus arguments and the prover lemmas' signatures; `make provers`. |
| `lean/` | A lake package that typechecks both generated Lean trees — one library points at `build/lean/out` (this model), the other at `build/pil` (the AIR constraints) — plus `lean/Proofs/`, the hand-written proofs about them. |
| `lean/Proofs/Slot.lean` | The pilot correspondence: one lane of Main against one step of the model, at any of the four lanes. |
| `lean/Proofs/Rom.lean` | The ROM lookup: the slot's instruction derived from the program, through the bus. |
| `lean/Proofs/Adders.lean` | The six adders, 21 lanes, prove `add` correctly (`AddsAt`). |
| `lean/Proofs/Binary.lean` | Binary, 7 lanes, proves `add` correctly, byte by byte through its table. |
| `lean/Proofs/Bus.lean` | What a bus guarantees: `LookupHolds`, for any number of provers, and `TableNever`. |
| `lean/Proofs/OpBus.lean` | The 33 operation-bus provers that cannot prove `add`, each proved so. |
| `lean/Proofs/AddPilot.lean` | The `add` pilot: slot 1 executing `add`, through the operation bus. |
| `lean/Proofs/Ops.lean` | External ops in general: `ltu`, `eq`, `and` and `add` as `sem`, and what a prover of one must do (`OpProves`). |
| `lean/Proofs/BinaryOps.lean` | Binary, 7 lanes, proves `ltu`, `eq` and `and`, byte by byte through its table. |
| `lean/Proofs/OpBusCmp.lean` | The 54 operation-bus provers that cannot prove `ltu`, `eq` or `and`, each proved so. |
| `lean/Proofs/ExtPilot.lean` | The external ops at any lane (`lane_external`), through that lane's operation-bus lookup; slot 1 executing `add`, `ltu`, `eq` and `and`. |
| `lean/Proofs/Provers.lean` | Every prover on the operation bus, collected: 61 arguments in 39 AIRs and the 12 of the FROPS table, each proved to compute the four external ops, and the run over them (`run_system`). Generated. |
| `lean/Proofs/Segments.lean` | Across segments: from the boot state, along the continuation bus's chain of Main segments, every step is a step of the model (`run_from_boot`). |
| `lean/Proofs/End.lean` | The program's end: from boot, the model runs the program to an end instruction and stops (`run_to_end`). |
| `lean/Proofs/MemCheck.lean` | Offline memory checking, in the abstract: from a permutation of proved and assumed tuples and strictly increasing steps, a register read returns the last write (`reads_last_write`). |
| `lean/Proofs/MemBus.lean` | No AIR but Main sends a register tuple on the memory bus: 173 arguments in 39 AIRs, each proved never to send op 3. Generated. |
| `lean/Proofs/SumBus.lean` | What a sound sum bus guarantees, tuple by tuple (`SumHolds`), and how senders that never send a tuple drop out. |
| `lean/Proofs/RegChain.lean` | From the memory bus's balance to a permutation of lists: senders of bits, counts below the characteristic (`perm_of_balance`). |
| `lean/Proofs/MainRegs.lean` | Main's 168 register-access pairs per segment are good, their gaps range-checked. Generated. |
| `lean/Proofs/Registers.lean` | Every register access in Main reads the last write (`MainSegs.reads`). |
| `lean/Proofs/ModelRegs.lean` | The model's step in general: any op but `halt`, register sources and stores (`execute_gen`). |
| `lean/Proofs/Execution.lean` | The runs and the register file over one chain of segments; each segment's first step from the continuation chain. |
| `lean/Proofs/RegFile.lean` | A register's value at a memory step (`regVal`): what an access proves, what a read keeps and a store sets. |
| `lean/Proofs/LaneGen.lean` | One lane over known source columns: its `c`, flag and next pc, for any source. |
| `lean/Proofs/StepRegs.lean` | One step with register sources and stores, given what the register sources put in the lane's columns (`step_gen`). |
| `lean/Proofs/RegRun.lean` | Main's register chain at a lane's memory steps: reads keep the value, a store sets it (`lane_read`, `store_set`, `store_none`). |
| `lean/Proofs/RegStep.lean` | The model's register file through one lane's step: sources read it, the store updates it (`reg_srcs`, `reg_update`). |
| `lean/Proofs/RegBoot.lean` | A run from boot with the register file: after `n` steps the model's registers are Main's register chain's (`run_from_boot_mem`). |
| `lean/Proofs/MemSort.lean` | Sorted memory checking, in the abstract: along a log sorted by address and step, every read returns the latest write (`MemSort.reads_last_write`). |
| `lean/Proofs/MemRam.lean` | The RAM's constraints as a sorted log, over a chain of its segments: every read returns the last write (`ram_reads_last_write`). |
| `lean/Proofs/MemInput.lean` | The input memory, over a chain of InputData segments: two reads of the same address, other than the base, return the same value (`input_consistent`). |
| `lean/Proofs/RegEnd.lean` | The program's end with the register file: the model runs to an end instruction of any op, source and store, with the chain's registers, and stops (`run_to_end_mem`). |
| `lean/Proofs/Run.lean` | Many steps: from an encoded start, Main's trace forces a run of the model, lane after lane and row after row, for programs of all six modeled opcodes. |
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
make tables  # check the virtual tables' contents (needs the PIL compile's tmp/fixed)
make provers # regenerate lean/Proofs/Provers.lean (after make pil, or a new prover lemma)
make membus  # regenerate lean/Proofs/MemBus.lean and MainRegs.lean (after make pil)
```

Two more targets typecheck the generated Lean, through the lake package in
`lean/`. They need a Lean toolchain — install
[elan](https://lean-lang.org/install/):

```sh
make pil-build    # elaborate the AIR constraints  (all 54 AIRs build, ~87s)
make proofs       # check lean/Proofs against them (~130s)
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
> not exist. What is proved is a pilot of it: runs of six opcodes (`copyb`,
> `flag`, and `add`, `ltu`, `eq`, `and` through the operation bus) from the
> boot state to the program's end, across every lane, row and segment, with
> two sources, the
> soundness of the bus arguments and the contents of the tables assumed (see
> [The pilot correspondence](#the-pilot-correspondence),
> [Many steps](#many-steps), [Across segments](#across-segments),
> [The program's end](#the-programs-end) and
> [The `add` pilot](#the-add-pilot) and
> [`ltu`, `eq` and `and`](#ltu-eq-and-and)). Delete this disclaimer
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
✔ [104/105] Built Proofs
Build completed successfully.
```

About 130 s from cold: elaborating the model, which `Proofs/Slot.lean` imports, the op-bus exclusions, the adders, Binary, the pilots for `add` and for `ltu`, `eq` and `and` — `ExtPilot.lean`, about 47 s of it — the runs, the operation bus's provers collected, the runs across segments, the program's end, the registers' memory checking, the runs with registers, to the end, and sorted memory checking.
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
18 tests.

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
- For the `copyb` and `flag` ops, with immediate or `c` sources, at any of
  Main's four lanes, the constraints force the lane's `c` and `flag` and the
  next step's `pc` to be exactly what `zisk_execute` computes, for the
  program's instruction at that pc, given a sound ROM lookup.
- For a program of those ops, steps chain: from a position that encodes the
  model's state, the trace forces `n` steps of the model, lane after lane and
  row after row within a segment, and ends encoding the state they reach. The
  same for programs that also use `add`, `ltu`, `eq` and `and`, at any lane,
  given a sound operation bus: each of its 73 provers, in every instance of
  its AIR, is proved to compute each op, from its constraints or its table's
  checked contents.
- And across segments, from the boot state: along a chain of Main segments
  linked by a sound continuation bus, every step the trace holds is a step of
  the model, the first from the boot pc with `c = 0`; and when the chain
  closes at the end pc, the model runs the program to an end instruction and
  stops.
- With registers: given a sound memory bus as well, register sources and
  register stores are covered too, and after every step the model's
  registers `x1`–`x31` are what Main's register chain holds — up to the end
  instruction, which the model executes and stops.
- The same for `add`, whose result another AIR computes: given a sound
  operation bus, whichever of its provers answers, the slot's `c` is
  `a + b` modulo 2^64.
- The same for `ltu`, `eq` and `and`: `c` and `flag` are `[a < b]` unsigned,
  `[a = b]`, and `a & b` with no flag, and the next `pc` follows the flag.
- The encoding of an instruction in the committed ROM that this relies on is
  the one the ROM setup code produces, checked row by row for every program
  `make difftest` runs.

### And what it does not

- **The correspondence is a pilot.** Six opcodes; immediate, `c` or register
  sources, and no store or a register store, with no memory or indirect
  access; `set_pc` only with a short `jmp_off1`, and in the runs without
  registers only on internal ops. The rest is steps 4 to 6 of
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
  given a meaning and used in a proof, together with the operation bus for
  `add`, `ltu`, `eq` and `and` and the range checks the adders use. The
  operation bus for the other external ops, and the memory bus, are next.
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
store, no `set_pc`, and a jump that does not wrap the address space. It is now
the slot-1 instance of `lane_internal`, the same statement for any lane (see
[Many steps](#many-steps)); the file takes 17s and depends on no axioms beyond
Lean's standard three.

**The direction is soundness.** The plan said "`zisk_execute` implies
`Main.holds`": a model step yields a trace the AIR accepts. The pilot proves
the other way round, that what the AIR accepts is what the model does, because
that is the property a proof system needs: an accepted proof must mean the
program ran correctly. The full theorem in step 6 is stated this way too.

Three things the pilot settled about the shape:

- **Slot 1 first.** Main packs four steps per row. Slot 1 reads its
  previous `c` from slot 0 and hands its pc to slot 2 inside one row, while
  slot 0 reads across a row boundary, or a segment boundary on the first row.
  The other lanes and the boundaries came next; see [Many steps](#many-steps).
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

- **`LookupHolds`** ([`lean/Proofs/Bus.lean`](lean/Proofs/Bus.lean)): the
  lookup argument is sound — every tuple Main assumes is a tuple the Rom
  proves, at a row with nonzero multiplicity. It is the guarantee of the gsum
  argument, which is not modeled. It is stated for any list of provers, from
  AIRs of any type, comparing tuples up to zero padding and the bus per row as
  the bus itself does; the ROM is the case of one prover. It also refuses a
  pairing that is not one: the assuming side must assume on one bus, and every
  prover must prove and serve that bus.
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
| Arith | its `op` column | no: its table's ops are mul/div codes, `0xB0`–`0xBF` |
| BinaryExtension (2 variants) | its `op` column | no: its table's ops are shifts, sign extension and bit counts |
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
standard three axioms.

Arith and BinaryExtension are ruled out by their tables, so they are proved
from three facts: the lookup that ties their `op` column to the table
(`LookupHolds`, stated over `Arith.bus41` and the merged `VirtualTableZisk1`,
which serves the Arith table on bus 331 and BinaryExtension's on 124), the
first element of a looked-up tuple being the op, and the table never holding
`add` (`TableNever`), which `make tables` checks. Four theorems, one per
proving argument (BinaryExtensionLarge has two lanes).

**The tables are checked, not assumed.** They are fixed data the pilout does
not carry, compiled into `VirtualTableZisk0` (the FROPS tables) and
`VirtualTableZisk1` (the Arith, BinaryExtension and range tables, among
others). `make tables` runs [`check_tables.py`](check_tables.py): for every
argument the virtual table
proves on the operation bus, it evaluates the tuple exactly as the extracted
bus argument states it, at every row of the compiled fixed file the setup
commits (`tmp/fixed/VirtualTableZisk0.fixed`), and checks every row of a
modeled op: `add` is `c = a + b mod 2^64` with `flag = 0`, `ltu` and `eq` are
`c = flag = [a < b]` (unsigned) and `[a = b]`, `and` is `c = a & b` with
`flag = 0`, every limb below `2^32`. All 18,168,075 such rows pass — 15,604,641
`add`, 625,346 `ltu`, 1,500,428 `eq` and 437,660 `and` — the same counts as
the source table `binary_basic_frops_fixed.bin`. Six of the eight sub-tables hold only 32-bit
operands, their high limbs folded to constants by the compiler, which is why
the check reads the compiled table rather than the source file. Making the
checker expect `a + b + 1` fails every row.

The same run checks `TableNever` for the two tables. A merged table serves
several buses through one argument, choosing per row, so the bus is evaluated
per row too: of `VirtualTableZisk1`'s rows, 124 are on the Arith table, serving
exactly the 14 mul/div opcodes, and about 2.5 million on BinaryExtension's,
serving shifts, sign extension and bit counts (`0x21`–`0x43`); none is `add`,
`ltu`, `eq` or `and`.
Forbidding `mul`, which the Arith table does serve, fails on its rows. It
also checks the Binary table (bus 125, five arguments of `VirtualTableZisk1`):
every row's op is below 2^10, and each row of a modeled op computes its byte:
262,144 `add` rows, 262,144 `ltu` (across two arguments), 262,144 `eq` and
131,072 `and`. Requiring one more than the true sum fails on half the `add`
rows; a wrong `ltu` rule fails 512 rows. The whole run takes about 18s.

**Every adder adds — proved.** [`lean/Proofs/Adders.lean`](lean/Proofs/Adders.lean)
covers the six dedicated adders, 21 lanes from two templates. What each lane
proves is stated the way the `add` pilot will consume it, over the tuple it
sends, `(op, a₀, a₁, b₀, b₁, c₀, c₁, flag)`: `AddsAt p j` says that if prover
`p`'s tuple at row `j` has op 10 and its `a` and `b` limbs are those of two
64-bit values `A` and `B`, its `c` limbs are those of `A + B` and its flag is 0.

- **BinaryAdd** (1, 2, 4 lanes; `adder64`): two 32-bit limbs and a carry bit
  between them. Op 10 clears `sel_sh3add` (78 is not zero in the field), the
  carries are bits by the AIR's constraints, and each `c` limb is two 16-bit
  chunks by four range checks.
- **BinaryAddHi** (2, 4, 8 lanes; `adderHi`): a 32-bit result, `a`'s high half
  zero and `b`'s either zero or `0xFFFF_FFFF`. Without the high bits the low
  limb does not carry; with them it must, and the high halves wrap to 0 modulo
  2^64. Either way `c = A + B`, with a zero high limb.

The range checks enter as `RangeHolds` (`Bus.lean`), the guarantee of a sound
range check, with the range read from each AIR's generated `ranges` (bus 104,
`[0, 65535]`). With them the limb equations become equations between naturals
below the characteristic, and the rest is arithmetic. Each template's algebra
is one lemma over plain field values; the 21 lane instances, written out
mechanically from the generated AIRs, name their lane's constraints, its
intermediates and its range checks. All check in about 4 seconds, on Lean's
standard three axioms.

**Binary adds — proved.** [`lean/Proofs/Binary.lean`](lean/Proofs/Binary.lean)
covers the general Binary AIR, 7 lanes over `Binary`, `BinaryLarge` and
`BinaryHuge`, to the same `AddsAt`. Binary proves every binary op as eight byte
lookups into the Binary table, `(pos, op, a, b, cin, c, flags)`, chained by the
carry, so whether it adds is mostly a fact about the table:

- **`AddRow`**, what an `add` row of the table is: bytes below 256, a carry-in
  of 0 or 1, and the byte computed — `c + 256·cout = a + b + cin` with the
  carry as its flags in a middle byte, `c = (a + b + cin) mod 256` with the
  carry discarded in the last. Every row's op is also something other than
  −6. `make tables` checks this against the committed table
  (`BinaryTableAdds`): all 262,144 `add` rows, every byte, carry-in and
  position.
- **`binary8`**, the lane's algebra. Binary's op is `b_op + 16·mode32`, so an
  op of 10 could be `b_op = −6` in 32-bit mode; no table row has that op, so
  `mode32 = 0`. Its carry columns are not range-checked: a carry is pinned only
  as part of a byte's flags, next to `16·result_is_a + 32·use_first_byte`, and
  as the next byte's carry-in. An `add` row's carry-in is 0 or 1 and its flags
  at most 1, which forces those extra flag bits to 0 and each carry to the
  table's; the last byte's flags force the final carry, which Binary adds into
  `c` and sends as the flag, to 0. Then eight bytes and their carries are
  `A + B` modulo 2^64.

The natural-number arithmetic is split into small lemmas (`chain4`,
`limbs_sum`): `omega` given the whole eight-byte chain with its bounds at once
does not finish, and an `omega` deep inside the proof sees every hypothesis in
scope. The file checks in about 4 seconds.

## The `add` pilot

[`lean/Proofs/AddPilot.lean`](lean/Proofs/AddPilot.lean) proves the
correspondence for an op Main does not compute itself. `slot1_add`: if slot 1
holds an `add` the model is about to execute — the program's instruction,
through the ROM lookup — with immediate or `c` sources, then slot 1's `c` and
`flag` and slot 2's `pc` are exactly what `zisk_execute` computes: `c = a + b`
modulo 2^64, no flag, the next instruction.

Main sends the `add` over the operation bus, so the theorem takes the bus's
guarantee (`LookupHolds`) over an arbitrary list of provers, and one fact about
each, `OpAdds`: on the operation bus it proves `add` correctly. Every kind of
prover has its route to that fact:

| Provers | `OpAdds` from | Where |
|---|---|---|
| the 21 adder lanes | their constraints, `AddsAt` at every row (`opAdds_of_adds`) | `Adders.lean` |
| the 7 Binary lanes | their byte lookups and the Binary table (`opAdds_of_adds`) | `Binary.lean` |
| the FROPS tables | their contents, `FropsAdds` (`opAdds_of_frops`), checked by `make tables` | `check_tables.py` |
| the 33 others | their op is never 10 (`opAdds_of_not`) | `OpBus.lean` |

Main's own constraints give the rest: the sources, from `main.pil:456`–`461`
(`slot1_sources`); `m32 = 0` for an `add`, from the ROM's flags, so the high
limbs go to the bus unmasked; the flag of 0 from the prover; and the pc, from
the transition with no `set_pc` and no flag. Standard three axioms.

What it still takes as given: the soundness of the two lookups, the ROM, the
tables `make tables` checks, slot 1's pc and slot 0's `c` being the model's,
and the pilot's scope — one slot, immediate and `c` sources, no store. Each
prover's `OpAdds` is proved for any one of its rows; the list the lookup
answers from, every instance of every AIR, is collected in
[The operation bus's provers](#the-operation-buss-provers).

## `ltu`, `eq` and `and`

The other external ops the model has follow the `add` pilot's route, prover by
prover, with what the pilot needed of `add` stated for any op in
[`lean/Proofs/Ops.lean`](lean/Proofs/Ops.lean): each op is a `sem`,
`(c, flag)` of two 64-bit values, proved to be the model's `zisk_exec`
(`ltuSem`, `eqSem`, `andSem`), and `OpProves code sem p` says prover `p`, at
every row it proves on the operation bus with a nonzero count, carries `sem`
of its operands when its op is `code`. The count matters only for Dma (below).

**The census.** The same provers as for `add`, sorted anew:

| Provers | `OpProves` from | Where |
|---|---|---|
| the 7 Binary lanes | their byte lookups and the Binary table | `BinaryOps.lean` |
| the FROPS tables | their contents, `FropsProves` (`opProves_of_frops`), checked by `make tables` | `check_tables.py` |
| the 21 adder lanes, the 29 constant- and selector-op provers | their op is never 6, 9 or 14 (`opProves_of_not`) | `OpBusCmp.lean` |
| Arith, BinaryExtension (4 arguments) | their table never holds those ops (`TableNever`) | `OpBusCmp.lean` |

The 29 are proved as for `add`, enumerating the values their op can take, now
for three codes. Dma is the exception: its op is a sum of opcodes plus
`6·sel_extended`, which is `ltu`'s code exactly when no base selector is set —
and then its count, the sum of the same selectors, is 0. So for `ltu` it is
excluded through its count (`Dma_bus51_not_ltu`, the op and the count
enumerated together over the same bits), and for `eq` and `and` through its op.

**Binary computes them — proved.**
[`lean/Proofs/BinaryOps.lean`](lean/Proofs/BinaryOps.lean) proves, for each of
the 7 lanes, all three ops at every row. As for `add`, it rests on what the
table's rows are (`OpRow`, checked by `make tables` as `BinaryTableOps`):

- **`ltu`**: each byte's flags are 1 if `a < b`, its carry-in if `a = b`, 0
  otherwise, and `c` is 0. Chained from the lowest byte, the last byte's flags
  are `[A < B]`.
- **`eq`**: each byte's flags are 1 unless the carry-in is 0 and the bytes are
  equal, inverted in the last byte, so the result is `[A = B]`.
- **`and`**: each byte is `a & b`, no carry.

The 32-bit forms are excluded as for `add`: Binary's op is `b_op + 16·mode32`,
and no row of the table has the `b_op` a 32-bit form of 6, 9 or 14 would need,
since every op is below 2^10. The byte chains are small Nat lemmas (`ltu8`,
`eq8`, `and8`), and the bridge from the lane's field columns to naturals is
shared by the three. About 5 s.

**The pilot.** [`lean/Proofs/ExtPilot.lean`](lean/Proofs/ExtPilot.lean)
generalizes `slot1_add` to `slot1_external code sem`, given every prover's
`OpProves code sem`, and instantiates it: `slot1_ltu`, `slot1_eq`, `slot1_and`.
Unlike `add`, `ltu` and `eq` set the flag, so slot 2's pc is the model's
`bif flag then pc + jmp_offset1 else pc + jmp_offset2`, both cases proved from
Main's transition. Standard three axioms; the file takes about 47 s, most of it
the case split that unfolds `zisk_execute` for the four ops.

The assumptions are the `add` pilot's: the lookups' soundness, the ROM, the
tables `make tables` checks, slot 1's pc and slot 0's `c`, and the one-slot
scope.

## Many steps

The pilot proved one step at slot 1 and assumed what the step before it left:
slot 1's pc and slot 0's `c`. Chaining steps discharges both, for every lane
and across rows.

**Every lane.** Main constrains its four lanes alike, each with its own copy
of `main.pil:456`–`503`; only the neighbours differ. `LaneHolds r l p0 p1 np`
([`Slot.lean`](lean/Proofs/Slot.lean)) is one lane's constraints as
equations, with the `c` it reads (`p0`, `p1`) and the pc it determines (`np`)
as parameters, and `lane_internal` proves the step for any lane from it. Four
lemmas derive `LaneHolds` from the generated AIR, naming each lane's
constraints and their intermediates:

| Lane | Reads `c` from | Determines the pc of |
|---|---|---|
| 0 | `previous_c`: lane 3 of the previous row (`previous_c_0`), or `segment_previous_c` on a segment's first row (`previous_c_0_first`) | lane 1 |
| 1, 2 | the lane before | the lane after |
| 3 | lane 2 | lane 0 of the next row (`laneHolds_3`, when the next row is not a segment's first), or the airval `segment_next_pc` on a segment's last row (`laneHolds_3_last`) |

The ROM lookup is generalized the same way: `romArg l` is lane `l`'s lookup
(`Main.bus86` to `bus89`), and `rom_lane` derives `SlotHolds` at any lane from
it. It also shows more than before: the lookup itself puts an instruction of
the program at the lane's pc, provided that pc is below 2^32.

**Steps chain.** [`lean/Proofs/Run.lean`](lean/Proofs/Run.lean) names a step's
place, `Pos` (a row and a lane), and when it *encodes* a model state: its pc
column is the state's pc and the `c` it reads is the state's `c`.

- `step_internal`: if position `s` encodes a state, the step there runs the
  program's instruction at its pc, which the ROM lookup finds, and `s.next`
  encodes the state `zisk_execute` leaves.
- `run_internal`: by induction, from an encoded start, the trace forces `n`
  steps of the model (`Runs prog n σ σ'`), and the position `n` steps on
  encodes `σ'`.
- `step_pilot`, `run_pilot`: the same for programs of all six modeled opcodes
  (`PilotProg`). An external op's result comes over the operation bus:
  `opArg l` is lane `l`'s lookup (`Main.bus46`, `bus59`, `bus72`, `bus85`, one
  tuple at four lanes), and `lane_external`
  ([`ExtPilot.lean`](lean/Proofs/ExtPilot.lean)) proves the step at any lane
  from `LaneHolds`, the lookup, and every prover's `OpProves`. `slot1_external`
  is now its slot-1 case. The provers' guarantee enters as `OpsProved ps`: each
  prover in the list computes all four ops, which the prover files give one by
  one.

Both runs share one induction, `run_of_step`, which turns any step lemma into
runs of any length.

The model's run is a conclusion, not a hypothesis: the trace determines which
instruction the model executes at each step, and the constraints that it
executes it as `zisk_execute` does. The scope is the pilot's, stated over the
whole program (`InternalProg`): every instruction is `copyb` or `flag`, with
immediate or `c` sources, no store and no `set_pc`, and both its jumps land in
the 32-bit pc range (`PilotProg` allows the four external ops too, and
indirect jumps, see [Indirect jumps](#indirect-jumps)). A run stays
within one segment; crossing to the next is
[Across segments](#across-segments). Standard three axioms; `Run.lean` checks
in about a second.

## Across segments

A ZisK execution is a chain of Main segments, each a Main instance of
`numRows` (2^22) rows, so `4·numRows` steps. The continuation bus (1000) links
them: a segment assumes the state it starts from, as airvals
(`segment_initial_pc`, `segment_previous_c`), and proves the state it hands on
(`segment_next_pc`, `segment_last_c`) — the generated `Main.bus714` and
`bus715`. Global updates open the chain at the boot state and close it at the
end. [`lean/Proofs/Segments.lean`](lean/Proofs/Segments.lean) joins the runs of
[Many steps](#many-steps) along that chain:

- **Entry**: `segment_initial_pc` is row 0's pc (`main.pil:661`), and lane 0
  of row 0 reads `segment_previous_c` (`previous_c_0_first`), so a segment's
  entry airvals encode its first position (`entry_encodes`).
- **Inside**: `run_within`, the run of `Run.lean` from that position, for any
  number of steps short of the last; every row but the first has
  `__L1__ = 0`.
- **Exit**: the last step, lane 3 of the last row, hands its pc to
  `segment_next_pc` (`laneHolds_3_last`) and its `c` to `segment_last_c`
  (`main.pil:532`), so the segment's exit airvals encode the state it leaves
  (`exit_step`). `step_core` ([`Run.lean`](lean/Proofs/Run.lean)) is the step
  for a lane with any next-pc value, which both uses share.
- **The chain** (`ChainHolds`): the segments, in order, each assume what the
  one before proved, and the first the boot tuple `(0, 0, 0, boot, 0, 0)`. It
  is what a sound continuation bus guarantees, stated over the generated
  arguments; the global update that writes the boot tuple is not extracted,
  so the tuple is written in the file.

`run_from_boot`: for any `n` below the segments' total steps, the model runs
`n` steps of the program from the boot state, and the position of step `n` —
segment `n / 4·numRows`, step `n mod 4·numRows` within it — encodes the state
it reaches. The start is no longer assumed. `run_from_boot_system` is the same
over the operation bus's provers of `Provers.lean`.

Besides the buses' soundness, a segment (`Segment`) assumes the setup's
`__L1__`: 1 on row 0, 0 on the others. The file checks in about two seconds,
on Lean's standard three axioms.

## Indirect jumps

An instruction with `set_pc` jumps to `c + jmp_off1`: how ZisK returns from a
call, and how the BIOS jumps to its end. Main needs no bus for it — the pc
transition computes the target as `c₀ + jmp_offset1`, with `set_pc·c₁ = 0`
(`main.pil:489`) and `flag·set_pc = 0` (`main.pil:478`) — but the target is
data, not a static offset, so no property of the program bounds it. The bound
comes from the trace instead: the next step's ROM lookup puts a 32-bit value
in its pc column (`rom_pc32`), the next segment's first lookup does for a
segment's last step (`ChainHolds.next_pc32`), and the closing global update
does at the end.

`lane_setpc` ([`Run.lean`](lean/Proofs/Run.lean)): given that bound, the
target is exactly the model's 64-bit `c + jmp_off1`. `c₁ = 0` puts `c` below
2^32, a short `jmp_off1` (`ShortJump`, 31 bits either way) keeps the sum in a
range far below the characteristic, so the field equation
`q = c₀ + jmp_offset1` is an equation between integers. `step_core` takes the
next pc column's bound as a hypothesis and handles both kinds of step, so the
runs, from boot to the end, now cover programs whose internal ops set the pc.
The runs with registers (`RegPilot`) allow `set_pc` on any op: the next pc
depends only on the lane's `c` and flag (`lane_npc`), which for an external op
the operation bus supplies, and Main's constraints force `flag = 0` and
`c < 2^32` whatever the op.

## The program's end

ZisK ends a program at `ROM_EXIT` (0x1004): a `copyb` of 0 with `is_end` set,
jumping to itself. The model stops there: `zisk_execute` returns `true` and
leaves the pc. Main does not see the flag — `is_end` is not among the ROM's
flags — so to Main the end instruction is a `copyb` that loops on itself, and
the last segment is padded with it. What ends the trace is the continuation
bus: a global update closes the chain at `(0, 1, 0, END_PC, 0, 0)`, so the last
segment's exit pc is `END_PC`.

[`lean/Proofs/End.lean`](lean/Proofs/End.lean) proves the end in two steps:

- **Main's view.** `mainView prog` is the program with every `is_end`
  cleared; the Rom encodes it as well as `prog`. `run_all_segments` runs it
  through every segment of the chain from boot, and `ChainCloses` — the
  closing tuple, written in the file since the global update is not extracted
  — pins the final pc to `END_PC`.
- **The model's run.** `runs_to_end`: the model's own run of `prog` follows
  Main's view step for step until the first end instruction it reaches,
  which it executes (`execute_end`), and stops.

`run_to_end`: from the boot state, the model runs the program, within the
steps the segments hold, to an end instruction, and ends. The scope
(`PilotProgEnd`) is the pilot's, with end instructions allowed as internal
ops. `run_to_end_system` is the same over the operation bus's provers. About 4
seconds, on Lean's standard three axioms.

## Registers: memory checking

Registers live in Main, as a linked list of accesses over the memory bus
(10), a permutation argument ("Registers on Main" in `main.pil`). Every access
to a register at memory step `s` proves the access before it,
`(3, reg, prev, 8, pv)`, with `prev < s` range-checked strictly, and assumes
itself, `(3, reg, s, 8, v)` — except the final flush of the last segment,
which closes the chain; a global update assumes the initial zero,
`(3, reg, 0, 8, 0)`.

[`lean/Proofs/MemCheck.lean`](lean/Proofs/MemCheck.lean) proves what a sound
permutation buys, for one register, in the abstract. If the proved tuples are a
permutation of the assumed ones, every access's `prev` is below its step, and
the steps are distinct, then every access proves the value of the latest
access before it — all of which assumed one — or, for the first, the initial
value (`reads_last_write`). The proof counts: at an access's step, the
permutation makes the tuples pointing below it exactly as many as the writes
below it, which forces every earlier access to be current and no later one to
point back; an access pointing past the latest write then leads to an endless
chain of later ones. Lean's core lists and `List.Perm` suffice, on the
standard three axioms.

**No AIR but Main sends a register tuple.** The chain argument needs the
tuples with op 3 to be Main's. [`lean/Proofs/MemBus.lean`](lean/Proofs/MemBus.lean),
generated by [`gen_membus.py`](gen_membus.py) (`make membus`), covers every
other argument on bus 10 — 173, in 39 AIRs — reading each op as a linear form
over the AIR's columns, and proves it is never 3 at a row that sends:

| Kind | Arguments | How |
|---|---|---|
| constant, or `c₀ + Σ cₖ·sₖ` over bits | 114 | `op_ne`, each `sₖ` a bit by the AIR's booleanity constraint |
| a sum of selectors that reaches 3 only if two are set | 16 | `op_ne_sel`: the DMA AIRs constrain the selectors' sum, `sel`, to a bit |
| a sum of the precompile's clock columns, plus 1 | 39 | `FixedOp`: 1 or 2 at every row of the setup's fixed data, checked by `make tables` |
| the Rom's data rows, sending their `flags` | 4 | `RomDataFlags`: 2 or 4, program data, checked by `make difftest` |

`make tables` evaluates the 39 fixed ops at every row of the 17 precompiles'
fixed files, and checks that it found as many per AIR as `MemBus.lean` takes
(requiring op 1 instead fails on the write rows). The exporter checks every
data row of the committed ROM; the programs difftested so far have none.
Standard three axioms.

**The bus, over every argument.** [`lean/Proofs/SumBus.lean`](lean/Proofs/SumBus.lean)
states what a sound sum bus guarantees in full, tuple by tuple: a *sender*
is one argument of one instance, sending its tuple at every row with its
count, and `SumHolds b ss gp ga` says that for every tuple, what the senders
and the global updates prove equals what they assume, in the field — what a
sound gsum argument gives. An argument whose tuple and count are constants and
air values (degree 0) is sent once per instance, not at every row: the `std`
library moves it into the instance's direct sum. The extractor now records
that (`once`, from the hint's `deg_expr` and `deg_sel`); Main's flushes —
`FLUSH_COUNT = 4` windows per segment here, 39 registers each — are such
arguments.

`MemBus.lean` then lists every instance of the 39 other AIRs (`MemOthers`)
and Main's 336 arguments on the bus (`mainMemSenders`), and proves
`reg_chain_balance`: on a sound memory bus over all of them, every register
tuple balances among Main's arguments and the registers' initial tuples
alone. The global updates are not extracted; `GlobalRegs` writes their
register part, from `main.pil`: none proves a register tuple, and the ones
that assume one are `(3, r, 0, 8, 0, 0)` for `x1` to `x39`.

**From a balance to a permutation.** [`lean/Proofs/RegChain.lean`](lean/Proofs/RegChain.lean)
starts the bridge to `MemCheck`, generically: a sender whose count is a bit at
every row contributes exactly the list of tuples it sends (`count_eq_times`),
and lists whose every tuple counts the same in the field, shorter than the
characteristic, are permutations of each other (`perm_of_times`). Together,
`perm_of_balance`: a balance of bits, restricted to any set of tuples, is a
`List.Perm` of what is proved and what is assumed plus the initial tuples.

It goes on generically: a *pair* is an access kind's two arguments, the
one proving the access before and the one assuming the access, over the same
rows, with each row's memory step; when a pair is `Good` — counts are bits,
the proved tuple is `(3, addr, prev, 8, v)`, and whenever the assuming side
sends a register tuple the proving side does too, with the access's own step
— its register tuples are exactly its accesses' (`Pair.pre_sent`,
`Pair.cur_sent`). A raw access's previous step is a field element; every
proved tuple being an assumed one makes it a step's cast, and the strict range
check puts it below the access's step (`chain_of_raw`). `pairs_chain` puts it
together: good pairs whose register tuples balance, with distinct, bounded
steps and range-checked gaps, are a `MemCheck` chain.

**Main's registers.** Main has 168 pairs per segment (`PairTag`): per lane,
`a`, `b` and the store (12), and per flush window and register (156). The
generator also writes [`lean/Proofs/MainRegs.lean`](lean/Proofs/MainRegs.lean):
every pair is good (`mainTags_good`) — with count 1 and op 3 the register
selector is 1 and the memory selectors 0, so the address is the register's
(`b` through `addr1`) and the memory step the formula's — every gap is
range-checked (`mainTags_gap`), and a flush's address is its register's. A
segment's hypotheses (`MainSegOk`) are Main's constraints, its first step
`k·2^24`, and the fixed `SEGMENT_STEP` being the row index, which
`make tables` checks against the setup.

[`lean/Proofs/Registers.lean`](lean/Proofs/Registers.lean) assembles the
segments (`MainSegs`): each register's accesses over every segment form a
`MemCheck` chain (`MainSegs.reg_chain`) — distinct steps because segments,
lanes, slots and flush windows each own theirs — and so
`MainSegs.reads`: on a sound memory bus, every access to a register in Main
proves the value of the latest access to it before, or, for the first, 0.
Standard three axioms. What remains assumed for the registers: the memory
bus's soundness (`SumHolds`), its global register updates (`GlobalRegs`),
the range checks' soundness, each segment's first step (the continuation
chain's, not yet derived here), and fewer than 2^20 segments.

**Toward the runs with registers.** Two pieces are in place.
[`lean/Proofs/Execution.lean`](lean/Proofs/Execution.lean) derives each
segment's first step from the continuation chain (`ChainHolds.initial_step`:
the boot tuple starts at 0, and every segment but the last — whose
`main_last_segment` the next one's assumed tuple forces to 0 — hands on
`initial_step + 2^24`), so `MainSegOk`'s first step need not be assumed.
[`lean/Proofs/ModelRegs.lean`](lean/Proofs/ModelRegs.lean) runs the model's
step in general (`execute_gen`): for any op but `halt`, kept abstract as
`zisk_exec inst.op a b`, with immediate, `c` or register sources (`rX` of an
index other than x0) and no store or a register store (`wX`), the step's `c`,
flag, pc, step counter and register file — one lemma, checked in about two
seconds, where the per-op lemmas took 47.

**Runs with registers.** The model's register file is now carried along the
run, encoded as Main's register chain holds it.

- [`lean/Proofs/RegFile.lean`](lean/Proofs/RegFile.lean) turns
  `reads_last_write` into a function of the memory step: `regVal es s`, the
  value the latest access before step `s` wrote, or 0. An access proves
  `regVal` at its own step; one step on, a read leaves it unchanged and a
  store sets it.
- [`lean/Proofs/RegRun.lean`](lean/Proofs/RegRun.lean) places Main's accesses:
  a lane's `a`, `b` and store accesses take memory steps `≡ 1, 2, 3 (mod 4)`
  and the flushes `≡ 0`, and every access but a store's is a read
  (`mainTags_read`, generated per pair). So a lane's source access proves the
  register's value at its step (`lane_read`), its store sets the register it
  names (`store_set`) and keeps every other (`store_none`), and every other
  step keeps them all (`reads_keep`).
- [`lean/Proofs/RegStep.lean`](lean/Proofs/RegStep.lean) states the
  encoding: `RegInv M xs b`, the model's `x1`–`x31` are the chain's values at
  memory step `b`. At a lane whose memory steps start at `b`, a register
  source's value is in the lane's `a` or `b` columns (`reg_srcs`), and after
  the step `RegInv` holds at `b + 4` for the register file the model's store
  leaves (`reg_update`). The value Main stores, `store_pc`'s `pc + jmp_offset2`
  or `c`, is the model's.
- [`lean/Proofs/LaneGen.lean`](lean/Proofs/LaneGen.lean) and
  [`lean/Proofs/StepRegs.lean`](lean/Proofs/StepRegs.lean) give the lane over
  any source columns, and with `execute_gen` one step of the register-aware
  pilot (`RegProg`: the six ops, immediate, `c` or register sources, no store
  or a register store; `step_gen`), given what the register sources put in
  the lane's columns — which `reg_srcs` supplies.
- [`lean/Proofs/RegBoot.lean`](lean/Proofs/RegBoot.lean) runs it.
  `run_from_boot_regs`: along a chain of segments from the boot state, with
  every register 0, every step the segments hold is a step of the model, and
  after `n` steps the model's registers are the chain's at memory step
  `1 + 4n`. `run_from_boot_mem` derives the chain's facts from a sound memory
  bus: the segments' `MainSegs.Ok` comes from the continuation chain
  (`initial_step`), `SEGMENT_STEP`, the range checks, and at most `2^20`
  segments.

- [`lean/Proofs/RegEnd.lean`](lean/Proofs/RegEnd.lean) takes it to the
  end. `run_to_end_mem`: along a chain that closes at the end pc, the model
  runs the program to its first end instruction, the registers it reaches it
  with after `m` steps are the chain's at `1 + 4m`, and it executes the end
  instruction — now any op, source and store the pilot has, through
  `execute_gen_end` — with the register file its store leaves, and stops.
  The model's run follows Main's (`runs_to_first_end`), and runs are
  deterministic (`Runs.det`), so the state it stops at is the one
  `run_from_boot_regs` describes.

Standard three axioms. What the runs with registers assume, besides
`run_from_boot`'s: the memory bus's soundness and its global register updates,
the range checks of the gaps, `SEGMENT_STEP`, that the segments are the memory
bus's Main instances, and the register-aware pilot's scope.

## Memory: sorted checking

Memory is checked differently from registers. Main assumes each load and
store on the memory bus, `(1 or 2, addr, step, width, v₀, v₁)`, and the Mem
AIR proves them from a log sorted by address, then by step, continued across
its segments by its own continuation bus. ZisK instantiates it three times —
the RAM (mutable, with dual reads, four lanes a row), `InputData` (free
input) and `RomData` (written once) — and `MemAlign` turns narrow or
unaligned accesses into aligned ones.

[`lean/Proofs/MemSort.lean`](lean/Proofs/MemSort.lean) proves what the
sorted log buys, in the abstract. A lane holds an access and possibly a dual
read of the same address and value at a later step. Against the lane before
it, a lane either keeps the address — its step no lower than that lane's
last, strictly higher for a write, a read returning the previous value — or
moves to a higher address, where a read returns the initial value. Then every
read the log emits returns the latest write to its address at or before its
step, or the initial value (`MemSort.reads_last_write`). The proof walks the
log lane by lane, keeping three facts: every access so far is at or below the
last lane's address and, at it, at or below its last step; the last lane's
value is its address's current one; and every read so far returns the latest
write.

[`lean/Proofs/MemRam.lean`](lean/Proofs/MemRam.lean) shows the RAM is such
a log. Each of a row's four lanes is checked against the lane before it —
lane `L-1` of the row, lane 3 of the row before, or on the first row the
previous segment's air values — and the generated constraints give a uniform
set of facts per lane (`RamLaneHolds`). The AIR fixes only differences, in the
field: the increment `l_increment + 2^22·h_increment + 1`, range-checked into
`[1, 2^38]`, is the address's step up when it changes and the step's when it
does not; a dual read's distance is range-checked too. With every lane's
address and step a natural below `2^40` (`RamNat`), those relations are
relations between naturals, and the lane `Follows` the one before
(`follows_cols`, `RamNat.lane_follows`). So a first segment's lanes are a
sorted log (`sorted_first`), a later segment's extend the log from the lane
its air values name (`sorted_next`), its last lane is what its exit air
values hand on (`last_lane`), and a chain of segments linked by the
continuation bus (`RamChain`) makes one sorted log, where every read returns
the latest write or 0 (`ram_reads_last_write`). Nothing unfolds a segment's
2^24 lanes: the run of lanes is stated for any length (`sorted_run`).

[`lean/Proofs/MemInput.lean`](lean/Proofs/MemInput.lean) does the same for
the input memory, `InputData`: read-only, one lane a row, sorted by address
alone, a lane keeping the value of the one before unless its address changes
— a step up of `1` plus a range-checked 24-bit distance — or it is a free
read, which the AIR allows only at the region's base. In the abstract
(`MemImm.imm_consistent`), along such a sequence two lanes at the same
address, other than the base, hold the same value; InputData's rows, over a
chain of segments linked by their continuation values (`InChain`), are such a
sequence (`input_consistent`). So the input memory is a function of the
address — the model's initial memory for the region.

The addresses need no help from the bus. The AIR range-checks the first one's
distance from the region's base and the last one's from its end, and in
between an address grows only by range-checked steps; so each is a natural,
built up from the first (`InTrace.A`, `addr_eq`), and lies in the region,
bytes `[0x4000_0000, 0x7FFF_FFFF]` (`A_region`). `InTrace` holds only what
the AIR gives — its constraints, `__L1__` and its range checks — and
`InTrace.toSeg` turns it into the segment `input_consistent` takes. The steps play no part, and the
segments are indexed as one sequence, `m ↦ row m mod numRows of segment
m / numRows`, with no list built.

Still to come: the bounds `RamNat` assumes — the RAM's addresses can be built
up the same way, while its steps need the memory bus, every lane's tuple being
one some AIR assumes with a bounded step; the continuation buses' guarantees
(`RamChain`, `InChain`); the memory bus's census for
loads and stores, as `MemBus.lean` did for register tuples; `MemAlign`; and
Main's memory sources and stores against the model's byte-addressed
`read_mem` and `write_mem`.

## The operation bus's provers

`run_pilot` takes the operation bus's provers as a list and assumes they all
compute the four external ops (`OpsProved`). The prover files prove that one
argument at a time; [`lean/Proofs/Provers.lean`](lean/Proofs/Provers.lean)
collects them, so the run holds over the provers ZisK actually has.

- **The census** is generated from the AIRs: every argument, in every AIR but
  Main, that proves on bus 5000. There are 61, in 39 AIRs, plus the 12 of the
  FROPS table (`VirtualTableZisk0`). The generator stops if one of them has no
  lemma for one of the four ops.
- **An instance** of an AIR (`<Air>Inst`) is its trace, its constraints at
  every row, and the soundness of the buses it assumes itself: the adders'
  range checks, Binary's byte lookups, Arith's and BinaryExtension's table
  lookups. A `System` lists any number of instances of each AIR, and the two
  virtual tables.
- **The tables' contents** enter as `Tables` (the Binary table's rows, and the
  Arith and BinaryExtension tables never serving the four ops) and `FropsOk`
  (each FROPS row carries its op's result), exactly what `make tables` checks.

`System.ok` proves every prover of every instance computes the four ops, and
`run_system` is `run_pilot` over `System.provers`, with `OpsProved` replaced by
the tables' contents. The file is generated by
[`gen_provers.py`](gen_provers.py) (`make provers`) from the prover lemmas'
signatures, each instance carrying the union of its lanes' hypotheses, and
checks in about 4 seconds, on Lean's standard three axioms.

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
   `flag`, immediate and `c` sources. Since extended to every lane and to runs
   within a segment; see [Many steps](#many-steps).
4. **Extract the buses — in progress.** Teach
   [`pilout-constraints`](../tools/pilout-constraints/README.md) to read the
   lookup and permutation arguments among the pilout's hints, so the facts the
   pilot assumes or cannot reach come from the generated AIR. In order of what
   they unlock:
   - the **ROM lookup** — done: every bus argument is extracted, and
     `SlotHolds` is derived from the ROM lookup; see
     [The ROM lookup](#the-rom-lookup);
   - the **operation bus**, which ties an external op in Main to the AIR that
     computes it, so `add`, `eq`, `ltu` and the rest can be proved. For
     `add`: the census of its provers is done, the FROPS tables and the
     Arith and BinaryExtension tables are checked, every dedicated adder is
     proved to add (21 lanes), Binary is proved to add (7 lanes), and all 33
     provers that cannot prove `add` are proved not to (see
     [Who can prove an `add`](#who-can-prove-an-add)), and the `add` pilot
     is proved on top (see [The `add` pilot](#the-add-pilot)). The same is
     done for `ltu`, `eq` and `and` (see [`ltu`, `eq` and `and`](#ltu-eq-and-and)),
     which with `add` are every external op the model has, and every
     prover's proofs are collected over every instance of its AIR (see
     [The operation bus's provers](#the-operation-buss-provers)); the rest
     come with the opcodes of step 5;
   - the **memory bus**, for memory and register sources and stores —
     registers done: from boot, with register sources and stores, the
     model's registers are Main's register chain's at every step (see
     [Registers: memory checking](#registers-memory-checking)); memory next;
   - the **range checks**, for limbs known to be below `2^32` — stated
     (`RangeHolds`, with each AIR's ranges generated) and used by the adders.
5. **Fill in the opcode table**, tracked by `make ops`, with each new opcode
   checked by `make difftest` on a program that executes it
   (`make difftest PROG=… INPUT=…`). Turn on `--strict` once coverage is
   complete.
6. **The full correspondence.** Extend the run from boot to end of
   [The program's end](#the-programs-end) to every source, store and opcode,
   up to: a trace every
   AIR accepts is a run of `zisk_execute`. Neither side is transcribed by hand,
   so neither can drift from what ZisK ships without the generators noticing.
7. **An `assembly` mapping** (optional). Sail mappings are bidirectional, so
   one definition yields both an assembler and a disassembler — which would
   subsume the round-trip test listed as a TODO in `ziskasm/README.md`
   (currently `src/assembler.rs` and `ZiskInst::to_zisk_asm` are two
   hand-written implementations of the same relation).

Out of scope throughout: that the witness computation fills the columns the
constraints describe, and the soundness of the STARK backend.
