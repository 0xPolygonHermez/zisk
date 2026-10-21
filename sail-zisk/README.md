# sail-zisk

A formal specification of the ZisK processor and instruction set in
[Sail](https://github.com/rems-project/sail), the ISA specification language
of the official RISC-V model, [sail-riscv](https://github.com/riscv/sail-riscv).
sail-zisk is to ZisK what sail-riscv is to RISC-V.

Its first use is a proof: that a RISC-V program and its ZisK transpilation
(`transpilers/riscv`) are equivalent — sail-riscv's semantics of the one
against sail-zisk's semantics of the other. One description of ZisK generates
everything that proof, and the checks before it, need:

```
                       ┌─→ C emulator         (differential-tested against ziskemu,
  model/*.sail ──sail──┤                       and op by op against ZisK's Rust)
                       ├─→ Lean 4 definitions ─┐
                       └─→ Rocq / Isabelle      │  (if ever wanted)
                                                ├─→ lean/: the equivalence proofs
  sail-riscv ────sail──→ Lean 4 definitions ───┘
```

## What Sail is, and why the ISA is not written in Lean directly

**Sail is a language for writing down an instruction set**, developed by the
[REMS](https://www.cl.cam.ac.uk/~pes20/rems/) project at Cambridge. A Sail
model reads roughly like the pseudocode in an ISA manual — registers, memory
effects, a function per instruction — but it is typechecked rather than prose:
bitvectors carry their length in their type, and the arithmetic side
conditions that produces are discharged by an SMT solver (z3) while the model
is checked. Besides RISC-V, there are Sail models of Arm-A and of CHERI.

Sail is a *specification* language, not a logic: it has no theorems, no
proofs, and no tactics. What it has instead are backends. It emits C,
SystemVerilog, OCaml, Lean 4, Coq/Rocq, Lem (and through it Isabelle/HOL and
HOL4), SMT problems, and documentation — every one of them from the same
source text.

**Lean is where the proving happens.** `sail --lean` turns each Sail function
into a Lean definition in the `SailM` state monad the backend generates — the
model's operand fetch, for instance:

```lean
def source_a (inst : zisk_inst) : SailM (BitVec 64) := do
  match inst.a_src with
  | A_C => readReg Zc
  | A_REG => (pure (rX (Sail.BitVec.extractLsb inst.a_imm 4 0)))
  ...
```

sail-riscv is generated to Lean by the same backend, so both sides of the
equivalence are machine-generated definitions, and the proof relating them is
hand-written Lean.

So the question is not Sail *or* Lean; it is whether the ISA side should be
hand-written Lean or generated from Sail. Five reasons it is Sail:

1. **A specification has to be validated, not just written.** A proof about
   the wrong description of ZisK is worth nothing, and no proof can tell you
   the description is wrong. Only running it can. `sail -c` compiles the model
   to a C emulator fast enough to execute real programs and be differential-
   tested against `ziskemu`, and to check every operation against ZisK's own
   op functions — so the spec is checked against the machine before anything
   is proved about it. Hand-written Lean gives no such check.
2. **One source, several artifacts, no drift.** The emulator, the Lean
   definitions and the documentation are all the same text compiled
   differently, so they cannot disagree.
3. **ISA-shaped types, checked by the compiler.** `ind_width` is `{1, 2, 4,
   8}` in the model, so an illegal indirect width is a type error at compile
   time where the Rust emulator has a runtime `panic!` — and `a_src`/`b_src`
   being separate types makes the asymmetry between them unrepresentable. In
   Lean the same constraints become proof obligations to carry through every
   lemma.
4. **The spec is the top of the trust chain, so it has to be readable.**
   Nothing above it is verified; it is trusted because people read it and
   agree it describes ZisK. Sail is readable by a ZisK engineer or an external
   auditor who does not know Lean.
5. **RISC-V is already in Sail.** The equivalence is with sail-riscv, the
   official RISC-V model. Writing ZisK in the same language, laid out the same
   way, puts both sides through the same backend into the same Lean, and keeps
   the two models readable side by side.

### What it costs

**The Sail → Lean backend joins the trusted base.** Nothing proves that the
Lean it emits means the same thing as the C it emits from the same source:
the no-drift argument buys a common origin, not a proof that two backends
agree. The Lean backend is also the newest of them, and the least exercised.
Upgrading to Sail 0.20.3 found one such disagreement: 0.19.1's library mapped
the arithmetic right shift to a *rotation* in Lean (see
[Toolchain versions](#toolchain-versions)).

**The generated Lean is machine-shaped.** It is monadic and verbose, and not
what anyone would write by hand to prove things about. Proofs work against
definitions this repository does not control, and a backend change can move
them — which is why the versions are pinned.

**Three toolchains instead of one.** Sail through opam, Lean through elan,
and CMake for sail-riscv's build.

## Scope

The ZisK instruction set as `core/` defines it:

- the instruction record (`ZiskInst`): sources a and b, the op, the store,
  and the jumps;
- every operation of `ZiskOp` (`core/src/zisk_ops.rs`), the precompiles as
  pure Sail functions, fcall and free input as an oracle;
- the memory map: ROM, input, RAM, and the registers mapped into memory.

On the RISC-V side, the profile the transpiler supports: RV64IMA with
compressed instructions, machine mode only, no interrupts, `ecall` as a ZisK
syscall, and the CSRs the transpiler emulates. F and D are out of scope: ZisK
runs them through a software float library, for Go compatibility only.

## Status

**Early.** 91 of the 129 operations are specified — every one that is a pure
function of a and b: the internal ones, Binary, BinaryE and Arith
([`doc/ops.md`](doc/ops.md)). Each is checked against ZisK's own op functions
on 320 vectors (`make optest`). Still to come: the precompiles, DMA and fcall,
which work on memory; memory accesses other than 8-byte ones; and the
equivalence proofs, whose shape depends on how the transpiler is represented
— a hand-written spec, differentially tested against the Rust, or the Rust
itself translated to Lean — which is still to be decided.

The model runs: compiled to C, it executes `ziskasm/examples/doubler-min` —
BIOS, launcher and program, 245 steps — and `test/programs/halt`, and
`make difftest` checks every step against `ziskemu`. Both models' Lean builds,
side by side, in `lean/`.

An earlier model of 8 operations, and Lean proofs that ZisK's circuits
implement it, lived in `../sail/`; they are at the git tag `sail-air-v0`, for
when the proofs move to this model.

## Layout

As sail-riscv's:

| Path | What |
|---|---|
| `model/zisk.sail_project` | The modules and their order. |
| `model/prelude.sail` | Bitvector helpers, with sail-riscv's names. |
| `model/zisk_types.sail` | Register indices, sources and stores. |
| `model/zisk_regs.sail` | The architectural state and the register file. |
| `model/zisk_mem.sail` | Memory. |
| `model/zisk_insts_begin.sail`, `zisk_insts_<family>.sail`, `zisk_insts_end.sail` | The operations, a file per family: `internal`, `binary`, `binary_ext`, `arith`, `system`. |
| `model/zisk_inst.sail` | The instruction record. |
| `model/zisk_rom.sail` | The default ROM, with no program; `make ROM=…` gives another. |
| `model/zisk_step.sail` | The execution step. |
| `model/main.sail` | The C emulator's main loop and trace. |
| `lean/` | The equivalence proofs: a Lake package importing both generated models. |
| `test/` | The per-operation checks (`make optest`), and `programs/`, the ZisK assembly programs `make difftest-all` runs besides the doubler. |
| `doc/ops.md` | Every ZisK operation: whether the transpiler emits it, whether the model specifies it. |
| `check_ops.py` | The opcode-name guard (`make ops`). |

## Building

This section is the per-target reference; for the whole path from a fresh
clone, with the output each command produces, see
[Verifying from scratch](#verifying-from-scratch).

```sh
make check         # typecheck the model
make ops           # check the model's opcode names against core/src/zisk_ops.rs
make optest        # check every operation against ZisK's own op functions
make opsdoc        # regenerate doc/ops.md
make emu           # build the C emulator (ROM=… for a program)
make difftest      # run PROG (or ELF=…) on ziskemu and on the C emulator, and diff the traces
make difftest-all  # every program the model is difftested on
make lean          # generate Lean definitions into build/lean
make lean-build    # elaborate them
make riscv         # fetch sail-riscv at the pinned commit and generate its Lean
make riscv-lean    # elaborate it (~12 min)
make equiv         # build lean/, both models side by side
```

Every target that runs Sail first checks that the installed Sail is the
pinned version, and stops with the command to install it otherwise.

The model's modules are in `model/zisk.sail_project`, as sail-riscv lays out
RISC-V in `riscv.sail_project`. A module lists what it `requires` — every
module whose definitions it uses, since requirements are not transitive — and
Sail orders the files accordingly. Sail typechecks every module of the project
whichever is selected; selecting one (`--project … zisk_step`) scopes what a
backend emits, which is how the Lean output leaves out the C emulator's main.

## Verifying from scratch

> **None of this proves the equivalence yet.** What the run below exercises
> is the *scaffolding*: the ZisK model typechecks, runs, and agrees with
> ZisK op by op and step by step; and both models' Lean builds, side by side,
> ready for the proofs. The theorem — that a RISC-V program and its ZisK
> transpilation have the same semantics — does not exist yet. Update this
> note when it does.

Nothing below is assumed already built.

### What you need

| Tool | For | Install |
|------|-----|---------|
| Rust | the workspace, `ziskemu` and `tools/sail-difftest` | [rustup](https://rustup.rs); `rust-toolchain.toml` pins the version |
| Sail 0.20.3 | both models | `opam install sail.0.20.3` (the Makefile checks the version) |
| z3 | Sail's typechecker | `apt install z3`, or what Sail's opam install pulls in |
| A C compiler, GMP, zlib | the C emulators | e.g. `apt install build-essential libgmp-dev zlib1g-dev` |
| CMake | sail-riscv's build | 3.20 or later |
| Python 3 | `check_ops.py`, the per-op checks, `doc/ops.md` | any 3.x |
| elan | Lean 4 | [lean-lang.org/install](https://lean-lang.org/install/). No `elan default` needed: each `lean-toolchain` picks v4.29.0, and elan fetches it on the first `lake build`. |

### The run

**0. Clone, and check out this branch.** The model lives on `feature/sail`
until it merges.

```sh
git clone git@github.com:0xPolygonHermez/zisk.git && cd zisk
git checkout feature/sail
cd sail-zisk
```

The remaining commands run from `sail-zisk/`.

**1. Typecheck the model.** Silent on success — Sail prints nothing when it
has nothing to say.

```sh
make check
```

**2. Check the opcode names against Rust.**

```sh
make ops
```

```
Rust ops: 129   Sail clauses: 91   modeled: 91/129 (70%)

UNMODELED -- 38 Rust ops with no Sail clause:
  add256, arith256, arith256_mod, arith384_mod, babyjubjub_add, ...

OK: no stale opcodes.
```

The UNMODELED list is informational and shrinks as the model fills in; the
last line is the one that matters. A **STALE** entry is the real failure — it
means the model describes a machine ZisK no longer ships. See
[Why `check_ops.py` exists](#why-check_opspy-exists).

**3. Check every operation against ZisK's own op functions.** The first run
builds `tools/sail-difftest`; after that, about a minute, most of it the C
compile of the 28 800 checks.

```sh
make optest
```

```
wrote build/optest/vectors.txt: 28800 vectors for 90 operations
28800 checks for 90 operations: add add_u_w add_w and andn bclr bext ...
0 operations the model does not specify yet:
OK: every operation agrees with the emulator on every vector
```

On a mismatch it prints each failing vector — the op, a, b, and both
results — and fails.

**4. Run the model against ziskemu.** A few seconds, most of it the C
compile.

```sh
make difftest-all
```

```
wrote build/difftest/doubler-min: 44 instructions, 8 opcodes (add, and, copyb, eq, flag, halt, ltu, pubout), 245 steps on ziskemu
OK: 245 steps and the output agree with ziskemu
wrote build/difftest/halt: 34 instructions, 6 opcodes (add, copyb, eq, flag, halt, pubout), 15 steps on ziskemu
OK: 15 steps and the output agree with ziskemu
```

On a divergence it prints the first lines of the diff and fails; the two
traces are left in `build/difftest/<prog>/`.

**5. Generate and elaborate the model's Lean.** The first build fetches the
Lean toolchain and Sail's Lean library (`lake update`); after that, about 20 s.

```sh
make lean
make lean-build
```

```
Build completed successfully (28 jobs).
```

**6. Generate and elaborate sail-riscv's Lean,** at the pinned commit, into
`build/sail-riscv`. The generation takes about 17 minutes — most of it z3 —
and the build about 12.

```sh
make riscv
make riscv-lean
```

```
Building Lean definitions from Sail model (rv64d)
...
Build completed successfully (141 jobs).
```

**7. Build both models side by side.**

```sh
make equiv
```

```
ℹ [159/161] Built Equiv.Models (1.0s)
info: Equiv/Models.lean:13:0: Zisk.Functions.zisk_execute : Zisk.zisk_inst → Zisk.SailM Bool
info: Equiv/Models.lean:14:0: LeanRV64D.Functions.execute : LeanRV64D.instruction → LeanRV64D.SailM LeanRV64D.ExecutionResult
✔ [160/161] Built Equiv (1.0s)
Build completed successfully (161 jobs).
```

Seconds, since both models are already built: `lean/` reuses them and the
one copy of Sail's Lean library they share. `lean/Equiv/Models.lean` imports
both and names the two functions the equivalence relates, `Zisk.Functions.zisk_execute` and
`LeanRV64D.Functions.execute`. The proofs will go next to it.

### What a clean run establishes

- The model is well typed, in the Sail typechecker's sense.
- Its operations are ZisK's: every one the model specifies agrees with
  ZisK's Rust op functions on every vector, and every name it uses is one
  ZisK has.
- Its step is ZisK's, on the programs difftested: every step agrees with
  ziskemu, pc, c, flag and registers.
- Both models elaborate in Lean, against one shared copy of Sail's Lean
  library — so the equivalence can be stated.

### And what it does not

- The equivalence, which is still to be written.
- That the tests cover the model: the per-op vectors are a sample, and the
  difftested programs exercise a fraction of the ISA.
- That the Lean backend agrees with the C backend the tests run.

## Toolchain versions

| Tool | Version | Why |
|---|---|---|
| Sail | 0.20.3 | sail-riscv's Lean needs it: 0.19.1 sends an unexpanded type synonym (`nfields_range`) to the SMT solver, and its library maps `sail_arith_shiftright` to `BitVec.rotateRight` in Lean |
| sail-riscv | `92079109` (2026-10-06) | The last commit 0.20.3's Lean backend handles: the next adds Sdtrig, whose `option((trigger_index, tdata))` register it rejects; upstream's Lean CI builds Sail from master |
| Lean | v4.29.0 | What Sail 0.20.3's output pins |
| lean-sail | `v6` | Sail's Lean library, which both generated models require |

Move the sail-riscv pin (`SAIL_RISCV_REV`) only together with the Sail
version (`SAIL_VERSION`) the commit requires, in `cmake/sail_required_version.txt`.

## Notes on modeling ZisK versus RISC-V

A few things differ from sail-riscv in ways worth knowing before adding
operations.

**There is no `encdec` mapping.** A ZisK instruction is never decoded from a
bit pattern — the transpiler and the assembler emit it already decoded, and
the ROM stores it as a record keyed by address. So each operation needs *one*
clause here (`union clause zisk_op`, `function clause zisk_exec`), where a
RISC-V instruction needs three (`ast`, `encdec`, `execute`).

**Every instruction has the same shape.** `(c, flag) = op(a, b)`, with the
sources, the store and the jumps around it common to all. So there is one
step function, and the per-operation content lives entirely in `zisk_exec`.

**There is no hardwired-zero register.** RISC-V models `x0` as always-zero in
the register accessor. ZisK does not need to: both `ZiskInstBuilder::src_a`
and the assembler rewrite `r0` to `SRC_IMM 0` at encode time, so `a_src =
A_REG` implies the index is non-zero. `rX` and `wX` assert it rather than
assume it.

**The source enums are split.** `a_src` and `b_src` are separate types so
that the asymmetry — `STEP` is a-only, `IND` is b-only — is a *type error*
rather than the runtime `panic!` the Rust emulator uses for the same
condition.

**`ind_width` is existentially typed.** It is `{1, 2, 4, 8}`, not a plain
integer, so the legal indirect widths are enforced statically. One
consequence to watch for: each *projection* of the field yields a fresh type
variable, so two reads of `inst.ind_width` cannot be proven equal. Bind it
once with a `let` and use that binding — see `store_c` in
`model/zisk_step.sail`.

**The flag is ZisK's.** Division and remainder set it when dividing by zero,
which RISC-V has no counterpart for; comparisons set it with `c = 1`. The
operations follow `core/src/ops_core.rs`, not the RISC-V manual, even where
they implement a RISC-V instruction.

## Registers and memory

`model/zisk_regs.sail` and `model/zisk_mem.sail` fix three decisions, each
taken from what the encoder and the emulator already do rather than chosen
freely.

**The register file is `x1`–`x31`.** ZisK maps registers at `REG_FIRST =
SYS_ADDR`, but an instruction never reaches the first 32 through memory. The
encoder (`ZiskInstBuilder::src_a`, `src_b`, `store` in
`core/src/zisk_inst_builder.rs`) turns register 0 into `SRC_IMM 0` or no
store at all, registers 1–31 into `SRC_REG`/`STORE_REG`, and the "virtual
registers" 32–63 into `SRC_MEM`/`STORE_MEM` at `REG_FIRST + 8·i`. So
registers 32–63 are plain memory, above `REG_LAST`, and the register file is
the 31 registers the main trace holds: a `vector(32, bits(64))` whose slot 0
is never used.

**Memory is a little-endian byte map, apart from the registers.** `read_mem`
and `write_mem` go through Sail's `read_ram`/`write_ram` builtins, which
Sail's C runtime implements for the C emulator and its Lean library over
`SequentialState.mem`. The emulator keeps `x0`–`x31` in an array of their
own, and `Mem` `debug_assert`s that no memory access lands on an aligned
register address. The model asserts the stronger condition: no access
overlaps `[REG_FIRST, REG_LAST + 8)` at all.

**The ROM is per program.** An instruction is a record, not bytes, so
`rom_fetch` is a function from `pc` to `zisk_inst` rather than a memory read.
`model/zisk_rom.sail` is a ROM with no instructions, so the model builds with
no program at all, and a program supplies its own file (`make ROM=…`, a path
relative to `model/`). The step is split so that proofs need no ROM:
`zisk_execute(inst)` runs one instruction, and `zisk_step()` is
`zisk_execute(rom_fetch(PC))`.

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
says what ZisK actually does, which no proof can provide: a proof only
relates the model to another model, so the model has to be right first.

The pipeline:

1. [`tools/sail-difftest`](../tools/sail-difftest/src/main.rs) builds the ROM
   — assembling `PROG` with the same code `ziskemu -z` uses, or transpiling
   `ELF` with the same code `ziskemu -e` uses — and writes three things into
   `build/difftest/<name>/`. `rom.sail` holds every instruction of the ROM,
   BIOS included, as a `rom_fetch`. `mem/` holds the initial memory sections:
   the ROM's data and the input behind its 8-byte header, as
   `Emu::create_emu_context` lays them out. `ziskemu.trace` is the reference
   run, recorded by stepping `Emu` directly.
2. The model is compiled to C with that ROM and `model/main.sail`, and run
   with the memory sections loaded through `-b addr,file`.
3. The two traces are diffed.

Each line of the trace is a step: the pc it ran at, `c`, `flag`, and every
register the step changed. After the last step comes how the run ended and
the output region. `a` and `b` are not traced, because they are locals of
`zisk_execute`, and exposing them would mean adding state the Lean side would
carry into every proof. A wrong source still shows up, in `c` or in a
register, at the step that uses it. A wrong memory write shows up when the
value is read back, or in the output.

The exporter refuses what the model does not describe, rather than exporting
something the model would get subtly wrong: an instruction that uses the
stack pointer or takes extended arguments (the precompiles), or a source or
store kind with no Sail counterpart. An operation with no Sail clause gets
through the exporter, then fails the Sail typecheck with the operation's
name.

## Per-operation testing

A program exercises the operations it happens to use. `make optest` checks
each operation on its own:

1. `sail-difftest ops` writes test vectors for every operation that is a pure
   function of a and b: the bounds of the signed and unsigned ranges at 8,
   16, 32 and 64 bits, pairwise, and 64 random pairs from a fixed seed — 320
   per operation — each with the `(c, flag)` ZisK's own op function returns
   (`ZiskOp::call_ab`).
2. [`test/gen_op_checks.py`](test/gen_op_checks.py) turns them into Sail: a
   `check_op` call per vector, for every operation the model specifies.
3. The checks are compiled to C against the model's `zisk_insts` module alone
   and run; each mismatch prints the vector and both results.

It catches what it should: replacing `add` with `or` fails 209 vectors, the
first at `a = b = 1`.

## Why `check_ops.py` exists

The ZisK opcode table is referenced **by name, as a string**, from places no
single compiler checks together: the `.zisk` assembly sources under
`ziskasm/`, and this model. A rename in `zisk_ops.rs` breaks them silently.

This is not hypothetical. Renaming `blake2` → `blake2b` upstream produced a
*textually clean* merge into `feature/ziskasm` that still compiled — the
breakage only surfaced when the `zisklib` assemble test ran. `make ops` makes
that class of failure immediate:

- **STALE** — the model names an operation that no longer exists in Rust.
  Hard error: the model is describing a machine we do not ship.
- **UNMODELED** — an operation in Rust with no Sail clause. Informational
  while the model is being filled in; `--strict` turns it into an error once
  coverage is meant to be complete.

Run it in CI alongside the typecheck.
