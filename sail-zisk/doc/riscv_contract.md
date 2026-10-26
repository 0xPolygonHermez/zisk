# ZisK's RISC-V: the contract

What a RISC-V program means when ZisK runs it: the RISC-V side of the
equivalence between sail-riscv and sail-zisk. Where ZisK follows RISC-V, the
meaning is sail-riscv's, in ZisK's profile (`riscv/zisk_profile.py`), and
sail-riscv itself is not changed. Where ZisK departs from RISC-V — how a
program starts and ends, `ecall`, the CSRs, and the CSR numbers through which
a program calls ZisK's precompiles and its free input — this document is the
specification, and sail-zisk's operations give the precompiles' meaning.

The theorem to prove, informally: for a program in the profile that meets the
preconditions below, the run sail-riscv and this contract define, and the run
of its transpilation on sail-zisk, end the same way, with the same output.

Each statement is from the transpiler and the BIOS of `pre-develop-1.4.0-alpha`
(`transpilers/riscv/src/riscv2zisk_context.rs`, `core/src/rom_layout.rs`),
in their default build.

## 1. The instruction set

RV64IMA, Zba, Zbb, Zbs and Zbkb, Zicsr and Zifencei; machine mode only; no
interrupts. Not in it: the compressed instructions (the transpiler's
`compressed` feature is off, IALIGN = 32), F and D (`float` feature), Zbc,
Zicond, the privileged instructions (`mret`, `wfi`, `sfence.vma`).

- `fence` and `fence.i` do nothing. A program that rewrites its own code is
  outside the contract: ZisK runs code from a read-only ROM.
- `ebreak`, an instruction outside the set, and a reserved encoding end the
  run in failure (§ 4).
- `lr.w` and `lr.d` are plain loads (sign-extended for `.w`), and `sc.w` and
  `sc.d` always succeed: the store, then 0 to rd. ZisK keeps no reservation:
  it is one hart, with no interrupts, so the failures RISC-V requires of an
  `sc` (another hart's store, a reservation lost to preemption) cannot
  happen, and a spurious failure is only permitted. Where this departs from
  RISC-V is an `sc` with no `lr` before it, or to another address or width
  than its `lr`'s: it succeeds here, and must fail on hardware; compiled
  code pairs them. (sail-riscv's Lean leaves its reservations as axioms, so
  this document, not sail-riscv, gives `lr` and `sc` their meaning.) An
  `lr` whose bytes include its own instruction word is outside the contract
  (precondition **L**): the machine runs the plain load in the `lr`'s place
  (§ "As a machine"), and would read that load's word there. The AMOs are
  RISC-V's.
- The transpiler decodes as RISC-V does, in this profile: a word RISC-V
  rejects becomes a halt, never another instruction, and a word of an
  extension outside the set fails only if it runs, as on RISC-V; it does not
  stop the whole program from transpiling. `make decoder-diff` checks this
  on every word the transpiler's decoder can tell apart, against
  sail-riscv's decoder. The only differences are on opcode SYSTEM, and this
  document describes them: `ebreak`, `mret`, `sret` and `wfi` end the run in
  failure (above), and the CSRs are ZisK's (§§ 6–9).

## 2. Memory

Three regions, as `model/zisk_mem.sail` states and ZisK's circuits prove:

| Region | Addresses | Access |
|---|---|---|
| input | `[0x1000_0000, +2 GB)` | read-only |
| ROM | `[0x9000_0000, +128 MB)` | read-only: the program's code and read-only data |
| RAM | `[0xA000_0000, +512 MB)` | read-write |

A load outside them, a store outside the RAM, or an access straddling a
region's end ends the run in failure. A program's writable data and its
stack are in the RAM; its ELF segments are placed there and in the ROM, as
`ziskbuild/zisk_linker_script.ld` lays them out.

Part of the RAM is ZisK's: the registers' slots at `[0xA040_0000,
0xA040_0140)`, which no access may touch (failure); the CSR cells (§ 6), at
`CSR_ADDR = 0xA040_8000 + 8·csr`, which loads and stores reach too; the
precompiles' parameter area at `EXTRA_PARAMS_ADDR = 0xA040_0F00`; and the
output region at `OUTPUT_ADDR = 0xA041_0000`.

The 8 bytes at `INPUT_ADDR = 0x1000_0000` are not memory: an 8-byte load
there returns the free input (§ 8). The input follows, from `INPUT_ADDR + 8`;
past its end, the region reads as 0.

## 3. The start

The program starts at its ELF entry, in machine mode, with

- `a0` = `INPUT_ADDR`, `a1` = `OUTPUT_ADDR`;
- `ra` = a return address into the BIOS: returning to it ends the run in
  success (§ 4);
- every other register 0;
- the CSR cells 0, but `marchid` (0xF12), which holds `0x0FFF_EEEE`, ZisK's
  architecture ID, and `mtvec` (0x305), which holds the BIOS's system call
  handler;
- the ROM and RAM holding the ELF's segments, the rest of the RAM 0;
- the input in the input region, and the free input 0, no fcall made.

## 4. The end

A run ends in **success** when the program

- returns to its initial `ra`; or
- makes the exit system call: `ecall` with `a7` = 93 and `a0` = 0.

It ends in **failure** — a run ZisK does not prove — when it makes the exit
system call with `a0` ≠ 0 (its exit code is `a0`), executes `ebreak` or an
instruction outside § 1, writes a read-only CSR (§ 6), or accesses memory
outside § 2. A precompile's own failures (§ 7) are failures too.

On success, the output is the 32 64-bit words at `OUTPUT_ADDR`, which the
BIOS publishes.

## 5. `ecall`

| `a7` | What `ecall` does |
|---|---|
| 93 | exit: success if `a0` = 0, else failure with code `a0` (§ 4) |
| anything else | nothing, but **`ra` ← pc + 4**, then continues at pc + 4 |

That `ra` is ZisK's own: RISC-V's `ecall` traps to `mtvec` and leaves the
registers alone. It is harmless today — both runtimes make exactly one
`ecall`, the exit, which does not return (`ziskos/entrypoint/src/lib.rs`,
`ziskasm/lang/c/src/_start.s`) — and the contract takes it as a
precondition:

> **Precondition E.** The program makes no `ecall` other than the exit
> system call.

So the theorem never meets a non-exit `ecall`, and sail-riscv, which would
trap, is never asked to run one: its run ends at the exit `ecall`, which is
the end of § 4. (Branch `feature/ecall-return-reg` makes ZisK's `ecall`
keep `ra`, returning through r39 instead; with it, a non-exit `ecall` could
join the contract as "does nothing".)

## 6. CSRs

A CSR is an 8-byte cell at `CSR_ADDR + 8·csr`, in the RAM. `csrrw`, `csrrs`,
`csrrc` and their immediate forms read it into `rd` and write it back as
RISC-V says — the swap, the bitwise or, the bitwise and-not — with none of
RISC-V's side effects or legal values: no WARL fields, no counters (`cycle`
reads whatever its cell holds, 0), no `mstatus`. A write to a read-only CSR
— one whose number has its two top bits set, as `unimp`'s `csrrw x0, cycle,
x0` does — is a failure (§ 4).

The CSR numbers of §§ 7–9 are the exceptions: an access of the form those
sections give means what they say, not a cell access.

The cells are ZisK's, not RISC-V's: sail-riscv, unchanged, gives `mstatus`,
`mtvec`, the counters and the rest their RISC-V meaning, which the cells do
not have. So the contract leaves them out:

> **Precondition C.** The program accesses no CSR but in the forms of
> §§ 7–8, and by reading `marchid`; and it stores nothing to `marchid`'s
> cell.

`marchid` is the one the runtime needs: `ziskos` reads it to tell ZisK from
QEMU, which decides where it writes the public outputs and how it exits
(`ziskos/entrypoint/src/lib.rs`). sail-riscv's `marchid` is a configuration
parameter, which ZisK's profile sets to ZisK's ID, `0x0FFF_EEEE`: so both
read the same, as long as the cell, which a store reaches (§ 2), still
holds the ID.

## 7. Precompiles: CSRs 0x800–0x821

A precompile call is a `csrrs`/`csrrsi` on its CSR, and for most, one or two
instructions after it that carry further operands; the sequence is one
operation — sail-zisk's of the same name (`model/zisk_insts_*.sail`), with its
`a`, `b` and extended argument as the table gives them — and continues after
the sequence. A sequence's later instructions are still in the program on
their own: reached by a jump, they do what RISC-V says, which, writing `x0`,
is nothing. A sequence of the wrong shape — a two-instruction form whose
second instruction is not the `add`/`addi` the table gives — becomes a
halt, as a reserved encoding does: the run fails if it gets there (§ 4); it
does not stop the whole program from transpiling. Any other access to these
numbers — with `rd` ≠ `x0` where the table has none, say — is a cell access
(§ 6), and `csrs ID, x0` does nothing.

| Form | Operation |
|---|---|
| `csrs ID, p` (p ≠ x0) | `op(b = p)`, for `keccak` 0x800, `arith256` 0x801, `arith256_mod` 0x802, `secp256k1_dbl` 0x804, `bn254_curve_dbl` 0x807, `arith384_mod` 0x80B, `bls12_381_curve_dbl` 0x80D, `poseidon2` 0x812, `secp256r1_dbl` 0x818, `poseidon1` 0x81B, `poseidon2_koalabear` 0x821 |
| `csrs ID, p1`; `add x0, p2, x0` | `op(a = p1, b = p2)`, for `secp256k1_add` 0x803, `sha256` 0x805, `bn254_curve_add` 0x806, `bn254_complex_add/sub/mul` 0x808–0x80A, `bls12_381_curve_add` 0x80C, `bls12_381_complex_add/sub/mul` 0x80E–0x810, `secp256r1_add` 0x817, `babyjubjub_add` 0x81D, `blake3` 0x81E, `blake2s` 0x81F, `ed25519_add` 0x820 |
| `csrs 0x819, p1`; `addi x0, p2, k` | `blake2b(a = p1, b = p2, ext = k)` |
| `csrrs rd, 0x811, p` (p ≠ x0) | `add256(b = p)`; `rd` ← its carry |
| `csrrs rd, ID, src`; `add x0, dst, n` | `[EXTRA_PARAMS_ADDR]` ← `n`, then `op(a = dst, b = src)`, `rd` ← its `c`: `dma_memcpy` 0x813, `dma_memcmp` 0x814, `jump_dest` 0x81C |
| `csrs ID, src`; `addi x0, dst, n` | `op(a = dst, b = src, ext = n)`, for `dma_xmemcpy` 0x813, `dma_xmemcmp` 0x814 |
| `csrs 0x816, dst`; `addi rd, n, v` | `dma_xmemset(a = dst, b = n, ext = v)`, `rd` ← its `c` |
| `csrsi 0x816, 2`; `addi rd, dst, n`; `addi _, _, v` | `dma_xmemset(a = dst, b = n, ext = v)`, `rd` ← its `c` |
| `csrs 0x815, _`; `add rd, dst, n` or `addi rd, dst, n` | `dma_inputcpy(a = dst, b = n)`, `rd` ← its `c` |
| `csrs 0x81A, t`; `addi x0, x0, k` | `profile`: nothing |

Where a sequence's later instruction names a destination `rd`, `rd` gets the
operation's result, not that instruction's own; two older forms, flagged as
deprecated by the transpiler, take `rd` from the `add`/`addi` of the
memcpy/memcmp sequences the same way.

## 8. Free input: CSRs 0x8C0–0x8FF, 0xFFE

A program asks the host for values through fcall; nothing relies on the
answers being right, and the contract, like sail-zisk, does not say how the
host computes them (`model/zisk_insts_fcall.sail`): the answers are an
oracle, a tape that is part of the initial state.

| Form | Operation |
|---|---|
| `csrs 0x8F0+k, p` | `fcall_param`: pass `w(k)` words (`w` = 1, 2, 4, 8, 12, 16, 20, 24, 25, 32, 48, 64, 80, 96, 128, 256), at `p`, or `p` itself when `w(k)` = 1 |
| `csrwi 0x8C0+k, i` | `fcall` of function `32·k + i`: the tape's next answer becomes the result, the free input its first word |
| `csrr rd, 0xFFE` | `rd` ← the free input; then move it to the result's next word (`fcall_get`) |
| 8-byte load of `INPUT_ADDR` | the free input |

## 9. zkvmcalls: CSRs 0x850–0x8BF — outside the contract, for now

A zkvmcall calls a routine of the ZisK library (`ziskasm/zisklib`): about 100
of them — keccak, sha256, modexp, the BN254 and BLS12-381 pairings, u256
arithmetic, signature verification — written in ZisK assembly, about 31 700
lines. Each one's meaning is its code; showing that it computes what its name
says is a verification of its own, against the function's mathematical
definition, and is left for later.

> **Precondition Z.** The program makes no zkvmcall: no access to CSRs
> 0x850–0x8BF or 0x8E0–0x8E7.

## As a machine

`lean/ZiskRiscv` states this contract in Lean, as a machine: sail-riscv's
step, in ZisK's profile, except at a sequence of §§ 7–8, which runs
sail-zisk's own operation on the RISC-V memory, at the end of § 4, and at
`lr` and `sc`, which run as § 1 says. It
is generic in the RISC-V model: on sail-riscv's Lean, it is the RISC-V side
of the theorem; on its executable Lean (`lean/ZiskRiscvRun`,
`make riscv-run`), it runs programs, and ends every compliance test and the
fibonacci guest as sail-zisk ends their transpilations (`make zisk-riscv`,
`make compliance`).

Memory a ZisK run starts as 0 — the RAM, the ROM's unused bytes, the input
past its end — the machine reads as 0 inside the regions of § 2; sail-riscv's
Lean, which has no such default, is retried with the byte written 0.

The regions of § 2 are sail-riscv's PMA regions in ZisK's profile, with
their permissions — the input readable, the ROM readable and executable, the
RAM readable and writable — and there is no PMP. So an access outside § 2,
against a region's permissions or straddling a region's end is an access
fault in sail-riscv: a trap, and a failure, as on ZisK. One part of § 2 a
PMA cannot state: the registers' slots, `[0xA040_0000, 0xA040_0140)`, which
share a 4 KB page with the parameter area; to sail-riscv they are RAM.

## Preconditions, together

A program is in the contract when it is in the instruction set of § 1 and
meets **E** (no `ecall` but exit), **C** (no CSR access but the ZisK calls
of §§ 7–8 and reading `marchid`, no store to `marchid`'s cell), **Z** (no zkvmcall) and **L** (no `lr`
reading its own instruction word). Everything else
it may do — touch memory outside § 2, write a read-only CSR, misuse a
precompile — has a defined outcome above: a failure, which ZisK does not
prove, and which the theorem must show both sides agree on.
