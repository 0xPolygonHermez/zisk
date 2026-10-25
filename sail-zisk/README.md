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

## What we are proving

For every RISC-V program in ZisK's profile, running it as RISC-V and running
its transpilation on ZisK end the same way: the same exit and the same public
output.

```
∀ P in the contract:
  end( ZiskRiscv(P) )  =  end( sail-zisk( T(P) ) )
```

- **ZiskRiscv** is RISC-V as ZisK runs it: sail-riscv (the official RISC-V
  model), plus the documented ZisK extras (start, exit, precompile and fcall
  CSR calls).
- **sail-zisk** is our formal specification of the ZisK machine.
- **T** is the transpiler. In the proof it will be a hand-written spec in
  Lean, kept in step with the Rust by a CI check (see the to-do list below);
  the proofs so far take the transpiler's output as a hypothesis.

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
- every operation of `ZiskOp` (`core/src/zisk_ops.rs`), the precompiles by
  their mathematical definitions, fcall and free input as an oracle;
- the memory map: ROM, input, RAM, and the registers mapped into memory.

On the RISC-V side, the profile the transpiler supports in its default
build: RV64IMA with Zba, Zbb, Zbs and Zbkb, Zicsr and Zifencei, machine mode
only, no interrupts, `ecall` as a ZisK syscall, and the CSRs the transpiler
emulates. Compressed instructions are out: the transpiler's `compressed`
feature is off by default, to match the EF RISC-V target standard
(IALIGN = 32), and a 2-byte instruction transpiles to an error. F and D are
out of scope: ZisK runs them through a software float library, for Go
compatibility only. [`riscv/zisk_profile.py`](riscv/zisk_profile.py) writes
sail-riscv's configuration for this profile; see
[ZisK's RISC-V](#zisks-risc-v) for how ZisK departs from RISC-V on `ecall`
and the CSRs.

## Status

**Early.** All 130 operations are specified
([`doc/ops.md`](doc/ops.md)): every one that is a pure function of a and b —
the internal ones, Binary, BinaryE and Arith, each checked against ZisK's own
op functions on 320 vectors (`make optest`) — and the ones that work on
memory or on the free input: DMA, fcall, `jump_dest`, and the 28
precompiles — the hashes, the Poseidon permutations, the elliptic-curve and
big-integer arithmetic. So is the memory map: what may be read and what
written. The
equivalence proofs are under way (`lean/`, see the to-do list below).

The model runs: compiled to C, it executes `ziskasm/examples/doubler-min` —
BIOS, launcher and program, 253 steps — `test/programs/halt`, ziskasm's
diagnostic program, which checks every family of operations, precompiles
included, in 7 692 steps; `test/programs/memops`, the memory and fcall cases
the diagnostic leaves out; every precompile on random operands
(`test/gen_precompile_test.py`); and the RISC-V architectural tests,
transpiled: all 126 of the I, M, A, B and Zbkb tests in ZisK's profile, up
to 22 926 steps each, agree with `ziskemu` step by step, reach their end,
and leave the signature sail-riscv's C emulator leaves running the RISC-V
program itself (`make compliance`). So does a Rust guest,
`examples/fibonacci`, built with `cargo-zisk`: 2 706 steps, through fcall and
DMA. Both models' Lean builds, side by side, in `lean/` — sail-riscv's in
ZisK's profile — and so does ZisK's RISC-V
([`doc/riscv_contract.md`](doc/riscv_contract.md)) as a machine,
`lean/ZiskRiscv`: sail-riscv's step, with sail-zisk's operations for the
calls ZisK adds. Run on sail-riscv's executable Lean, it ends every
compliance test, and the fibonacci guest, as sail-zisk ends its
transpilation.

### To do

- **The transpiler in the proof, short term (option A):** a hand-written
  spec of the transpiler in Lean, the per-instruction translation as a pure
  function, kept in step with the Rust (`transpilers/riscv`) by a CI check;
  the equivalence proofs then take T(P) from it instead of as a hypothesis.
  Started: `lean/Transpiler/Spec.lean` describes RV64I but for `ebreak`
  and the CSRs, x0 forms included, Zifencei, M, Zba's sh1add, sh2add,
  sh3add and slli.uw, Zbs, and Zbb's register ops, and
  `lean/Transpiler/Bios.lean` the BIOS around them; `make transpiler-check`
  finds both equal to the Rust on every instruction they describe in the 129
  compliance tests (1 272 177 instructions, the BIOS's included) and the
  fibonacci guest (11 409). Still to do: the rest of the instruction set;
  wiring `make transpiler-check` and `make decoder-diff` into CI; and the
  whole ZisK ROM of a program from the spec, internal addresses included,
  for the top theorem (each step's ZisK instruction already follows from the
  spec: `lean/Equiv/FromSpec.lean`).
- **The decode lemmas' checking time:** each decode lemma is in its own
  module (`lean/Equiv/Decode/`, `lean/Equiv/DecodeZbs/`, `lean/Equiv/Decode/Zbb/`; the
  `lean/Equiv/RiscvDecode*.lean` modules are their bases and aggregators),
  so Lake checks them in parallel: the whole `Equiv` package, every decode
  lemma rechecked, builds in about 27 minutes on a 128-core machine (some
  11 cores busy on average), where one after another they took about two
  and a half hours. Each lemma still steps through sail-riscv's decoder one
  clause at a time (the `peel` macros), each guard refuted by `bv_decide`,
  and the slowest — a family split by cases, the ten R-type ops — bound the
  time; splitting those per op would cut it further. A faster way past the
  clauses that cannot match — a lemma per opcode that skips them together,
  say, or the decoder's clauses grouped by opcode once — would cut the work
  itself. Measured (Lean's profiler on `decode_ld`): `bv_decide` is not the
  cost; each `dsimp` the `peel` macros run after a step takes about 2 s,
  traversing the whole remaining decoder, and each `rw` 0.3–0.5 s. Dropping
  the `dsimp` after a bind step, where there is nothing to reduce, takes
  `decode_ld` from 2m22s to 1m46s; rewriting with match-reduction lemmas
  instead of `dsimp` was slower (higher-order patterns). A larger gain
  needs the decoder's term not traversed at every step.
- **The transpiler in the proof, long term (option D), not now:** rewrite
  the transpiler in Lean and ship it, so that the transpiler that runs is the
  one proved.
- **The remaining instruction forms:** A; Zba's add.uw and sh1add.uw,
  sh2add.uw, sh3add.uw (two ZisK instructions each); Zbb's unary ops (clz,
  ctz, cpop, sext.b, sext.h, zext.h, rev8, orc.b), its W forms and rori; Zbkb; and the
  CSRs (`jalr`
  with rs1 = x0, a jump to an absolute address, is in `prog_spec` now, the
  transpiler's immediate 0 for x0 as its b operand). (`ecall` is in the
  spec; it ends the run, which the link below covers. The other
  x0 forms — `mv`, `li`, `nop`, hints, `beqz`, `sd zero, …`, a load to x0 —
  are in `prog_spec`.)
- **The link to `lean/ZiskRiscv`:** the simulation stated against the
  contract machine, ending in the theorem above. Started, on sail-zisk's
  side: the BIOS's start (`lean/Equiv/BiosSim.lean`: after its six steps,
  ZisK's state is related to sail-riscv's at the contract's start) and its
  end (`lean/Equiv/BiosEnd.lean`: from related states, a return to the
  initial ra or the exit system call with code 0 ends ZisK's run in success
  with sail-riscv's memory, a code c ≠ 0 in error with code c), given the 32
  output words and the 15 input words the BIOS reads are in memory. The
  relation now includes no error so far (Zerror false). On the machine's
  side (`lean/Equiv/Machine.lean`): sail-riscv's Lean as the machine's
  RISC-V model; its step, `try_step 0 true`, the one the step lemmas are
  about once the hart is active (`lean/Equiv/RiscvTryStep.lean`); every
  instruction the simulation covers left by the machine to sail-riscv
  (not SYSTEM's opcode); whole runs (`prog_machine`: the runs of
  `prog_spec` are runs of the machine, given the exit address and the trap
  vector are not executable); and the machine's end on a return to the
  initial ra or the exit system call. Assembled
  (`lean/Equiv/Theorem.lean`: `exit_end`, `return_end`): for a program
  started as the contract says, on memory as ZisK starts it (every byte of
  its regions defined, `lean/Equiv/Full.lean`; sail-riscv never forgets a
  byte, `lean/Equiv/MemGrow.lean`), with a ZisK program that holds the
  BIOS and, at every code address (the ones the transpiler walks) whose
  word the spec describes, the spec's translation (`SpecRom`,
  `lean/Equiv/SpecRom.lean`: a hypothesis on the program; a run, which
  changes only the RAM, keeps it, given no code in the RAM), and whose run
  stays at code addresses (`RunIn`) and the simulation covers (`RunOkT`, of
  the RISC-V side: every step an instruction proved, with its accesses
  good) up to the
  exit system call or the return to the initial ra, the machine's run ends
  in `exit c` and sail-zisk's, BIOS included, in success with the same
  memory — so the same output — when c is 0, in error with code c
  otherwise. On memory as the machine is given it, sparse
  (`lean/Equiv/Sparse.lean`: `exit_end_sparse`, `return_end_sparse`), the
  machine's run ends the same way with the same output, or stops with its
  own error (a step past its 4096 retries): Sail's interpreter is monotone
  in memory (`lean/Equiv/MemMono.lean`), so a run on sparse memory, retried
  with 0 where it lacks a byte, takes the steps it takes on the memory with
  those 0s. For ZisK's memory map (`lean/Equiv/Final.lean`: `zisk_exit`,
  `zisk_return`), with the PMA regions the model's initialisation writes
  (`lean/Equiv/Regions.lean`, generated from sail-riscv's Lean by
  `riscv/gen_lean_regions.py`), what the statements take of the regions is
  proved: only the ROM is executable, so neither the exit address nor the
  trap vector is, and code is never in the RAM. The theorems rest on Lean's axioms,
  on `bv_decide`'s (it trusts its compiled checker), and on the primitives
  sail-riscv's Lean leaves undefined (softfloat, reservations, platform
  hooks), which the proofs never look into. `make transpiler-check` checks
  `SpecRom` of the Rust's ROM on its programs, for the code addresses it
  walks (the last word of a section read with no next word, where the
  machine would read whatever memory follows: it matters only if that is a
  jalr after an auipc). Still to do: the run's conditions that are not
  the program's — an access in its region, a jump's target aligned — shown
  to fail alike on both sides when they do not hold; a load or store across
  a 16-byte granule (sail-riscv's split path: one within its granule,
  aligned or not, is covered, `AlignOk`, `alignOk_gran` for ZisK's regions);
  and the run's other ends (a
  trap, a ZisK call, the instructions not yet covered).
- **zkvmcalls** (CSRs 0x850–0x8BF, the ZisK library's routines): outside the
  contract for now ([`doc/riscv_contract.md`](doc/riscv_contract.md) § 9), a
  later, separate effort.

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
| `model/zisk_insts_begin.sail`, `zisk_insts_<family>.sail`, `zisk_insts_end.sail` | The operations, a file per family: `internal`, `binary`, `binary_ext`, `arith`, `system`, `dma`, `fcall`, `evm`, and the precompiles' `bigint`, `curves`, `hash`, `poseidon`. |
| `model/zisk_precompiles.sail` | What the precompiles share: big integers in memory, arithmetic modulo a prime. |
| `model/zisk_op_attrs.sail` | Each operation's type, input and output sizes and `m32`, generated from `zisk_ops.rs` by `gen_op_attrs.py` (`make constants`). |
| `model/zisk_poseidon_constants.sail` | The Poseidon constants, generated from the Rust sources by `gen_poseidon_constants.py` (`make constants`). |
| `model/zisk_inst.sail` | The instruction record. |
| `model/zisk_rom.sail` | The default ROM, with no program, for the typecheck and the Lean build. |
| `model/zisk_rom_table.sail` | The C emulator's ROM: any program's, read from a table in memory. |
| `model/zisk_step.sail` | The execution step. |
| `model/main.sail` | The C emulator's main loop and trace. |
| `lean/` | The equivalence proofs: a Lake package importing both generated models. On the RISC-V side, `Equiv/Riscv.lean` and `Equiv/RiscvImm.lean`, sail-riscv's semantics instruction by instruction (R-type, I-type, immediate shifts, `beq`); `Equiv/RiscvMem.lean`, `Equiv/RiscvLoad*.lean` and `Equiv/RiscvStore*.lean`, its memory accesses down to the bytes (fetch, every load and store); `Equiv/RiscvDecode*.lean` and `Equiv/Decode/`, its decoder on those instructions, a lemma per module so that they check in parallel; `Equiv/RiscvStep*.lean`, a whole sail-riscv step. On the ZisK side, `Equiv/Zisk*.lean`, sail-zisk's semantics for the instructions the transpiler emits. Between them, `Equiv/MemRead.lean` and `Equiv/MemWrite.lean`, what both interpreters' memory accesses compute; `Equiv/Sim.lean`, `Equiv/StepSim.lean`, `Equiv/Block.lean` and `Equiv/Program.lean`, the simulation: one whole step on each side keeps their registers, pc and memory related, and so does straight-line code of R-type, I-type and immediate-shift instructions (`code_sim`); `Equiv/LoadSim.lean`, `Equiv/StoreSim.lean`, `Equiv/LoadWSim.lean` and `Equiv/StoreWSim.lean`, the step on every load and store (`ld`, `sd`, then the narrow ones), given the access is good when it runs; `Equiv/Run.lean`, straight-line runs of all of them (`run_sim`), each instruction good to run in the state sail-riscv reaches; `Equiv/BranchSim.lean` and `Equiv/JumpSim.lean`, the step on a conditional branch, lui, auipc, jal and jalr, both sides at the target; `Equiv/Prog.lean`, whole runs, loops included (`prog_sim`): k steps on each side from related states end related; `Equiv/FusedSim.lean`, the auipc-jalr pair the transpiler fuses into one ZisK instruction (two sail-riscv steps, one ZisK step), jalr with an odd offset, which it splits into two (one sail-riscv step, two ZisK steps), and whole runs with both (`prog_sim3`); `Equiv/ZiskSpec.lean`, `Equiv/RiscvDecodeX0.lean` and `Equiv/X0Sim.lean`, the step on every instruction above with any registers, x0 among them, its ZisK instruction the spec's translation, whichever form the transpiler gives it (a copyb for `mv` and `li`, a flag for `nop`, no store for rd = x0); `Equiv/ProgSpec.lean`, whole runs with the ZisK program the spec's translation of the RISC-V code and nothing more (`prog_spec`); `Equiv/RiscvW.lean`, `Equiv/SimW.lean`, `Equiv/RiscvDecodeW.lean` and `Equiv/WSim.lean`, the same for RV64I's word instructions (`addw`, `addiw`, `sext.w`, …); `Equiv/RiscvM.lean`, `Equiv/SimM.lean`, `Equiv/RiscvDecodeM*.lean` and `Equiv/MSim.lean`, the same for the M extension, division by zero and signed overflow included (M enabled is part of the state's readiness: misa's M bit, which no program in the contract writes); `Equiv/RiscvDecodeFence.lean` and `Equiv/FenceSim.lean`, the fences (`fence`, `fence.tso`, `fence.i`): a barrier on sail-riscv, the nop on sail-zisk; `Equiv/RiscvZba.lean`, `Equiv/RiscvDecodeZba.lean` and `Equiv/ZbaSim.lean`, Zba's sh1add, sh2add, sh3add and slli.uw (Zba enabled is part of the state's readiness: misa's B bit); `Equiv/RiscvZbs.lean`, `Equiv/RiscvDecodeZbs*.lean` and `Equiv/ZbsSim.lean`, the same for Zbs (bclr, bext, binv, bset and their immediate forms: sail-zisk's bit by index, sail-riscv's mask); `Equiv/RiscvZbb.lean`, `Equiv/RiscvDecodeZbb*.lean`, `Equiv/Decode/Zbb/` and `Equiv/ZbbSim.lean`, the same for Zbb's register ops (andn, orn, xnor, max, maxu, min, minu, rol, ror: max and min compare the other way round on each side, and sail-zisk's ror is a rotation left); `Equiv/ZiskBios.lean`, sail-zisk's semantics for the BIOS's own instructions (memory cells, the call, pubout, `end`, `halt`); `Equiv/BiosSim.lean` and `Equiv/BiosEnd.lean`, the BIOS's start and end on sail-zisk, from and to related states (`start_rel`, `rel_return_end`, `rel_exit_ok`, `rel_exit_fail`); `Equiv/RiscvTryStep.lean`, sail-riscv's step the same whatever `try_step`'s arguments, the hart active; `Equiv/Machine.lean`, sail-riscv's Lean as the model of ZisK's RISC-V machine, and the simulation's runs as the machine's (`prog_machine`); `Equiv/MemGrow.lean` and `Equiv/Full.lean`, memory as ZisK starts it, kept by a run; `Equiv/RamOnly.lean` and `Equiv/SpecRom.lean`, the ZisK program as a hypothesis on the program, kept by a run (`prog_rom`); `Equiv/Theorem.lean`, the theorem so far (`exit_end`, `return_end`); `Equiv/MemMono.lean` and `Equiv/Sparse.lean`, the same on sparse memory (`exit_end_sparse`, `return_end_sparse`); `Equiv/Regions.lean` (generated) and `Equiv/RegionFacts.lean`, ZisK's PMA regions, and `Equiv/Final.lean`, the theorem for them (`zisk_exit`, `zisk_return`). `Transpiler/Spec.lean`, the transpiler as a function in Lean, a hand-written spec, and `Transpiler/Bios.lean`, the BIOS; `TranspilerCheck/`, `transpiler_check`, which compares it with the Rust transpiler's output (`make transpiler-check`); `DecodeDiff/`, `decode_diff`, sail-riscv's decoder on every word the transpiler's decoder can tell apart, for `make decoder-diff`. `ZiskRiscv/`, ZisK's RISC-V as a machine over sail-riscv; `ZiskRiscvRun/`, the machine on sail-riscv's executable Lean, and `zisk_riscv_run`, which runs programs on it. |
| `test/` | The per-operation checks (`make optest`); `programs/`, the ZisK assembly programs `make difftest-all` runs besides the doubler, `memmap/` among them, the accesses ZisK refuses; and `compliance/`, the RISC-V architectural tests' harness for ZisK (`make compliance`). |
| `riscv/` | `zisk_profile.py`: sail-riscv's configuration for the RISC-V ZisK runs; `gen_lean_registers.py`: the register set-up of sail-riscv's Lean emulator, for `zisk_riscv_run`. |
| `doc/riscv_contract.md` | ZisK's RISC-V: what a RISC-V program means when ZisK runs it, where ZisK departs from RISC-V. |
| `doc/ops.md` | Every ZisK operation: whether the transpiler emits it, whether the model specifies it. |
| `check_ops.py` | The opcode guard (`make ops`). |

## Building

This section is the per-target reference; for the whole path from a fresh
clone, with the output each command produces, see
[Verifying from scratch](#verifying-from-scratch).

```sh
make check         # typecheck the model
make ops           # check the model's opcode names against core/src/zisk_ops.rs
make optest        # check every operation against ZisK's own op functions
make opsdoc        # regenerate doc/ops.md
make constants     # regenerate model/zisk_op_attrs.sail and model/zisk_poseidon_constants.sail
make emu           # build the C emulator, which runs any program
make difftest      # run PROG (one or more; or ELF=…) on ziskemu and on the C emulator, and diff the traces
make difftest-all  # every program the model is difftested on
make compliance    # the RISC-V architectural tests, transpiled, difftested (ARCH_TEST=…, RISCV_GCC=…)
make transpiler-check  # the transpiler's Lean spec against the transpiler, on the compliance ELFs (or TRANSPILER_ELFS=…)
make decoder-diff  # the transpiler's decoder against sail-riscv's, on every word it can tell apart (~11 min)
make lean          # generate Lean definitions into build/lean
make lean-build    # elaborate them
make riscv         # fetch sail-riscv at the pinned commit, generate its Lean in ZisK's profile (~5 min)
make riscv-lean    # elaborate it (~15 min)
make riscv-sim     # build its C emulator, for compliance's signatures (~15 min)
make riscv-run     # build ZisK's RISC-V machine on its executable Lean, and zisk_riscv_run (~15 min)
make zisk-riscv    # run ELF (prepared by make difftest) on that machine, and diff its end with sail-zisk's
make equiv         # build lean/: both models side by side, and ZisK's RISC-V machine
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
| Python 3 | `check_ops.py`, the per-op checks, `doc/ops.md`, the compliance runner | any 3.x |
| A RISC-V GCC | compiling the architectural tests | e.g. the [xPack RISC-V GCC](https://xpack-dev-tools.github.io/riscv-none-elf-gcc-xpack/) 14 (`riscv-none-elf-gcc`) |
| riscv-arch-test | the architectural tests | `git clone --branch 3.9.1 https://github.com/riscv-non-isa/riscv-arch-test` |
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

**2. Check the opcodes against Rust.**

```sh
make ops
```

```
Rust ops: 130   Sail clauses: 130   modeled: 130/130 (100%)

OK: no stale opcodes, and every opcode number agrees.
OK: model/zisk_op_attrs.sail has every operation's attributes, as zisk_ops.rs gives them.
```

It fails on a **STALE** entry — the model describes a machine ZisK no longer
ships — on a **WRONG CODE** one, an opcode the model numbers differently
from Rust, and on an **UNMODELED** one, an operation Rust has and the model
does not; and when `model/zisk_op_attrs.sail` is not what `make constants`
would generate from `zisk_ops.rs` now. See
[Why `check_ops.py` exists](#why-check_opspy-exists).

**3. Check every operation against ZisK's own op functions.** The first run
builds `tools/sail-difftest`; after that, about a minute, most of it the C
compile of the 28 800 checks.

```sh
make optest
```

Every pure operation but `profile`, which has no `op_profile` to call.

```
wrote build/optest/vectors.txt: 28800 vectors for 90 operations
28800 checks for 90 operations: add add_u_w add_w and andn bclr bext ...
0 operations the model does not specify yet:
OK: every operation agrees with the emulator on every vector
```

On a mismatch it prints each failing vector — the op, a, b, and both
results — and fails.

**4. Run the model against ziskemu.** Under a minute, about half of it
building the C emulator, once.

```sh
make difftest-all
```

```
wrote build/difftest/doubler-min: 52 instructions, 8 opcodes (add, and, copyb, eq, flag, halt, ltu, pubout), 253 steps on ziskemu
OK: 253 steps and the output agree with ziskemu
wrote build/difftest/halt: 42 instructions, 6 opcodes (add, copyb, eq, flag, halt, pubout), 15 steps on ziskemu
OK: 15 steps and the output agree with ziskemu
wrote build/difftest/memops: 117 instructions, 18 opcodes (add, and, copyb, dma_inputcpy, dma_memcmp, ...), 269 steps on ziskemu
OK: 269 steps and the output agree with ziskemu
wrote build/difftest/diagnostic: 1424 instructions, 108 opcodes (add, add256, add_w, and, andn, arith256, ...), 7692 steps on ziskemu
OK: 7692 steps and the output agree with ziskemu
wrote build/difftest/memmap-edges: 46 instructions, 7 opcodes (add, and, copyb, eq, flag, halt, pubout), 198 steps on ziskemu
OK: 198 steps and the output agree with ziskemu
wrote build/difftest/memmap-bios_read: 41 instructions, 7 opcodes (add, and, copyb, eq, flag, halt, pubout), 9 steps on ziskemu
OK: 9 steps agree with ziskemu, then both stop: ziskemu panics (Mem::read() section not found for addr: 8192=2000 with width: 8), the model asserts (memory read outside ZisK's memory map)
...
wrote build/precompiles/precompiles.zisk: 8 vectors for each of 28 precompiles
wrote build/difftest/precompiles: 3000 instructions, 35 opcodes (add, add256, and, arith256, ...), 3152 steps on ziskemu
OK: 3152 steps and the output agree with ziskemu
```

On a divergence it prints the first lines of the diff and fails; the two
traces are left in `build/difftest/<prog>/`. `test/programs/halt` must also end
the way it is written to — in error, with code 3, through the BIOS's `halt` —
or `difftest-all` fails: the program jumps to the BIOS's trap handler at a
fixed address, so a BIOS whose `mtvec` moves would otherwise make it end
normally, with both emulators still agreeing. And ziskasm's diagnostic must
report every check passed, `output[0] = 0`.

`test/programs/memmap` holds, besides `edges.zisk` — the last word of the RAM
and of the input window, a ROM constant — a program for each access ZisK
refuses: a write to the ROM or the input, by a store, by DMA and by a
precompile; a read of the BIOS's area or of the stack guard; a write past
the RAM, and a read straddling its end. Each must stop both: `ziskemu`
panics, the model asserts, after the same steps (`EXPECT_FAULT=1`).

The precompile program is generated: `PRECOMP_VECTORS` random operands for
each precompile (8 by default), from `PRECOMP_SEED` (1); each result word is
loaded into a register, so the trace compares it with `ziskemu`'s. A longer
run is `make difftest-all PRECOMP_VECTORS=40 PRECOMP_SEED=7`.

A RISC-V ELF runs the same way, with `ELF=` instead of `PROG=`, and its
input; a Rust guest needs ZisK's Rust toolchain and this checkout's
`cargo-zisk` to build it — an older `cargo-zisk` links it for an older
memory map — so `difftest-all` leaves it out. For the fibonacci example,
with n = 20 as `ziskos::io::read` expects it, an 8-byte length and the data:

```sh
cargo-zisk build --release            # in examples/fibonacci/guest
printf '\x01\0\0\0\0\0\0\0\x14\0\0\0\0\0\0\0' > fib20.bin
make difftest ELF=../examples/target/riscv64ima-zisk-zkvm-elf/release/fibonacci-guest INPUT=fib20.bin
```

```
fibonacci(20) => 6765
wrote build/difftest/fibonacci-guest: 11763 instructions, 40 opcodes (...), 2706 steps on ziskemu
OK: 2706 steps and the output agree with ziskemu
```

**5. Run the RISC-V architectural tests, transpiled.** Each test of the I, M,
A, B suites and of K's Zbkb tests is compiled for ZisK's memory map,
transpiled as `ziskemu -e` transpiles it, and difftested; and, once
`make riscv-sim` has built sail-riscv's C emulator (after `make riscv`; about
15 minutes, once), run on sail-riscv too, whose signature must be the
model's; and, once `make riscv-run` has built ZisK's RISC-V machine (step 9),
on that, which must end the test as the model does. Under a minute for all
of them.

```sh
make riscv-sim
make compliance ARCH_TEST=…/riscv-arch-test RISCV_GCC=…/riscv-none-elf-gcc
```

```
PASS  A-amoadd.d-01                    4603 steps; signature = sail-riscv's = ZisK's RISC-V machine's (536 words)
...
SKIP  B-clmul-01                       skipped: the transpiler's default build has no Zbc (its zbc feature)
...
126 of 126 tests agree with ziskemu, sail-riscv and ZisK's RISC-V machine: A 18/18, B 40/40, I 51/51, K 4/4, M 13/13; 3 skipped
```

The results are also in `build/compliance/results.txt`. `SUITES=…` picks
others: C, Zicond and Zifencei are known to the runner, and skipped for the
same kind of reason — the transpiler's default build has no compressed
instructions and no Zicond, and Fence.i's test rewrites its own code, which
ZisK's read-only ROM does not allow. See
[Compliance testing](#compliance-testing).

**6. Generate and elaborate the model's Lean.** The first build fetches the
Lean toolchain and Sail's Lean library (`lake update`); after that, about 20 s.

```sh
make lean
make lean-build
```

```
Build completed successfully (28 jobs).
```

**7. Generate and elaborate sail-riscv's Lean,** at the pinned commit, in
ZisK's profile: sail-riscv's own Lean command and unchanged sources, with
the configuration `riscv/zisk_profile.py` writes — the Lean backend fixes
the configuration when it generates — into `build/riscv-lean/Lean_ZiskRV`.
The generation takes about 5 minutes, the build about 15.

```sh
make riscv
make riscv-lean
```

```
python3 riscv/zisk_profile.py build/sail-riscv/build/config/rv64d_v256_e64.json build/riscv/zisk_rv64.json
wrote build/riscv/zisk_rv64.json: RV64 A B M Zbkb Zicsr Zifencei, M-mode only
...
Build completed successfully (141 jobs).
```

**8. Build both models side by side.**

```sh
make equiv
```

```
info: Equiv/Models.lean:13:0: Zisk.Functions.zisk_execute : Zisk.zisk_inst → Zisk.SailM Bool
info: Equiv/Models.lean:14:0: LeanZiskRV.Functions.execute : LeanZiskRV.instruction → LeanZiskRV.SailM LeanZiskRV.ExecutionResult
Build completed successfully (175 jobs).
```

Seconds, since both models are already built: `lean/` reuses them and the
one copy of Sail's Lean library they share. `lean/Equiv/Models.lean` imports
both and names the two functions the equivalence relates, `Zisk.Functions.zisk_execute` and
`LeanZiskRV.Functions.execute`. The proofs will go next to it. The build
also checks ZisK's RISC-V machine (`lean/ZiskRiscv/Tests.lean`): its
decoding of the contract's CSR sequences, and sail-zisk's keccak run on its
memory.

**9. Run programs on ZisK's RISC-V machine.** `make riscv-run` builds the
machine on sail-riscv's executable Lean (about 15 minutes, once), and
`make zisk-riscv` runs a program `make difftest` has prepared on it, from the
same memory, and diffs how it ends with sail-zisk's run of the
transpilation: the exit, the output, the dump range. The fibonacci guest,
after its difftest (step 4):

```sh
make riscv-run
make zisk-riscv ELF=../examples/target/riscv64ima-zisk-zkvm-elf/release/fibonacci-guest
```

```
2563 steps
OK: ZisK's RISC-V machine ends as sail-zisk does: exit, output
```

2 563 RISC-V steps against sail-zisk's 2 706 ZisK steps, the BIOS's
included; on the way, the guest's 2 fcall parameters, 2 fcalls and 10
memcpys run as sail-zisk's operations on the RISC-V memory.

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

**The register file is `r1`–`r39`.** ZisK maps registers at `REG_FIRST =
SYS_ADDR`: the 32 RISC-V registers, then `r32`–`r39`, which RISC-V code does
not see — the transpiler uses them as scratch, for instance in the atomics.
An instruction never reaches them through memory. The encoder
(`ZiskInstBuilder::src_a`, `src_b`, `store` in
`core/src/zisk_inst_builder.rs`) turns register 0 into `SRC_IMM 0` or no
store at all, registers 1–39 into `SRC_REG`/`STORE_REG`, and the "virtual
registers" 40–63 into `SRC_MEM`/`STORE_MEM` at `REG_FIRST + 8·i`. So
registers 40–63 are plain memory, above `REG_LAST`, and the register file is
the 39 registers the main trace holds: a `vector(40, bits(64))` whose slot 0
is never used, indexed by 6 bits.

**Memory is three regions.** As the circuits prove memory — one RAM air,
InputData and RomData, in `pil/zisk.pil` — and `core/src/mem.rs` lays it
out:

| Region | Addresses | Access |
|---|---|---|
| input | `[0x1000_0000, +2 GB)` | read-only: the free input, then the input |
| ROM | `[0x9000_0000, +128 MB)` | read-only: the program's code and constants |
| RAM | `[0xA000_0000, +512 MB)` | read-write: stack, registers, CSRs, output, data, heap |

An access outside them, or a write outside the RAM, is one no circuit
proves; `ziskemu` panics on it, and the model asserts. An access lies wholly
inside one region, or is refused. The emulator is stricter on the ROM: it
reads only the bytes the program's data sections define, where RomData
proves any address of the region; the model follows the circuits. The
tables the C emulator alone reads — the ROM table, the fcall tape, the dump
range — sit far above all three, and are read directly, not through
`read_mem`.

**Memory is a little-endian byte map, apart from the registers.** `read_mem`
and `write_mem` go through Sail's `read_ram`/`write_ram` builtins, which
Sail's C runtime implements for the C emulator and its Lean library over
`SequentialState.mem`. The emulator keeps `r0`–`r39` in an array of their
own, and `Mem` `debug_assert`s that no memory access lands on an aligned
register address. The model asserts the stronger condition: no access
overlaps `[REG_FIRST, REG_LAST + 8)` at all.

**The ROM is per program.** An instruction is a record, not bytes, so
`rom_fetch` is a function from `pc` to `zisk_inst` rather than a memory read.
`model/zisk_rom.sail` is a ROM with no instructions, so the model builds with
no program at all; that is the ROM the typecheck and the Lean build use. The
step is split so that proofs need no ROM: `zisk_execute(inst)` runs one
instruction, and `zisk_step()` is `zisk_execute(rom_fetch(PC))`.

The C emulator uses `model/zisk_rom_table.sail` instead: its `rom_fetch`
decodes the instruction at `pc` from a table of 64-byte entries in memory, at
`ROM_TABLE + 64·pc`, far above every address ZisK uses, which
`tools/sail-difftest` writes beside the program's memory sections. So one
emulator runs every program. Compiling each program's ROM into its own
emulator, as one `match` on the pc, took up to 20 minutes for a transpiled
RISC-V test of 8 000 instructions; reading it takes nothing. The table names
the op by its opcode, which is part of the model: each operation's
`zisk_op_of_code` clause, which `make ops` checks against `zisk_ops.rs`.

**Operations that work on memory are a second function.** `zisk_exec(op, a,
b)` is pure, so that a proof about an ALU op needs no state. DMA, fcall,
`jump_dest` and the precompiles are clauses of
`zisk_exec_mem(op, a, b, ext)` instead, which reads and writes memory and the
fcall state, and gets the instruction's extended argument: `jmp_off1` when
the op reads memory as a precompile (its `input_size`, in
`model/zisk_op_attrs.sail`, is not 0), else 0, as the emulator hands it
`extended_arg`; `jmp_off1` never serves as a jump then, since these ops leave
`flag` false. `zisk_execute` asks
`zisk_exec_mem` first; `None` means a pure op.

**DMA works byte by byte.** The machine splits an unaligned copy into a
head, aligned words and a tail, and proves each part separately; that is how
it proves the operation, not what the operation does, so the model copies,
compares and fills one byte at a time. A copy is a `memmove`: overlapping
ranges come out as if copied through a buffer, as `Mem::memcpy` does. So is
`jump_dest`: the machine walks the bytecode a word at a time, skipping the
words a `PUSH` covers; the model walks it a byte at a time, to the same
bitmap.

**fcall is an oracle.** A guest asks the host for values it cannot compute
cheaply — an inverse, a square root — and checks them itself, so nothing
relies on the host answering truthfully, and the model does not say how it
answers. The answers are a tape in memory, at `FCALL_TAPE`: each `fcall`
takes the tape's next entry, a length and that many words, as its result,
and the parameters `fcall_param` passed do not matter to the model. The free
input — what an 8-byte read of `INPUT_ADDR` returns, as `Mem::read` has it —
is the result's current word; `fcall_get` and `dma_inputcpy` move along it.
A proof takes the tape as part of the initial state, like the input;
`tools/sail-difftest` records it from `ziskemu`'s own answers.

**A precompile is its mathematical definition.** The emulator computes the
hashes and the field arithmetic with optimised code — arkworks, sha2, x86
assembly; the model states what they compute: the Keccak-f permutation, the
SHA-256 compression, the curve formulas over integers modulo the field's
prime, `(x * y + z) mod m` over Sail's unbounded integers. Where the emulator
panics, the model asserts: an operand that is not a canonical field element,
a division by zero — adding a point to itself, say — a modulus of 0, an
unaligned operand. The Poseidon permutations are the exception that follows
its source step by step: their constants are generated from it, and
Poseidon1 is in the optimised form, with sparse partial-round matrices, that
proofman-fields computes.

One simplification, which holds today but which the model does not
check:

- **No stack pointer.** The emulator adds `sp` to a `SRC_MEM`/`STORE_MEM`
  address when the instruction's `use_sp` bit is set. Nothing in the
  transpilers sets it, and `sp` starts at 0 and is never written, so the
  model leaves it out.

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
   `build/difftest/<name>/`. `mem/` holds the initial memory sections: the
   ROM's data and the input behind its 8-byte header, as
   `Emu::create_emu_context` lays them out; the ROM itself, as the table the
   emulator reads; and the fcall tape, `ziskemu`'s answers to the program's
   fcalls. `rom.sail` holds every instruction of the ROM, BIOS
   included, as a `rom_fetch`, to read. `ziskemu.trace` is the reference run,
   recorded by stepping `Emu` directly.
2. The model's C emulator, built once, runs with the memory sections loaded
   through `-b addr,file`.
3. [`test/compare_traces.py`](test/compare_traces.py) compares the two runs.
   They agree when the traces are equal and the model ran to its end; or,
   when `ziskemu` panicked on a step — the trace's last line is then
   `fault <message>` — when the model ran the same steps before it and then
   stopped on an assertion.

Each line of the trace is a step: the pc it ran at, `c`, `flag`, and every
register the step changed. After the last step comes how the run ended and
the output region. `a` and `b` are not traced, because they are locals of
`zisk_execute`, and exposing them would mean adding state the Lean side would
carry into every proof. A wrong source still shows up, in `c` or in a
register, at the step that uses it. A wrong memory write shows up when the
value is read back, or in the output.

The exporter refuses what the model does not describe, rather than exporting
something the model would get subtly wrong: an instruction that uses the
stack pointer, or a source or store kind with no Sail counterpart. An operation with no Sail clause gets
through the exporter, then stops the emulator when it reaches it: its opcode
has no `zisk_op_of_code` clause, and `rom_fetch` fails with "an opcode the
model does not specify".

## Compliance testing

`make compliance` runs the RISC-V architectural tests
([riscv-arch-test](https://github.com/riscv-non-isa/riscv-arch-test)) through
the model, as ZisK runs them: compiled for ZisK, transpiled, and difftested
against `ziskemu`, every step. They are what the transpiler turns RISC-V into,
so they check the model on the instructions the equivalence will be about —
loads and stores of every width, sign extension, the atomics, the bit
manipulation — and not only on hand-written ZisK.

[`test/compliance/env`](test/compliance/env) is the tests' harness for ZisK,
from [ziskof](https://github.com/0xPolygonHermez/ziskof)'s, for the current
memory map: `link.ld` puts code and read-only data at the program ROM,
`ROM_ADDR`, and the signature and other data in RAM; `model_test.h` ends a
test through ZisK's exit syscall once it finds ZisK's marker at
`ARCH_ID_CSR_ADDR`. [`test/compliance/run.py`](test/compliance/run.py)
compiles each test of the chosen suites with the suite's `-march`, runs them
in parallel through `make difftest`, and prints a line per test and a
summary.

A test passes on three counts:

- **The model agrees with ZisK.** Every step of the model's run, and the
  test's signature (`DUMP=` its `begin_signature`, `end_signature`), is
  `ziskemu`'s.
- **The test runs to its end.** The run reaches `exit_cleanup`, the test's
  halt. Agreeing is not enough: a test both emulators abandon at the same
  instruction — one ZisK does not run, which transpiles to an error — agrees
  too. That is how the compressed suite passed, hollowly, until this check:
  every test stopped at its first 2-byte instruction, on both. How the run
  ends does not tell, since ZisK's exit syscall ends a run through `halt` as
  well.
- **The signature is RISC-V's.** sail-riscv's C emulator, configured for
  ZisK's profile (`riscv/zisk_profile.py`), runs the RISC-V program itself up
  to `exit_cleanup`, and its signature must be the model's: the transpiled
  program computes what the RISC-V one does, which is the equivalence to be
  proved, checked on these tests. Without `make riscv-sim`'s build, this
  check is left out.
- **ZisK's RISC-V machine ends it so too.** `lean/ZiskRiscv`, run on
  sail-riscv's executable Lean, must end the test as the model does: the
  same exit, output and signature (`make zisk-riscv`). Without
  `make riscv-run`'s build, this check is left out.

Tests of instructions ZisK does not run are skipped, each with its reason:
the compressed instructions, carry-less multiplication and Zicond, which the
transpiler's default build does not have, and Fence.i's test, which rewrites
its own code. A test that fails is either a difference between the model and
ZisK or RISC-V, or an instruction ZisK does not run that the skip list
should name.

## ZisK's RISC-V

What the transpiler does is not always what RISC-V specifies, on purpose:

- **`ecall` is a call into the BIOS.** It stores pc + 4 in `x1` (`ra`) and
  jumps to the BIOS's syscall handler at a fixed address, ZisK code with no
  RISC-V counterpart; RISC-V would trap to `mtvec`, setting `mepc` and
  `mcause` and leaving `ra` alone. (With the `float` feature, the jump goes
  to `mtvec` instead.)
- **A CSR is a memory cell.** Every CSR is 8 bytes at `CSR_ADDR + 8·csr`,
  read and written with none of RISC-V's side effects or legal values: no
  WARL fields, no counters. A write to a read-only CSR, which `unimp` is,
  ends the run in error; and `csrs` on the precompile and DMA CSR numbers,
  with the instructions that follow, becomes a precompile call.
- **`ebreak` ends the run in error.**

The architectural tests stay clear of these: their harness's CSR accesses
set up a trap handler no test takes, and they end before the exit syscall.
[`doc/riscv_contract.md`](doc/riscv_contract.md) states them all — how a
program starts and ends, `ecall`, the CSRs, the precompile and free-input
CSR sequences — as the RISC-V side of the equivalence, with its
preconditions: no `ecall` but exit, no CSR access but ZisK's own calls, no
zkvmcall.

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

It also checks what `m32` promises. The circuit hands an `m32` operation —
the 31 whose names contain `_w` — only the low 32 bits of a, zeroed above
(`pil/operations.pil`), while the emulator, and so the model, hand it all of
a. The two agree only if the operation ignores a's high half, which nothing
in ZisK checks: a new `_w` operation that read it would be proven on other
operands than it runs on, with every test still passing. So for each `m32`
operation and vector, `check_op` also runs it with a's high half zeroed and
requires the same result. Marking `add` `m32` fails it at once.

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
- **WRONG CODE** — the model numbers an operation differently from Rust.
- **UNMODELED** — an operation in Rust with no Sail clause. An error now
  that the model specifies every operation: `make ops` runs
  `check_ops.py --strict`.

Run it in CI alongside the typecheck.
