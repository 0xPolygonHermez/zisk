# EF zkEVM Standards Conformance

ZisK aims to conform to the Ethereum Foundation **zkEVM standards** published by
the `eth-act` working group at
<https://github.com/eth-act/zkevm-standards/tree/main/standards>. These standards
define a common contract between a zkVM and the guest programs it proves — the
RISC-V target, the ELF it loads, the memory layout, the I/O and termination
interfaces, the C ABI for cryptographic accelerators (the EVM precompiles), the
C ABI for 256-bit EVM-word arithmetic, host randomness and diagnostic logging —
so that a single guest can be built once and proven on any conforming zkVM.

This document records, standard by standard, how ZisK currently measures up.
It was last checked against the repository on 2026-10-27 (`main` at `f433d48`,
whose `standards/` last changed on 2026-08-11) and against the open pull requests
of that date; the drafts are tracked in [Draft standards](#draft-standards).

The standards are one item of a broader rubric: the EF's
[zkVM handbook](https://github.com/eth-act/zkevm-standards/blob/main/handbooks/zkvm-handbook.md)
lists the criteria for a zkVM to be considered for EIP-8025 (the test monitor,
formal verification of the circuits, real-time proving on the EF reference cluster,
MIT + Apache 2.0 licensing, final proofs of at most 300 KiB, cryptographic security
review). Those are outside the scope of this document.

> Note: the EF standards target a **RISC-V** guest. ZisK proves RISC-V programs;
> the ZisK-assembly (`.zisk`) routines mentioned below are ZisK's *internal*
> hand-written implementations that the guest's calls are turned into jumps to at
> transpile time — they are an implementation detail of how ZisK accelerates the
> standard interfaces, not something a conforming guest is written in.

## How the standard interfaces are wired: zkvmcalls

The guest is compiled by an **ordinary RISC-V toolchain** that knows nothing about
ZisK. It must nonetheless call the standard symbols — `zkvm_keccak256`,
`read_input`, `write_output`, and so on. The real
implementations are hand-written `.zisk` files under `ziskasm/zisklib/` (e.g.
`zkvm/keccak.zisk`). They are assembled — via `include_str!`, at ZisK build time —
into a **reserved ROM/RAM region** (`ZISKLIB_ROM_ADDR`, carved out of the address
space so it never collides with guest allocations) and merged into the guest's ROM.
The guest reaches them through zkvmcalls.

A zkvmcall takes one of two forms.

**Thunk.** Every function in `zkvm_accelerators.h`, `zkvm_io.h` and
`zkvm_random.h`, the division
family and `exp` of `zkvm_u256.h` and `zkvm_u256_le.h`, and every function in
ZisK's `zkvm_zisklib.h`, is a two-instruction thunk — for the C ABI in
`ziskasm/lang/c/src/zkvm_calls.s`, for Rust guests as naked functions in the
`zisklib` crate:

```asm
zkvm_modexp:
    csrs 0x856, x0
    ret
```

The CSR number identifies the function; the IDs live in
`definitions/src/zkvmcall.rs` (`0x850..=0x8BF`). The caller has already put the
arguments in `a0..a7` and its return address in `ra`, following the RISC-V calling
convention. When `elf2rom` converts the ELF into a ZisK ROM, it replaces each
`csrs <id>, x0` with a **static tail-jump into the library routine**. Because it is
a *tail* jump, `ra` is untouched, so the `.zisk` routine's own `ret` returns
straight to the guest's original caller; the thunk's `ret` never runs. The
transpiler finds zkvmcalls by instruction, not by symbol name, so **the guest ELF
may be stripped**.

**Inline zkvmcall.** By default the other `zkvm_u256.h` and `zkvm_u256_le.h`
functions (add, compare, shift, ...) are `static inline` asm in the header: a
sequence of `csrs`, the zkvmcall carrying argument 0 and one `csrs 0x8E0 + k - 1`
per further argument k, each naming the register the compiler chose for it.
`elf2rom` replaces the whole sequence with the routine's **body**, reading those
registers: no argument moves, no call and no return, and the constant `ZKVM_EOK`
status folds away. The body writes only memory and the virtual registers
`r32..r39`, so the asm clobbers no register. Each such `zkvm_u256.h` function also
has a thunk (used with `ZKVM_U256_CALLS` or by a declarations-only header), which
expands the same body; the inline `zkvm_u256_le.h` functions have none.
`definitions/src/zkvmcall.rs` marks which IDs are inline and explains the
sequence; [the C binding's README](../../ziskasm/lang/c/README.md#coverage) lists
the builds of each family.

`zkvm_keccak_f1600` is neither: it is a single keccak-f precompile, so the header
defines it inline. An 8-byte-aligned state costs one `csrs 0x800, state` and an
alignment test at the call site; an unaligned state is DMA-copied into an aligned
stack buffer, permuted there and copied back.

Consequences worth knowing:

- **A bad zkvmcall is a transpile error.** An unknown or malformed zkvmcall makes
  `elf2rom` reject the ELF with a message naming it.
- **Link with `--gc-sections`.** Otherwise every thunk in the archive stays in the
  ELF, and `elf2rom` assembles the library even for a guest that calls none of them.

A guest that uses no zkvmcall never gets the library: `elf2rom` then neither
assembles nor merges it, and the ROM is exactly what it would be without it.

## How we test conformance: ziskethone (for now)

The EF standards are written around a **C/C++ guest** that calls the standard
interfaces (`read_input`/`write_output`, `zkvm_accelerators.h`, the standard
`_start`, etc.). Our long-term guest is **evm-asm** — an EVM client in **RISC-V
assembly generated from LEAN** code — but it is not ready yet.

Until then we exercise and validate the standards using **ziskethone**, the
`evmone`-based C++ block prover (`cpp-guest`). ziskethone is a real Ethereum
execution-layer client compiled to a RISC-V ELF, so it stresses every standard in
this document: it is loaded and validated by `elf2rom`, runs against our memory
layout and RISC-V target, reads its block input and writes the block hash through
the I/O interface, and — most importantly — routes all of its EVM precompiles
through the standard `zkvm_accelerators.h` C ABI. We validate correctness by
running ziskethone on real mainnet blocks through `ziskemu` and checking that the
computed block hash is byte-identical to the native (non-accelerated) reference.
When evm-asm matures it will replace ziskethone as the conformance vehicle, but
the standards it must satisfy are the ones tracked here.

### Building and running the test

Two builds are involved: **ZisK** (to get `ziskemu`, which carries the `elf2rom`
transpiler and the zkvmcall table that maps the standard `zkvm_*` functions onto
the native `.zisk` accelerators) and **ziskethone** (the guest ELF,
built with the standard C ABI turned on).

**1. Build ZisK (`ziskemu`)** — from the `zisk` repository:

```sh
cargo build --release -p ziskemu --bin ziskemu   # -> target/release/ziskemu
```

The ZisK library is always built in: every `ziskemu` and `cargo-zisk` handles
zkvmcall guests, in emulation, ROM generation and proving alike. (The `ziskasm`
*features* under `test-artifacts/programs/` are unrelated: they switch those test
guests from the Rust `zisklib` to the ZisK library.)

**2. Build the ziskethone guest** — from the `ziskethone` repository, branch
`feature/zkvm-abi`. A RISC-V bare-metal C++ toolchain (xPack `riscv-none-elf-g++`
14.x) must be on `PATH`:

```sh
# once: fetch evmone and apply the branch's patches (host build, cpp-guest/build)
cmake -S cpp-guest -B cpp-guest/build

# the ZisK guest
cmake -S cpp-guest/zisk -B cpp-guest/zisk/build \
      -DCMAKE_TOOLCHAIN_FILE=$(pwd)/cpp-guest/zisk/toolchain.cmake \
      -DCMAKE_BUILD_TYPE=Release
cmake --build cpp-guest/zisk/build -j8
# -> cpp-guest/zisk/build/zisk_eth_guest.elf
```

On this branch the guest always uses the zkVM interface; there is no switch to
turn it off. Every precompile, the EVM's 256-bit arithmetic, its memory operations
and its I/O call the standard functions (`zkvm_keccak256`, `zkvm_u256_*`,
`read_input`, ...), whose zkvmcalls `elf2rom` turns into the native `.zisk`
routines. `EVM_BACKEND=zevm` selects ziskethone's hand-written interpreter instead
of evmone (use a separate build directory); the guest's
[`cpp-guest/zisk/README.md`](https://github.com/0xPolygonHermez/ziskethone/blob/feature/zkvm-abi/cpp-guest/zisk/README.md)
lists the other options and which ABI function each guest feature uses.

**3. Run through `ziskemu`** and read the public output (the 32-byte block hash):

```sh
ZE=<zisk>/target/release/ziskemu
GUEST=<ziskethone>/cpp-guest/zisk/build/zisk_eth_guest.elf
$ZE -e "$GUEST" -i <block-input>.bin -o /tmp/out.bin -X    # -X prints step/cost stats
xxd -p -c32 /tmp/out.bin                                    # the block hash
```

`ziskemu` expects the input framed as `[u64 LE length][payload]`, where the payload
is ziskethone's block container (it starts with the `ZEG0` magic). Framed inputs
live in the `zisk-eth-client` repo, e.g.
`bin/guests/stateless-validator-ziskethone/inputs/*.bin`.

**4. Confirm conformance (A/B).** Run the same block through ziskethone's native
host build, which uses none of the zkVM interface, and check that it prints the same
block hash:

```sh
cmake --build cpp-guest/build -j8                      # the native guest
tail -c +9 <block-input>.bin > /tmp/blk.container      # drop the 8-byte length frame
./cpp-guest/build/zisk_eth_guest /tmp/blk.container | tail -1
```

A byte-identical hash shows the zkVM interface produces exactly the native result.
The rarer precompiles, which real blocks may not reach, are covered by the
per-function golden-vector guests under `ziskasm/zisklib/scripts/benchmark/`.

## Summary

Legend: **Conformant** — meets the normative requirements; **Partial** — meets
the substance, with gaps or items still to confirm; **To verify** — believed to
conform but not yet audited against the spec.

| # | Standard | Assessment | One-line status |
|---|----------|------------|-----------------|
| 1 | [C interface for accelerators](#1-c-interface-for-accelerators) | **Conformant** | All 19 `zkvm_*` functions implemented natively in `.zisk`; runtime-validated on real blocks via ziskethone. `zkvm_keccak_f1600` (draft PR #26) also provided. |
| 1b | [U256 arithmetic C interface](#1b-u256-arithmetic-accelerator-c-interface) | **Conformant** (draft) | Draft standard (eth-act PR #22). All 27 `zkvm_u256_*` functions implemented natively in `.zisk` over the shared uint256 precompile cores; golden-vector-validated against an EVM reference. |
| 2 | [Accelerated memory operations](#2-accelerated-memory-operations) | **Conformant** | All four `mem*` accelerated via DMA precompiles in ziskos and in the packaged C archive; `memmove` shares `memcpy`'s DMA op, which is overlap-safe in the emulator and the prover; the C archive's definitions sit in `_start`'s object, so they win symbol resolution regardless of link order. |
| 3 | [ELF loading and validation](#3-elf-loading-and-validation) | **Partial** | `elf2rom` enforces header, PT_LOAD-only loading, zero-fill, no overlap, W^X and entry-point validation. But it accepts both `PF_X` and `PF_X\|PF_R` executable segments, and the standard requires a zkVM to accept exactly one. |
| 4 | [I/O interface](#4-io-interface) | **Conformant** | `read_input` / `write_output` implemented in the ZisK library, reached through zkvmcalls. |
| 5 | [Memory layout restrictions](#5-memory-layout-restrictions) | **Conformant** | Standard is non-prescriptive; ZisK ships a vendor linker script defining its map. |
| 6 | [Memory safety guard regions](#6-memory-safety-guard-regions) | **Conformant** | Null region and the span below the stack bottom are both unmapped and trap; the guard is adjacent with no gap, named in `core/src/mem.rs` and enforced by compile-time assertions. |
| 7 | [RISC-V target](#7-risc-v-target) | **Partial** | RV64IMA, little-endian, LP64, unaligned access supported and counted/priced by `ziskemu` stats; `compressed` (`C`) feature implemented (off by default); unaligned operands no longer reach a precompile. One change remains: emit a per-proof unaligned-access count from the proving path (the tally already exists in `MemCounters`). |
| 8 | [Standard termination semantics](#8-standard-termination-semantics) | **Conformant** (Type 1, to confirm) | A nonzero `main` return, a trap or a write to a read-only CSR halts as a failed execution, reported with its exit code and rejected before proving. A failed run cannot end at the pc the circuit requires, so it cannot be proven; the PIL owners are to confirm that argument. |
| 9 | [Static library and linker script](#9-static-library-and-linker-script) | **Conformant** | `package.sh` stages `libzisklib_c.a` (`_start` incl. C++ ctors/dtors, I/O, all accelerators) + headers + the W^X linker script exporting `_heap_start`/`_heap_end`; verified by linking C and C++ guests against the installed artifacts alone. |
| 10 | [Instruction-address-misaligned semantics](#10-instruction-address-misaligned-exception-semantics) | **Conformant** | At the default `IALIGN=32` (`compressed` off): misaligned entry points and executable-segment starts rejected at load; a misaligned computed jump hits its own `emu_end` slot in the per-byte jump map (rounding impossible by construction) and exits with `end=0`; the Rust emulator panics. |
| 11 | [Host randomness](#11-host-randomness) | **Conformant** | `zkvm_random_u64` returns the SHAKE256 stream of a 256-bit seed drawn per execution from the OS CSPRNG (or provided with `--random-seed` to replay a run); the guest cannot set or observe it. Not available with hints. Standard still a draft (eth-act PR #42). |
| 12 | [Diagnostic logging](#12-diagnostic-logging) | **Conformant** (draft) | Draft standard (eth-act PR #27). Each `write_log` call site is one no-op step, the same in every ROM, so logging never changes the verification key; `ziskemu --write-log` prints the text, and executions and proofs ignore it. |

Draft standards that ZisK does not meet yet (`abort`, proving cost estimation,
minimum memory resources) are listed in
[Draft standards](#draft-standards).

---

## 1. C interface for accelerators

**Standard.** Defines the portable C API (`zkvm_accelerators.h`) through which a
guest reaches the zkVM's optimized implementations of the EVM precompiles.
Byte-encoded (big-endian) field elements, a `zkvm_status` return, byte-struct
operands.

**ZisK.** This is the standard we implement most completely. All **19** functions
of `zkvm_accelerators.h` are implemented as hand-written ZisK-assembly routines
under `ziskasm/zisklib/zkvm/*.zisk`, exposed under the exact `ziskasm_zkvm_*`
entry points and reached from a guest through the zkvmcalls of the standard `zkvm_*`
functions:

- Hashes: `zkvm_keccak256`, `zkvm_sha256`, `zkvm_ripemd160`
- secp256k1: `zkvm_secp256k1_verify`, `zkvm_secp256k1_ecrecover`
- secp256r1 (P-256): `zkvm_secp256r1_verify`
- Modular exponentiation: `zkvm_modexp`
- BLAKE2: `zkvm_blake2f`
- BN254 (alt_bn128): `zkvm_bn254_g1_add`, `zkvm_bn254_g1_mul`, `zkvm_bn254_pairing`
- BLS12-381: `zkvm_bls12_g1_add`, `zkvm_bls12_g2_add`, `zkvm_bls12_g1_msm`,
  `zkvm_bls12_g2_msm`, `zkvm_bls12_pairing`, `zkvm_bls12_map_fp_to_g1`,
  `zkvm_bls12_map_fp2_to_g2`
- KZG (EIP-4844): `zkvm_kzg_point_eval`

Each routine performs the byte↔limb marshalling required by the EF encoding
(including the BN254 EIP-197 imaginary-first Fp2 order and the BLS12-381 packed
48-byte fields) and returns `ZKVM_EOK`. The header and the zkvmcall thunks live in
`ziskasm/lang/c/`. Its types, struct layouts and prototypes match the upstream header
exactly.

The header also defines `zkvm_keccak_f1600(uint64_t* state)`, which draft
[eth-act PR #26](https://github.com/eth-act/zkevm-standards/pull/26) adds to the
standard header; see [How the standard interfaces are wired](#how-the-standard-interfaces-are-wired-zkvmcalls).

**Validation.** Beyond per-function golden-vector tests, the full ABI was wired
into ziskethone and run on three real mainnet blocks; the block hash is
byte-identical to the native software path in every case, exercising
keccak/sha256/secp256k1 and BN254 end-to-end on-chain traffic.

**Assessment: Conformant.**

---

## 1b. U256 arithmetic accelerator C interface

**Standard.** Draft, [eth-act/zkevm-standards PR #22](https://github.com/eth-act/zkevm-standards/pull/22)
(not merged; unchanged since 2026-03-30). A companion header (`zkvm_u256.h`) in the same
`c-interface-accelerators` family, defining accelerated **256-bit unsigned-integer
(EVM word) arithmetic**: the arithmetic, comparison, bitwise and shift operations
that back the EVM opcodes. Every operand and result is a 32-byte **big-endian**
array (`zkvm_u256`, reusing `zkvm_bytes_32`); the result pointer **may alias** any
input; division/modulo by zero and `addmod`/`mulmod` with a zero modulus return
zero (EVM semantics), and `mulmod` must reduce the full 512-bit product.

**ZisK.** All **27** functions of `zkvm_u256.h` are implemented as hand-written
ZisK-assembly routines in `ziskasm/zisklib/zkvm/u256.zisk`, exposed under the
`ziskasm_zkvm_u256_*` entry points and reached from a guest through the zkvmcalls
of the standard `zkvm_u256_*` functions:

- Arithmetic: `add`, `sub`, `mul`, `div`, `mod`, `divmod`, `addmod`, `mulmod`, `exp`
- Signed (two's complement): `sdiv`, `smod`, `sdivmod`
- Comparison: `lt`, `gt`, `slt`, `sgt`, `eq`, `iszero`
- Bitwise / shifts: `and`, `or`, `xor`, `not`, `byte`, `shl`, `shr`, `sar`
- Extended: `signextend`

Rather than a separate bignum implementation, these wrappers **map onto the shared
ZisK arithmetic precompiles** already used by `ziskos`: they byte-reverse each
big-endian operand to the little-endian limbs the precompiles expect and reuse the
`uint256/*.zisk` cores — `add256` (add/sub), `arith256` (mul, and the `div_rem256`
quotient/remainder hint-verify), `arith256_mod` (`addmod`/`mulmod`/`exp`) — then
byte-reverse the result back. Bitwise and equality operations are byte-order
agnostic and act on the raw words directly (no marshalling). The EVM edge cases
the cores don't cover are added in the wrappers: a zero divisor/modulus yields
zero (instead of the core's panic), and the signed operations go through
absolute-value plus sign (so `-2^255 / -1 = -2^255` falls out naturally). Because
all inputs are read into private scratch before the output is written, the
result-aliases-input guarantee holds. The header and zkvmcall thunks live
in `ziskasm/lang/c/` (`zkvm_u256.h`, `zkvm_calls.s`) and the `zisklib` Rust crate.

**Validation.** A generated guest exercises all 27 functions (45 cases including
the div/mod/addmod/mulmod-by-zero guards, cross-word shifts, `sar` sign-fill and
`shift >= 256`, `byte >= 32`, `signextend`, and the signed min-int case) and every
result is **byte-identical to a Python EVM reference**. The prototypes match the
draft header's 27 functions exactly. (`zkvm_u256_le.h`, the little-endian variant, is
ZisK's own header, not part of the draft.)

**Assessment: Conformant** to the draft.

---

## 2. Accelerated memory operations

**Standard.** Acceleration of `memcpy` / `memmove` / `memset` / `memcmp` is
**optional**. If a zkVM provides them, they must be behaviorally identical to libc
for all inputs and alignments (including `n == 0`), assume no alignment, and be
guaranteed to win symbol resolution in the guest link (strong runtime definition
or `--whole-archive`; link order alone does not conform).

**ZisK.** ZisK accelerates bulk memory operations through dedicated **DMA
precompiles** (`dma_memcpy`, `dma_memcmp`, `dma_xmemset`). The guest runtime
defines `memcpy`/`memmove`/`memcmp`/`memset` as assembly thunks whose
CSR `0x813/0x814/0x816` + `add`/`addi` pattern the transpiler lowers to a single
DMA op. The Rust runtime has them in `ziskos/entrypoint/src/dma/*.s`, the packaged
C archive in `ziskasm/lang/c/src/_start.s`, and ziskethone vendors its own copies
(built with `-fno-builtin` so the calls stay out-of-line). Arbitrary alignment
and `n == 0` are handled, and the DMA circuits prove the aligned, unaligned and
partial head/tail cases (`precompiles/dma/`).

**`memmove`.** `memmove` uses the same CSR `0x813` op as `memcpy`. That op has
`memmove` semantics: when source and destination overlap, the emulator copies
through a temporary buffer (`Mem::memcpy` in `core/src/mem.rs`), and the prover
constrains the overlapping case too (confirmed with the DMA precompile
developers). One DMA op is therefore correct for every overlap direction.

**Link precedence.** The thunks are **strong** global definitions.

- *ziskos* (the mandatory Rust guest runtime): they override `compiler_builtins`'
  weak byte-loop fallbacks.
- *Packaged C archive* (§9, `libzisklib_c.a`): the thunks are in the same object
  as `_start`, not in archive members of their own. `ENTRY(_start)` pulls that
  object into every guest, so the accelerated definitions are always linked,
  whatever the link order. If a libc earlier on the command line also contributes
  its `mem*`, the link fails with a multiple-definition error instead of silently
  keeping the byte loop. A separate archive member would lose silently in that
  case, since it is only pulled when the symbol is still undefined. Both outcomes
  were checked against a stand-in libc archive. A runtime test linked only against
  the installed package checks overlapping `memmove` in both directions, `n == 0`,
  unaligned `memcpy`, `memset` with zero and non-zero fill, and the sign of
  `memcmp` under `ziskemu`. Each call executes as one DMA op.
- *ziskethone* links its copies as object files, so they are always part of the
  link.

**Assessment: Conformant** (all four operations accelerated, `memmove` overlap-safe
end to end, and precedence independent of link order).

---

## 3. ELF loading and validation

**Standard.** The loader must validate the ELF header (`\x7fELF`, ELFCLASS64,
little-endian, `EM_RISCV`, `ET_EXEC`), load only from `PT_LOAD` program headers,
zero-fill `p_memsz > p_filesz`, keep all addresses in range, reject
misaligned/overlapping segments, enforce W^X (reject `PF_W|PF_X`), and validate
the entry point (aligned, inside a loaded executable segment). Invalid ELFs must
be rejected with diagnostics before any state reaches the prover.

An executable segment is either `PF_X` (execute-only: a load from it must end the
run abnormally) or `PF_X|PF_R` (execute-and-read: a load returns the real byte).
**A zkVM supports exactly one of the two**, as an inherent property, and its loader
rejects an executable segment carrying the other. The standard encourages
`PF_X|PF_R`, because a jump table or constant pool that the compiler leaves in
`.text` stays correct under it.

**ZisK.** `transpilers/riscv/src/elf_extraction.rs` (called from `elf2rom.rs`):

- **Header.** Parsed as little-endian only (a big-endian file fails to parse rather
  than being adapted to), then `ELFCLASS64`, `EM_RISCV` and `ET_EXEC` are checked,
  each with its own diagnostic.
- **Segments.** The image is built exclusively from `PT_LOAD` segments. Each one must
  lie inside the ROM or RAM window, must not overlap another, must not have
  `p_filesz > p_memsz`, and must be aligned (an executable one to IALIGN). The
  `p_memsz > p_filesz` tail is zero-filled. An executable segment must lie in ROM and
  be fully file-backed.
- **W^X.** A `PF_W|PF_X` segment is rejected.
- **Entry point.** `validate_entry_point`: non-zero `e_entry`, aligned to IALIGN
  (4 bytes by default, 2 with the `compressed` feature, see §10), inside a loaded
  executable segment, with a diagnostic telling the user to declare
  `ziskos::entrypoint!`.

**Gap: ZisK accepts both executable-segment models.** A `PF_X` segment is treated as
execute-only. A `PF_X|PF_R` segment is accepted too, with a performance warning, and
its bytes are also loaded as readable ROM data. That violates "a zkVM supports exactly
one combination". The guest linker script emits `PF_X` (§9), so ZisK's model is
execute-only, and the fix is to reject `PF_X|PF_R` executable segments instead of
warning. That breaks any guest linked with another script whose code segment is
`PF_X|PF_R`, so the change should come with a clear diagnostic. (The comment at that
check also calls ZisK "an execute-and-read zkVM", which contradicts the code next to
it.)

Two consequences of the execute-only choice still need checking: that a data load
from a `PF_X` segment ends the run abnormally rather than returning a value, and that
our guest builds keep read-only data out of `.text` (`-fno-jump-tables`, or a
post-link check, as the standard recommends).

**Assessment: Partial** (header, loading, overlap, W^X and entry-point checks are in
place; the executable-segment model must be made exclusive).

---

## 4. I/O interface

**Standard.** Must provide `void read_input(const uint8_t** buf_ptr, size_t*
buf_size)` (returns a read-only pointer + length to the private input; never
fails; idempotent) and `void write_output(const uint8_t* output, size_t size)`
(successive calls concatenate into the public result; never fails).

**ZisK.** Both functions are implemented in the ZisK library (`zisklib_read_input`,
`zisklib_write_output`) and reached through zkvmcalls (0x863, 0x864). Input is exposed at the memory-mapped
free-input region (`INPUT_ADDR = 0x1000_0000`); output is written to the public
output region (`OUTPUT_ADDR = 0xa041_0000`). Reads are non-failing and side-effect
free; successive `write_output` calls concatenate. C guests get both from
[`zkvm_io.h`](../../ziskasm/lang/c/include/zkvm_io.h) and the zkvmcall thunks in
`ziskasm/lang/c/src/zkvm_calls.s`, linked via `zisklib_c` (§9). The draft
`write_log` function, which the same header declares, is covered in §12.

**Assessment: Conformant.**

---

## 5. Memory layout restrictions

**Standard.** Intentionally **non-prescriptive**: vendors define their own memory
map via a vendor-specific linker script rather than a single standardized map.
The requirement is that a vendor *provides* such a script (and, when linking
against libc, defines heap boundaries and region demarcations), and that programs
are linked with it.

**ZisK.** ZisK defines its map in `core/src/mem.rs` and ships the matching guest
linker script (`ziskbuild/zisk_linker_script.ld`):

| Region | Address |
|--------|---------|
| Free input | `0x1000_0000` (len `0x8000_0000`) |
| ROM (program) | `0x9000_0000` (len `0x0800_0000`) |
| RAM base / stack | `0xa000_0000` (stack 4 MB) |
| System reserved | `0xa040_0000` |
| Public output | `0xa041_0000` |
| RAM top | `0xc000_0000` |

**Assessment: Conformant** (ZisK supplies and requires its own vendor linker
script, exactly as the standard intends).

---

## 6. Memory safety guard regions

**Standard.** Two mandatory guard regions whose access must abnormally terminate:
a **null-pointer trap** over `0x0000`–`0x0FFF` (unmapped), and a **stack guard**
of at least 4 kB immediately below the stack bottom, contiguous with no gap.

**ZisK.** Both guard regions are satisfied by the address map, and the invariant is
now enforced rather than incidental.

**Null-pointer trap.** The lowest data area is the input window at
`INPUT_ADDR = 0x1000_0000`. Below it there is only the BIOS ROM (`0x1000`–`0x0fff_ffff`),
which holds instructions and is never a data area: no `Mem` or `RomData` section covers
any address below `INPUT_ADDR`. So a load or store anywhere in `0x0000`–`0x0FFF` fails the
`Mem` section bounds check and aborts, and since no instruction lives there either, a jump
into it aborts too.

**Stack guard.** The stack bottom is `STACK_ADDR = RAM_ADDR = 0xa000_0000` — the
lowest mapped RAM address — with the stack growing down from `0xa040_0000`. ROM ends
at `0x97ff_ffff`, so the span `0x9800_0000`–`0x9fff_ffff` (128 MB) is unmapped. That
region therefore begins at the byte immediately below the stack bottom, which is
exactly the standard's "contiguous and adjacent to the bottom of the stack with no
gap between them", and it is far larger than the required 4 kB.

`core/src/mem.rs` names the region — `STACK_GUARD_ADDR` = `ROM_ADDR + ROM_SIZE`,
`STACK_GUARD_SIZE` = `STACK_ADDR - STACK_GUARD_ADDR` (128 MB) — with
`STACK_GUARD_MIN_SIZE` recording the 4 kB the standard demands. The assembly
emulator's `GUARD_ADDR`/`GUARD_SIZE` are the same span, derived the same way, so the
two cannot drift.

The invariant is enforced at compile time: the stack must start at `RAM_ADDR`, the
guard must be adjacent to it, at least `STACK_GUARD_MIN_SIZE`, and the input window
must stay below the program ROM, which ends where the guard starts. Each failure mode was checked by making the change and observing
the build stop — growing `ROM_SIZE` until under 4 kB remains, growing it past
`RAM_ADDR` (which underflows the size computation and fails const evaluation),
widening `MAX_INPUT_SIZE`, and moving `STACK_ADDR` off the RAM base.

`ziskasm::assemble_library` additionally rejects a `rom_base` outside ROM or a
`ram_base` below `RAM_ADDR`, since those bases are caller-supplied and a library
placed in the guard would defeat it.

**Host-level reservation (assembly emulator).** The Rust emulator validates every
address against its section bounds in software, so an access to the guard aborts
regardless of the host. The **assembly** emulator does not: it maps the guest regions
`MAP_FIXED` at their real addresses and relies on the host MMU, so a guest access
traps only because nothing is mapped there. An unmapped hole is exactly where the
kernel may satisfy a later `mmap(NULL, …)` or a large `malloc` in the same process —
and if anything landed in the span, a guest access into the guard would silently
*succeed*.

`server_setup()` (`emulator-asm/src/server.c`) therefore reserves the whole span
`GUARD_ADDR`..`RAM_ADDR` (0x9800_0000–0x9fff_ffff, 128 MB) as
`PROT_NONE | MAP_FIXED | MAP_NORESERVE`: the range is claimed so nothing else can be
placed in it, any access faults, and `MAP_NORESERVE` keeps it off the commit charge
(a single VMA, no backing pages). Measured before and after in a standalone harness:
an allocation targeting the middle of the span **succeeds** without the reservation
and is **refused** with it, and an access immediately below the stack bottom raises
`SIGSEGV`.

**Verified.** Writes at `0x0`, `0x800` and `0xfff` (the null region, at both ends and
the middle) and at `0x9fff_fff8` and `0x9fff_f000` (immediately below the stack bottom
and 4 kB below it) each abort with
`Mem::write_silent() invalid addr=… write section start=a0000000 end=c0000000`, and
**no output file is produced** — matching the standard's Detection and Reporting
clause that execution halts, failure is reported, and no valid proof of successful
execution may be generated.

**Assessment: Conformant.**

---

## 7. RISC-V target

**Standard.** Base **RV64I**; **MUST** support **M** and **Zicclsm** (misaligned
loads/stores to main memory); **MUST NOT** support **C** (compressed) or **F/D**
(floating point); little-endian; **LP64** soft-float ABI; flat memory, no MMU;
statically linked ELF; machine mode only. Zicclsm additionally requires the zkVM
to expose *visibility* into the number of unaligned accesses during proving.

**ZisK.** ZisK targets **RV64IMA** — it provides the required **M** extension
(plus **A**, atomics, which are trivially satisfied on the single-threaded
machine), is little-endian, uses the **LP64** soft-float ABI, has a flat
no-MMU memory model, and loads statically linked ELFs in machine mode. Guests are
compiled `-march=rv64ima` (no F/D). Unaligned loads/stores are supported (the
library and guests rely on them, e.g. unaligned 64-bit absorbs in keccak).

**Unaligned-access visibility.** `Zicclsm` requires the zkVM to expose how many
unaligned accesses a run performed. `ziskemu` already does — it counts and prices
*every* memory access, classifying it by shape, in
`emulator/src/stats/mem_operations_stats.rs` (`MemoryOperationsStats`). Because
RISC-V registers are memory-mapped in ZisK, this covers register accesses too.

Each access to a known region (RAM-stack, RAM-non-stack, ROM, INPUT) is tagged as
`aligned 8B` or one of the unaligned shapes, and both a **count** and a **cost**
are accumulated per shape and per region:

| shape | meaning |
|-------|---------|
| `aligned 8B` | 8-byte access on an 8-byte boundary — the only aligned case |
| `unaligned 1B` | single-byte access (writes split **clean**/**dirty**) |
| `unaligned 2B single` / `2B double` | 2-byte access within one / straddling two 8-byte rows |
| `unaligned 4B single` (32-align vs not) / `4B double` | 4-byte access, within one row / straddling two |
| `unaligned 8B double` | 8-byte access straddling two rows |

("double" = the access crosses an 8-byte boundary and so touches two memory rows;
it is the case that costs an extra memory row in the proof, which is why the split
matters. The two widest double cases also set a `MEM_ACCESS_MONITOR` flag so the
emulator can log them with full execution context.)

The totals are read back through accessors such as `get_unaligned_count()` /
`get_unaligned_cost()` (grand totals) and their per-region variants
(`get_ram_unaligned_count`, `get_rom_unaligned_count`, `get_input_unaligned_count`,
`get_ram_stack_…`, `get_ram_no_stack_…`), each with an `_aligned_` counterpart.
They surface in the **statistics report**:

| flag | what it adds |
|------|--------------|
| `-X` / `--stats` | full statistics report (opcodes + memory usage) — carries the memory data |
| `--mem-stats` | the **MEM COST BY TYPE** section |
| `--mem-full-stats` | the **DETAILED MEM COST** section: per-region/per-shape `count`+`cost` rows (e.g. `RAM STACK unaligned 8B double read`) plus `TOTAL unaligned 1B/2B/4B/8B` rollups (requires `-X`) |
| `--save-stats` / `--ref-stats` | snapshot the numbers to a file and diff two runs |

(`-x` / `--legacy-stats` is the older step/usage report and does *not* carry this
breakdown; `-m` / `--log-metrics` is performance metrics.)

Running a guest with those flags produces (excerpt — the `MEM COST BY TYPE` and
`DETAILED MEM COST` sections, from a ZisK guest that does heavy byte↔limb
marshalling):

```text
$ ziskemu -e guest.elf -i input.bin -o out.bin -X --mem-stats --mem-full-stats
...
MEM COST BY TYPE                   COUNT       %            COST       %
------------------------------------------------------------------------
RAM STACK ALIGNED                    194   0.92%           3,298   0.69%
RAM NO STACK ALIGNED               1,644   7.77%          28,006   5.84%
RAM INIT                          13,960  65.98%         251,280  52.38%
ROM ALIGNED                          366   1.73%           5,124   1.07%
ROM INIT                           2,956  13.97%          41,384   8.63%
INPUT ALIGNED                         47   0.22%           1,363   0.28%
RAM NO STACK UNALIGNED             1,990   9.41%         149,220  31.11%
ROM UNALIGNED                          1   0.00%              39   0.01%
                         -----------------------------------------------
TOTAL ALIGNED                     19,167  90.59%         330,455  68.89%
TOTAL UNALIGNED                    1,991   9.41%         149,259  31.11%

DETAILED MEM COST                                        COUNT       %            COST       %
---------------------------------------------------------------------------------------------
RAM NO STACK unaligned 1B read                           1,443   6.82%          59,163  12.33%
RAM NO STACK unaligned 4B single 32-align read              45   0.21%           5,490   1.14%
RAM NO STACK unaligned 1B clean write                       97   0.46%           6,402   1.33%
RAM NO STACK unaligned 4B single 32-align clean write      304   1.44%          58,672  12.23%
RAM NO STACK unaligned 4B single 32-align dirty write      101   0.48%          19,493   4.06%
ROM unaligned 1B read                                        1   0.00%              39   0.01%
                                              -----------------------------------------------
TOTAL unaligned 1B                                       1,541   7.28%          65,604  13.68%
TOTAL unaligned 4B                                          450   2.13%          83,655  17.44%
...

DETAILED OFFSET BYTE MEMORY OPERATIONS
--------------------------------------
offset          0      1      2      3      4      5      6      7    total
reads         180    180    180    181    180    181    182    180    1,444
clean writes   12     12     12     12     12     11      9     17       97
dirty writes    0      0      0      0      0      0      0      0        0
```

`TOTAL UNALIGNED` (both `COUNT` and `COST`) is the headline figure; the
`DETAILED MEM COST` rows attribute it by region and access shape, and the final
`DETAILED OFFSET BYTE MEMORY OPERATIONS` table shows the distribution across the
eight byte offsets within an 8-byte word. (`RAM INIT` / `ROM INIT` are the
one-time zero-initialization accounting, not run-time accesses.)

**Precompile operand alignment.** The memory-based precompiles (keccak, sha256,
the arithmetic/EC precompiles, …) require their operand buffers to be **aligned**
— they read and write the state as aligned words, not through the byte-granular
unaligned path. An unaligned guest pointer must therefore never reach a precompile;
the library routines stage such operands in aligned buffers:

- The `.zisk` routines behind `zkvm_accelerators.h` marshal the EF byte-array ABI
  into library-owned, aligned scratch, so their operands are always aligned.
- The routines that used to hand the caller's result pointer straight to a precompile
  (`zisklib_overflowing_add256/sub256/mul256`, the `zkvm_u256_le_*` add, sub, mul,
  addmod, mulmod and negated signed-division results) now test the pointer and, when
  it is unaligned, compute into aligned scratch and DMA-copy the result out. The
  aligned path costs one extra step.
- `zkvm_keccak_f1600`, the bare keccak-f precompile emitted inline, DMA-copies an
  unaligned state into an aligned stack buffer and back.

The `unaligned_out` test covers 90 cases across `zkvm_zisklib.h`, `zkvm_u256_le.h`,
`zkvm_u256.h` and `zkvm_accelerators.h`.

**Compressed instructions.** The misaligned-instruction semantics of §10 are
defined for `IALIGN = 32` (no `C`), and the ZisK transpiler is able to decode
2-byte (compressed) instructions. As agreed with the EF, this is now controlled by
a dedicated **`compressed` cargo feature (off by default)** that gates RVC decoding
in both the RISC-V decoder and the transpiler (`zisk-riscv`:
`riscv_interpreter.rs` / `riscv2zisk_context.rs`), propagated up the build chain the
same way `float` is (through `zisk-transpiler-common`/`-riscv`, `zisk-rom-setup`,
`zisk-prover-backend`, and the `ziskemu`/`cargo-zisk` binaries):

- **Without** the feature (the default), the `C` extension is disabled: a 2-byte
  parcel is an illegal instruction and the run halts with error (`CHalt`), so ZisK
  is `IALIGN = 32` and matches the standard's "MUST NOT support C".
- **With** `--features compressed`, RVC instructions decode and execute normally,
  which is the configuration that fully meets a guest built for `rv64imac`.

This was validated end-to-end: an `rv64ima` (no-C) guest runs identically with and
without the feature, an `rv64imac` (with-C) guest is **rejected** (halt-with-error)
by a default `ziskemu` and **runs correctly** under `ziskemu --features compressed`.

**Observability: what the standard actually requires.** The open question we carried
here — whether an offline, emulator-reported count satisfies "during proving" — is
settled by the standard's own wording
([standards/riscv-target/target.md](https://github.com/eth-act/zkevm-standards/blob/main/standards/riscv-target/target.md)):

> "zkVMs **must** provide visibility into the number of unaligned memory accesses
> that occur during proof generation." At minimum, they **should** "expose a count of
> unaligned accesses **per proof** through **command-line output or log files**,
> though more granular metrics are encouraged."

So the bar is explicitly CLI or log output, per proof. There is no requirement for
the count to be an in-circuit quantity or a prover-internal artifact — our earlier
reading was stricter than the text. But it does rule out what we ship today:
`ziskemu -X --mem-stats` is a *separate, offline invocation*, not per-proof output
from proof generation.

**The count already exists on the proving path.** `MemCounters` tallies
`mem_align_counters.{full_5, full_3, full_2, read_byte, write_byte}`
(`state-machines/mem-common/src/mem_counters.rs`) unconditionally — it has to, because
that is how the MemAlign AIRs are sized — and a MemAlign operation *is* an unaligned
access. The existing `mem_align_stats` cargo feature gates only the per-instance
*printing*, not the counting. The totals are therefore available for free at planning
time, on both paths: `MemPlanner::plan()` (`state-machines/mem/src/mem_planner.rs`),
which receives every chunk's counters and is driven from
`emulator-asm/asm-runner/src/asm_mo_runner.rs`, and `gpu_count_and_plan.rs`, which
already aggregates the same five values for the GPU path.

**TODO 1 — report the count per proof.** Sum the five shapes across all chunks and
emit one line per proof, at a stable and greppable format, from both planning paths.
It must *not* sit behind a cargo feature: the standard says MUST, so it has to be on
by default. The per-shape breakdown comes for free and covers the "more granular
metrics are encouraged" clause. The exact log format becomes a documented interface
the EF may check against, so it should be fixed here once chosen.

**TODO 2 — align precompile operands: done** (October 2026; see *Precompile operand
alignment* above).

**Assessment: Partial** (base + M + little-endian + LP64 + unaligned support are met,
the `compressed` feature is implemented and unaligned operands no longer reach a
precompile; the observability question is resolved *against* us, and the required
per-proof CLI/log output, TODO 1, remains).

---

## 8. Standard termination semantics

**Standard.** Successful termination halts with a complete/valid trace and a
provable execution; `main` returning `0` is success, non-zero is abnormal
termination carrying that value as the error code. Failure halts and reports the
code to the host; the verifier either rejects failed-execution proofs (Type 1) or
accepts only proofs matching the expected code (Type 2). Runtimes must map
language-level aborts (Rust panic, C `abort()`) onto this interface.

Since [eth-act PR #34](https://github.com/eth-act/zkevm-standards/pull/34) (merged
2026-08-11) the standard also says that the verifier requirement is a **soundness
property enforced by the circuit**: a prover that declines to prove a failed run
does not satisfy it, because a malicious prover controls its own software.

**ZisK.** Implemented by PR #1375 (merged 2026-10-13):

- **Exit code.** Both `_start`s (`ziskasm/lang/c/src/_start.s` and `ziskos`) pass
  `main`'s return value in `a0` to the `SYS_exit` ecall. The BIOS exit handler
  (`core/src/rom_layout.rs`) checks it: `0` publishes the output and jumps to the
  `end` instruction at `ROM_EXIT` (`0x1004`); any other value executes a `halt`
  whose operand is the exit code.
- **Faults.** `unimp` and any other write to a read-only CSR (all three forms),
  `ebreak` and `c.ebreak` also halt as failed executions. Rust panics end in a
  trap (`ziskos-staticlib`'s panic handler emits `unimp`). C/C++ guests have no
  vendor `abort()` yet; see draft PR #46 in [Draft standards](#draft-standards).
- **Host report.** Both emulator entry points, `emulate` (`ziskemu`,
  `cargo-zisk execute`) and `compute_minimal_traces` (the first phase of proving),
  return `ZiskEmulatorErr::ExecutionFailed` with the reason (trap or exit code),
  step and pc, so the prover stops before generating any proof. `ziskemu` prints the
  exit code and exits with its low 8 bits (1 if those are 0).
- **Tests.** `emulator/tests/failed_execution.rs` runs the guests in
  `elf-regressions/failed_execution/` (each failure kind, plus an `exit_ok` control)
  through both entry points, with and without `compressed`.

**Why a failed run cannot be proven (Type 1).** Rejecting before proving is what an
honest prover does; the circuit gives the guarantee. Main's continuation bus requires
the last segment to end with `pc = END_PC_ADDR = 0x1004` (`state-machines/main/pil/main.pil`),
the `end` instruction that only a zero exit code reaches. No failure `halt` leads
there: the exit-code one in the BIOS jumps to itself, and a trap's
(`halt_with_error` in `riscv2zisk_context.rs`) continues to the next address in the
guest's code. A trace that ends in one cannot close the bus, so no valid proof of it
exists. This argument still has to be confirmed by the PIL owners.
ZisK provides no Type 2 (proof-of-failure) verifier.

**Assessment: Conformant (Type 1)**, pending the PIL owners' confirmation of the
argument above.

---

## 9. Static library and linker script

**Standard.** The vendor must ship a `.a` providing `_start` (stack/global-pointer
/ I/O init, C++ constructors, then `main`), the I/O functions, all accelerator
functions, and a GNU ld/LLD-compatible linker script that sets `_start` as the
entry, enforces W^X (executable `.text*`/`.init`/`.fini` separate from read-only
`.rodata*`), and exports `_heap_start`/`_heap_end`. `main` is `int main(void)`.

**ZisK.** ZisK provides the `ziskos` runtime (entry `_start`, boot/IO setup) for
Rust guests and, for the EF C ABI, the `ziskasm/lang/c` binding. `CMakeLists.txt`
builds the whole guest-side surface into one static library, **`zisklib_c`**:

| Requirement | Provided by |
|---|---|
| `_start` (gp/sp init, **C++ constructors**, `main`, **destructors**, termination) | `src/_start.s` |
| I/O functions | `src/zkvm_calls.s` + [`include/zkvm_io.h`](../../ziskasm/lang/c/include/zkvm_io.h) |
| Host randomness (`zkvm_random_u64`) | `src/zkvm_calls.s` + [`include/zkvm_random.h`](../../ziskasm/lang/c/include/zkvm_random.h) |
| Accelerator functions | `src/zkvm_calls.s` (19 `zkvm_*` + 27 `zkvm_u256_*`; `zkvm_keccak_f1600` is inline in the header), plus ZisK's 24 `zkvm_zisklib_*` |

Every entry is a zkvmcall thunk that the transpiler turns into a jump to the
hand-written `.zisk` routine, so a C guest compiles against the headers, links
`zisklib_c`, and runs the ziskasm implementations. `main` is `int main(void)`.

`_start` walks `[__init_array_start, __init_array_end)` before `main` and
`(__fini_array_end, __fini_array_start]` after it, so a C++ guest's static
constructors run in priority order and its destructors in reverse — the standard
requires `_start` to do this, and the linker script's `KEEP`'d `.init_array` is
inert without it. Verified with a guest carrying two constructors and two
destructors: the emitted order is `01 02 5A 82 81`.

There is a **single** guest linker script, `ziskbuild/zisk_linker_script.ld`
(embedded for Rust guests via `ZISK_LINKER_SCRIPT`, and referenced directly by the C
example). It sets `ENTRY(_start)` and lays out clean W^X segments via `PHDRS`:
`text FLAGS(1)` — **execute-only**, the form ZisK prefers for performance and the
stricter of the two the standard permits — with `rodata` `FLAGS(4)` and `data`/`bss`
`FLAGS(6)`, and it KEEPs `.init_array`/`.fini_array` for C++ ctors/dtors.

The script exports **`_heap_start` / `_heap_end`** as required. ZisK's own allocator
consumes the same bounds under its historical names (`_heap_bottom` / `_heap_top`,
with `_heap_size`), so the standard names are added alongside rather than renaming
them, leaving existing consumers untouched.

The two standard names are assigned **unconditionally**, not with `PROVIDE`. This
matters: `PROVIDE` only materializes a symbol that some input object *references*,
and a consumer that discovers the heap bounds by reading the ELF symbol table never
references them at link time — so under `PROVIDE` they are absent from `.symtab` for
exactly the guests the requirement exists to serve. Verified with a guest that does
not mention either name: both appear in `nm` output with the correct addresses.

**Packaging.** `ziskasm/lang/c/package.sh` builds the distributable artifact with a
cross toolchain file (`zisk-guest-toolchain.cmake`) and stages it:

```
dist/include/{zisklib.h,zkvm_accelerators.h,zkvm_io.h,zkvm_random.h,zkvm_u256.h}
dist/lib/libzisklib_c.a
dist/share/zisk/zisk_linker_script.ld
```

The archive and the script are installed together because the archive's `_start`
depends on symbols only the script defines. It is built `rv64ima`, not `rv64imac`,
so it contains no 16-bit encodings and is usable by a default (`IALIGN = 32`) ZisK —
verified by disassembling the archive. Verified end to end by linking both a C guest
and a C++ guest with a static object against **only** the installed artifacts, with
nothing from the source tree: the C guest reports `08 01 aa 5a` (read_input,
heap bounds, write_output) and the C++ guest `c7 5a` (constructor, then `main`).

One property to communicate with the artifact: it is **not standalone-functional**.
Every accelerator and I/O function in it is a zkvmcall thunk, so a guest linked
against it runs only under ZisK. A clean link proves nothing on its own.

**Assessment: Conformant** (`_start` incl. C++ constructors/destructors, the I/O
functions, every accelerator, a W^X linker script exporting `_heap_start`/`_heap_end`,
and a build that stages them as a distributable `.a` + script). `write_log` (draft
PR #27, §12) is in `zisklib_c` too. Draft PR #46 would add `abort` to the library's
required contents, and draft #36 the cost-scope functions; neither is in `zisklib_c` yet
(see [Draft standards](#draft-standards)).

---

## 10. Instruction-address-misaligned exception semantics

**Standard.** When the ISA would raise an instruction-address-misaligned
exception (a jump/branch to a misaligned target), the zkVM **must** abnormally
terminate — **no** recovery or continuation, and specifically **no** rounding the
target down to an aligned address. Applies to RISC-V without the C extension
(`IALIGN = 32`).

**ZisK.** ZisK has a `compressed` cargo feature, **off by default** (§7), that
controls whether the `C` extension is decoded. With it off ZisK is `IALIGN = 32`, so
"misaligned" means `pc % 4 != 0`, and the requirement is met at load time and on both
runtime paths.

**Load time.** `validate_entry_point` (§3) rejects an entry point that is not
IALIGN-aligned, and requires the same of every executable segment's start — the
transpiler decodes instructions from a segment's first byte, so a misaligned start
would desynchronize the whole segment. The alignment constant tracks the feature
(4 without `compressed`, 2 with), so the loader enforces `IALIGN = 32` exactly when
the ISA is at `IALIGN = 32`.

**Runtime — x86-64 assembly emulator (production).** Computed jumps resolve through
the `map_pc_*` branch table, which is indexed **per byte**:
`[map_pc_80000000 + (pc - ROM_ADDR)*8]`, one `.quad` per byte address
(`core/src/zisk_rom_2_asm.rs`). Every slot that is not a valid instruction address —
which includes every misaligned address — holds `.quad emu_end` instead of a real
target. A misaligned target therefore has its **own** slot and lands on the exit
path; it cannot alias onto a nearby valid entry, so rounding down is impossible by
construction rather than by check. The exit is also *unsuccessful*: `end = 1` is
written only when executing an instruction explicitly marked as terminating, so
leaving through a map slot leaves `end = 0`, distinguishable from a normal exit.

**Runtime — Rust emulator (`-l` only).** `ZiskRom::get_instruction(pc)`
(`core/src/zisk_rom.rs`) routes a misaligned `pc` to `rom_program_na_instructions`,
the non-aligned instruction table. Without `compressed` the transpiler never emits a
non-aligned instruction, so that table is **empty** (`num_program_na_instructions ==
0`); any misaligned `pc` fails its bounds check and **panics**, aborting immediately
rather than rounding, continuing, or fabricating an instruction.

> **This guarantee is contingent on `compressed` being off.** With the C extension
> enabled the non-aligned table is sized to the largest non-aligned address and
> pre-filled with `ZiskInst::default()`, so a misaligned `pc` below that bound would
> return a fabricated default instruction instead of aborting. That is the correct
> behavior at `IALIGN = 16`, where those addresses are legal — but it means the
> misaligned-abort semantics described here apply specifically at `IALIGN = 32`, i.e.
> with the default feature set.

**Assessment: Conformant** at the default `IALIGN = 32`: a misaligned entry point or
executable segment start is rejected at load time; at runtime a misaligned computed
jump exits through `emu_end` with `end = 0` in production and panics in the Rust
emulator. No rounding, no continuation, no successful termination.

---

## 11. Host randomness

**Standard.** Draft, [eth-act/zkevm-standards PR #42](https://github.com/eth-act/zkevm-standards/pull/42)
(`standards/host-randomness`). The vendor static library must export
`uint64_t zkvm_random_u64(void)` (`zkvm_random.h`). Each call returns 64 uniformly
distributed bits, independent of every other call, and cannot fail. The host draws
them from a cryptographically secure pseudorandom number generator, independently
for each execution, so the author of the guest input cannot predict them. They are
free witness values with no attestation: for a fixed input, the guest's public output
and exit code must be the same for every sequence the function can return (that part
is the guest's job). The host may force the values to replay a run; the guest has no
seeding function and cannot tell whether they were forced. The motivation is hash-map
seeding: without it, one set of colliding keys found offline would slow down every
prover running the same guest.

**ZisK.** Every execution has a **256-bit seed**, and `zkvm_random_u64` returns the
**SHAKE256 output stream of the seed's 32 bytes**, read as consecutive little-endian
64-bit words.

- **The seed.** It is decided once per task, before any emulator runs, because every
  emulation of the task (emulator threads, the ASM services, MPI ranks, distributed
  workers) must see the same values. It is drawn from the OS CSPRNG unless the host
  provides one with `--random-seed` / `ZISK_RANDOM_SEED` (every command that runs a
  guest) to replay a run; every run reports the seed it used, and a remote coordinator
  accepts a client random seed only if configured to allow it (`common/src/seed.rs`,
  `RandomSeed`).
- **Reaching the emulators.** The executor hands the seed to the active emulator: the
  Rust emulator in `EmuOptions.random_seed`, the ASM services through the control-input
  shared memory (words 4..7 hold the seed, word 8 is set once it is written), read when
  each execution starts. Standalone, `ziskemu` takes or draws the seed itself (`-v`
  prints it).
- **The guest side.** `zkvm_random_u64` is a zkvmcall thunk (`0x8B3`) to
  `ziskasm_zkvm_random_u64` (`ziskasm/zisklib/zkvm/random.zisk`). On the first call it
  reads the seed with the `FCALL_RANDOM_SEED` fcall (27) and absorbs it as SHAKE256 does;
  then each call returns the next word, and every 17 words one keccak-f precompile
  squeezes the next block. The common case costs 5 steps. Rust guests get the same
  routine through `ziskos` (`zkvm_random_u64`) or `zisklib::random_u64()`; it is also
  exported by `libziskos.a` and `zisklib_c`.
- **No seed, no values.** An emulator that was given no seed fails the fcall (and so
  the execution) rather than fall back to a fixed default.

The guest cannot reach the seed: there is no seeding function, and nothing in its
inputs or memory reveals whether the seed was forced.

**Not with hints.** A guest that uses `zkvm_random_u64` cannot be proven with
precompile hints. The hints come from a separate native run of the guest, which would
draw other values than the prover's, so the precompile calls it records could differ
from the ones the prover executes. This is enforced in code: the hints setup of the
ASM emulator rejects such a guest (`HINTS_WITH_RANDOM_ERROR`), and in a hints build the
native `zkvm_random_u64` panics.

**Validation.** `emulator/tests/zkvm_random.rs` runs a committed guest
(`elf-regressions/zkvm_random/`, from `ziskasm/lang/c/example/zkvm_random_guest.c`)
that draws 500 values and checks in the guest that they are distinct and that every
bit varies. The test compares the output with SHAKE256 of the seed computed by Python's
`hashlib`, covers the minimal-trace path and the hints rejection, and checks that a
missing seed fails. The same guest was run through the SDK with provided seeds on both
the Rust and the ASM emulator, and through the ASM server/client with the seed passed
in the control input.

**Assessment: Conformant.** The interface, the per-execution unpredictable seed and
replay without guest visibility are all in place. Two caveats:

- The standard's goal of following a recognized random bit generation specification
  cites NIST SP 800-90A. SHAKE256 keyed with a 256-bit seed is a standard XOF at
  256-bit security, but it is not one of the 800-90A DRBG constructions.
- A guest that uses it cannot use hints (see above).

---

## 12. Diagnostic logging

**Standard.** Draft, [eth-act/zkevm-standards PR #27](https://github.com/eth-act/zkevm-standards/pull/27),
which adds to the I/O interface (§4)
`void write_log(const uint8_t* utf8_bytes, size_t utf8_len)`. It writes UTF-8
diagnostic text that is not part of the public output and has no effect on the
execution. A zkVM may ignore it when proving.

**ZisK.** `write_log` is a zkvmcall (`0x8B4`) with no library routine: the transpiler
implements it itself, as one step that does nothing in the proof.

- **The guest side.** In
  [`zkvm_io.h`](../../ziskasm/lang/c/include/zkvm_io.h), on RISC-V, `write_log` is a
  `static inline` that emits the inline zkvmcall form: `csrs 0x8B4, rP` and
  `csrs 0x8E0, rL`, naming the registers that hold the pointer and the length. With
  `ZKVM_IO_CALLS` defined, the header only declares it, and the guest calls the thunk
  `csrs 0x8B4, x0; ret` in `zkvm_calls.s` (`zisklib_c`, §9), with the arguments in
  `a0`/`a1`. Rust guests get the thunk through `ziskos` (`write_log`), which
  `libziskos.a` also exports; natively it writes the text to stdout.
- **The transpiler.** For each call site, `add_zisk_code` emits one ZisK `flag`
  instruction whose `a` and `b` operands are the pointer and length registers, and
  which jumps past the `csrs` sequence (`emit_write_log` in
  `transpilers/riscv/src/riscv2zisk_context.rs`). Main constrains a `flag` only to
  `c = 0` and `flag = 1`, so the operands are free and the step is a no-op in the
  proof. No library routine is linked for it: `elf2rom` leaves zkvmcalls with no
  target out of the library set.
- **Printing.** The instruction's emulator function is `opc_write_log`
  (`core/src/ops_core_context.rs`) instead of `opc_flag`. When the run asks for it, it
  reads `b` bytes at address `a` and prints them (invalid UTF-8 is replaced, not
  rejected), then does what `opc_flag` does. The Rust emulator calls it through each
  instruction's function pointer, so other instructions pay nothing.
  `ziskemu --write-log` (`EmuOptions.write_log`) is the only way to turn printing on.
  Witness generation, minimal traces and the ASM emulator never print: the ASM emulator
  generates code from the opcode, which is a plain `flag`.

**Why it never changes the ROM.** Every call site becomes the same `flag` whether the
text is printed or not. Any change to the ROM would change the program's verification
key and every cache keyed by the ELF hash (the ASM binaries, the ROM merkle tree, the
`ProgramId`), so printing is decided only by the emulator at run time, never at
transpile time.

**Cost.** One Main step per inline call site, with no other state machine involved:
the registers read live in Main, and the text is never read when proving. The thunk
form costs 3 steps (call, `flag`, `ret`). Around the call there is the code that
puts the pointer and length in registers, and the inline asm's `"memory"` clobber,
which makes the compiler store pending writes before the call so `ziskemu` reads the
final text.

**Validation.** `emulator/tests/write_log.rs` runs a committed guest
(`elf-regressions/write_log/`, from `ziskasm/lang/c/example/write_log_guest.c` and
`write_log_thunk.c`) with six inline call sites and one thunk. It checks that each call
site is one `flag` that reads the right registers and jumps past the sequence, that
`write_log` needs no library routine, that the output is the same with and without
printing, and that the minimal-trace path runs it. `ziskemu --write-log` prints the
seven lines; the step count is the same without the flag.

**Assessment: Conformant** (draft). Logging has no effect on the execution, the public
output or the proof, and ZisK ignores it when proving, as the draft allows. The text
can only be seen from `ziskemu`, not from SDK executions or proofs.

---

## Draft standards

Open pull requests on eth-act/zkevm-standards as of 2026-10-27, beyond those already
covered above (#22 in §1b, #26 in §1, #42 in §11, #27 in §12):

| PR | Draft | What it requires | ZisK |
|----|-------|------------------|------|
| [#46](https://github.com/eth-act/zkevm-standards/pull/46) | Termination: `abort` | The vendor static library exports `_Noreturn void abort(void)`, which ends the run as a failed termination with a vendor-defined nonzero code, as if `main` had returned nonzero. Language runtimes map their failures onto it. | **Missing.** `zisklib_c` defines no `abort`. It would be a thunk that executes the exit ecall with a nonzero `a0`. |
| [#36](https://github.com/eth-act/zkevm-standards/pull/36) | Proving cost estimation | The emulator reports a deterministic scalar cost, linear in proving time, with a cost-model version. The library exports `zkvm_cost_scope_start/end(uint8_t)`, which cost nothing and are attributed inclusively; the emulator stops with a diagnostic on a misnested or still-open scope. | **Partial.** `ziskemu -X` reports a deterministic cost with a breakdown; there is no version and no cost scopes. |
| [#20](https://github.com/eth-act/zkevm-standards/pull/20) | Minimum memory resources | A minimum stack and heap size, reflected in the linker script. The values are still placeholders. | Nothing to do until values are set. ZisK's stack is 4 MB (§5). |

## Roadmap

- **§3:** reject `PF_X|PF_R` executable segments (ZisK is execute-only); check that a
  data load from a `PF_X` segment ends the run abnormally.
- **§7:** emit the per-proof unaligned-access count from the proving path (TODO 1).
- **§8:** have the PIL owners confirm the Type 1 argument (final pc bound to
  `END_PC_ADDR`).
- Follow the open drafts above, `abort` (#46) first: it is small and fixes how a C
  guest fails.
- Track the **host randomness** draft (eth-act PR #42) until it is merged, and revisit
  the generator if the standard ends up requiring an SP 800-90A DRBG.
- Track the **diagnostic logging** draft (eth-act PR #27) until it is merged.
- Replace ziskethone with **evm-asm** as the conformance vehicle once it is ready,
  re-running the same real-block validation against the C ABI.
