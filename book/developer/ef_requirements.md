# EF zkEVM Standards Conformance

ZisK aims to conform to the Ethereum Foundation **zkEVM standards** published by
the `eth-act` working group at
<https://github.com/eth-act/zkevm-standards/tree/main/standards>. These standards
define a common contract between a zkVM and the guest programs it proves — the
RISC-V target, the ELF it loads, the memory layout, the I/O and termination
interfaces, and the C ABI for cryptographic accelerators (the EVM precompiles) —
so that a single guest can be built once and proven on any conforming zkVM.

This document records, standard by standard, how ZisK currently measures up.

> Note: the EF standards target a **RISC-V** guest. ZisK proves RISC-V programs;
> the ZisK-assembly (`.zisk`) routines mentioned below are ZisK's *internal*
> hand-written implementations that guest RISC-V symbols are redirected to at
> transpile time — they are an implementation detail of how ZisK accelerates the
> standard interfaces, not something a conforming guest is written in.

## How the standard interfaces are wired: stub + redirect

The guest is compiled by an **ordinary RISC-V toolchain** that knows nothing about
ZisK. It must nonetheless call the standard symbols — `zkvm_keccak256`,
`read_input`, `write_output`, the `ziskos_*` primitives, and so on. ZisK bridges
that gap with a **stub-and-redirect** scheme:

1. **A stub library (the "fake" library).** The guest links a small library that
   *defines* every standard symbol as a real, exported function with a throwaway
   placeholder body — for the C ABI this is `zkvm_stubs.c` (declared in
   `zkvm_accelerators.h` / `zisklib.h`); for Rust guests it is the `#[no_mangle]`
   stubs in the `zisklib` crate. These stubs exist only to satisfy the linker and
   to give each symbol a concrete address in the ELF; their bodies are never meant
   to run. (Many fill the output with a sentinel such as `0xBA`, which doubles as a
   negative control — see below.)

2. **The real implementations live in ZisK assembly.** The actual routines are
   hand-written `.zisk` files under `ziskasm/zisklib/` (e.g. `zkvm/keccak.zisk`).
   They are assembled — via `include_str!`, at ZisK build time — into a **reserved
   ROM/RAM region** (`ZISKLIB_ROM_ADDR`, carved out of the address space so it never
   collides with guest allocations) and merged into the guest's ROM.

3. **The redirect happens at transpile time.** When `elf2rom` converts the guest
   ELF into a ZisK ROM, it consults a fixed **`REDIRECTS` table**
   (`transpilers/common/src/elf2rom.rs`) of `(guest stub symbol → library routine)`
   pairs — for example `("zkvm_keccak256", "ziskasm_zkvm_keccak256")` and
   `("read_input", "zisklib_read_input")`. For each pair it looks up the stub's
   address (and size) in the ELF symbol table and, when transpilation reaches that
   address, emits a **static tail-jump into the library routine and skips the stub
   body**. Because it is a *tail* jump, the return address register (`ra`/`x1`) is
   untouched, so the `.zisk` routine's own `ret` returns straight to the guest's
   original caller. The guest source is unchanged and unaware — it simply called
   `zkvm_keccak256`.

Two consequences worth knowing:

- **Do not strip the guest ELF.** `elf2rom` resolves the stubs by name in the
  symbol table (`.symtab`); a stripped ELF has nothing to redirect, so the
  placeholder bodies run instead.
- **The placeholder is a built-in negative control.** If the redirect does *not*
  fire — the ELF was stripped, the symbol wasn't in the table, or (see below) ZisK
  was built without the `ziskasm` feature — the stub's sentinel body runs and the
  result is obviously wrong (e.g. a `0xBABA…` "hash"), rather than silently
  producing a plausible-but-unaccelerated value.

This whole mechanism is **gated behind the `ziskasm` cargo feature** (off by
default): without it, `elf2rom` neither assembles the library nor installs any
redirect, and the guest runs its own (stub or software) code unchanged. See
[Building and running the test](#building-and-running-the-test) for how to enable
it.

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
transpiler and the redirect table that maps the standard `zkvm_*`/`ziskos_*`
symbols onto the native `.zisk` accelerators) and **ziskethone** (the guest ELF,
built with the standard C ABI turned on).

**1. Build ZisK (`ziskemu`)** — from the `zisk` repository:

```sh
cargo build --release -p ziskemu --bin ziskemu --features ziskasm   # -> target/release/ziskemu
```

The **`ziskasm` feature is required** to get the redirect: it is off by default,
and without it `elf2rom` neither assembles the ZisK library nor redirects any
`zkvm_*`/`ziskos_*` symbol (a default `ziskemu` behaves like mainline, and an
EF-ABI guest run through it produces the stub's sentinel output instead of the
accelerated result — a handy negative control). The feature also enables the
emulator's `-z` ZisK-assembly path. The same feature is plumbed through the
proving pipeline, so `cargo build -p cargo-zisk --features ziskasm` redirects
`zisklib` guests during ROM generation and proving as well (Cargo feature
unification keeps every transpile path in the build consistent). (Not to be
confused with the unrelated `ziskasm` *feature* under `test-artifacts/programs/`,
which toggles the Rust `zisklib` for the dual-backend unit tests.)

**2. Build the ziskethone guest with the standard ABI** — from the `ziskethone`
repository. A RISC-V bare-metal C++ toolchain (xPack `riscv-none-elf-g++` 14.x)
must be on `PATH`:

```sh
# once: populate evmone into cpp-guest/build/_deps
cmake -S cpp-guest -B cpp-guest/build

# build the guest with the EF accelerator ABI enabled
cmake -S cpp-guest/zisk -B cpp-guest/zisk/build \
      -DCMAKE_TOOLCHAIN_FILE=$(pwd)/cpp-guest/zisk/toolchain.cmake \
      -DCMAKE_BUILD_TYPE=Release \
      -DZKVM_ABI="ZKVM_KECCAK;ZKVM_SHA256;ZKVM_SECP256K1;ZKVM_MODEXP;ZKVM_BLAKE2F;ZKVM_SECP256R1;ZKVM_BLS;ZKVM_KZG;ZKVM_BN254"
cmake --build cpp-guest/zisk/build -j8 --target zisk_eth_guest.elf
```

The `-DZKVM_ABI="…"` list is the switch that makes the guest **use the zkVM
interface**: each token replaces one crypto call site with the standard `zkvm_*`
symbol (e.g. `ZKVM_KECCAK` → `zkvm_keccak256`), which `elf2rom` then redirects to
the native `.zisk` routine. Building with an empty `-DZKVM_ABI=""` instead selects
ziskethone's own in-guest software/precompile paths — that build is the **native
reference** for the A/B check below.

**3. Run through `ziskemu`** and read the public output (the 32-byte block hash):

```sh
ZE=<zisk>/target/release/ziskemu
GUEST=<ziskethone>/cpp-guest/zisk/build/zisk_eth_guest.elf
$ZE -e "$GUEST" -i <block-input>.bin -o /tmp/out.bin -X    # -X prints step/cost stats
xxd -p -c32 /tmp/out.bin                                    # the block hash
```

Framed block inputs live in the `zisk-eth-client` repo, e.g.
`bin/guests/stateless-validator-ziskethone/inputs/*.bin` (already wrapped as
`[u64 LE length][payload]`).

**4. Confirm conformance (A/B).** Build a second guest with `-DZKVM_ABI=""`, run it
on the same block, and check the two `out.bin` block hashes are **byte-identical** —
proving the standard C ABI produces exactly the native result. (On real mainnet
blocks this holds for keccak/sha256/secp256k1/bn254; the rarer precompiles are
covered by the per-function golden-vector guests in `ziskasm/lang/c/example/`.)

## Summary

Legend: **Conformant** — meets the normative requirements; **Partial** — meets
the substance, with gaps or items still to confirm; **To verify** — believed to
conform but not yet audited against the spec.

| # | Standard | Assessment | One-line status |
|---|----------|------------|-----------------|
| 1 | [C interface for accelerators](#1-c-interface-for-accelerators) | **Conformant** | All 19 `zkvm_*` functions implemented natively in `.zisk`; runtime-validated on real blocks via ziskethone. |
| 2 | [Accelerated memory operations](#2-accelerated-memory-operations) | **Partial** | `memcpy`/`memcmp`/`memset` accelerated via DMA precompiles; `memmove` and the link-precedence guarantee to confirm. |
| 3 | [ELF loading and validation](#3-elf-loading-and-validation) | **Conformant** | `elf2rom` enforces header, PT_LOAD-only loading, zero-fill, W^X and entry-point validation. |
| 4 | [I/O interface](#4-io-interface) | **Conformant** | `read_input` / `write_output` implemented and redirected to the ZisK library. |
| 5 | [Memory layout restrictions](#5-memory-layout-restrictions) | **Conformant** | Standard is non-prescriptive; ZisK ships a vendor linker script defining its map. |
| 6 | [Memory safety guard regions](#6-memory-safety-guard-regions) | **Partial** | Null-pointer page and stack overflow trap (unmapped), but not via a named ≥4 kB stack-guard region. |
| 7 | [RISC-V target](#7-risc-v-target) | **Partial** | RV64IMA, little-endian, LP64, unaligned access supported; C-extension decoding and the unaligned-access counter to confirm. |
| 8 | [Standard termination semantics](#8-standard-termination-semantics) | **To verify** | `main` return maps to halt + host report; exact exit-code propagation to confirm. |
| 9 | [Static library and linker script](#9-static-library-and-linker-script) | **Partial** | `_start`, I/O, accelerators and a W^X linker script are provided; `_heap_start`/`_heap_end` export to confirm. |
| 10 | [Instruction-address-misaligned semantics](#10-instruction-address-misaligned-exception-semantics) | **To verify** | Misaligned targets must abnormally terminate with no rounding — behavior to confirm in `ziskemu`. |

---

## 1. C interface for accelerators

**Standard.** Defines the portable C API (`zkvm_accelerators.h`) through which a
guest reaches the zkVM's optimized implementations of the EVM precompiles.
Byte-encoded (big-endian) field elements, a `zkvm_status` return, byte-struct
operands.

**ZisK.** This is the standard we implement most completely. All **19** functions
of `zkvm_accelerators.h` are implemented as hand-written ZisK-assembly routines
under `ziskasm/zisklib/zkvm/*.zisk`, exposed under the exact `ziskasm_zkvm_*`
entry points and reached from a guest through the `elf2rom` symbol redirect of the
standard `zkvm_*` symbols:

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
48-byte fields) and returns `ZKVM_EOK`. The header and drop-in stubs live in
`ziskasm/lang/c/`.

**Validation.** Beyond per-function golden-vector tests, the full ABI was wired
into ziskethone and run on three real mainnet blocks; the block hash is
byte-identical to the native software path in every case, exercising
keccak/sha256/secp256k1 and BN254 end-to-end on-chain traffic.

**Assessment: Conformant.**

---

## 2. Accelerated memory operations

**Standard.** Acceleration of `memcpy` / `memmove` / `memset` / `memcmp` is
**optional**. If a zkVM provides them, they must be behaviorally identical to libc
for all inputs and alignments (including `n == 0`), assume no alignment, and be
guaranteed to win symbol resolution in the guest link (strong runtime definition
or `--whole-archive`; link order alone does not conform).

**ZisK.** ZisK accelerates bulk memory operations through dedicated **DMA
precompiles** (`dma_memcpy`, `dma_memcmp`, `dma_xmemset`), and the C guest link
routes `memcpy`/`memcmp`/`memset` to assembly shims backed by those precompiles
(ziskethone's `runtime.cpp` relies on this, building with `-fno-builtin` so the
calls stay out-of-line). Arbitrary alignment and `n == 0` are handled.

**Gaps to confirm.** (a) `memmove` acceleration and its overlap semantics; (b) the
formal link-precedence guarantee (that the ZisK definitions always win via a
strong/always-linked definition rather than link order).

**Assessment: Partial** (acceleration present and used; `memmove` and the
link-precedence guarantee to be confirmed).

---

## 3. ELF loading and validation

**Standard.** The loader must validate the ELF header (`\x7fELF`, ELFCLASS64,
little-endian, `EM_RISCV`, `ET_EXEC`), load only from `PT_LOAD` program headers,
zero-fill `p_memsz > p_filesz`, keep all addresses in range, reject
misaligned/overlapping segments, enforce W^X (reject `PF_W|PF_X`; executable
segments only `PF_X` or `PF_X|PF_R`), and validate the entry point (aligned,
inside a loaded executable segment). Invalid ELFs must be rejected with
diagnostics before any state reaches the prover.

**ZisK.** `transpilers/common/src/elf2rom.rs` (with the `elf_extraction` module)
builds the ROM exclusively from `PT_LOAD` segments, zero-fills BSS, checks every
segment/address lies within the ZisK addressable space (and errors otherwise,
e.g. the `PT_LOAD 0x0-0x0` rejection), and validates the entry point
(`validate_entry_point`): non-zero `e_entry`, correctly aligned, inside a loaded
executable segment — with an explicit diagnostic instructing the user to declare
`ziskos::entrypoint!`. Executable segments are `PF_X`/`PF_X|PF_R`; a `PF_X|PF_R`
segment currently produces a performance **warning** (allowed by the standard),
and the guest linker script emits a clean W^X layout (see §9).

**Assessment: Conformant** (header-field strictness, e.g. rejecting a non-RISC-V
`e_machine`, is worth an explicit audit but the substantive checks are in place).

---

## 4. I/O interface

**Standard.** Must provide `void read_input(const uint8_t** buf_ptr, size_t*
buf_size)` (returns a read-only pointer + length to the private input; never
fails; idempotent) and `void write_output(const uint8_t* output, size_t size)`
(successive calls concatenate into the public result; never fails).

**ZisK.** Both symbols are implemented in the ZisK library and wired via the
`elf2rom` redirects `("read_input", "zisklib_read_input")` and
`("write_output", "zisklib_write_output")`. Input is exposed at the memory-mapped
free-input region (`INPUT_ADDR = 0x4000_0000`); output is written to the public
output region (`OUTPUT_ADDR = 0xa041_0000`). Reads are non-failing and side-effect
free; successive `write_output` calls concatenate.

**Assessment: Conformant.**

---

## 5. Memory layout restrictions

**Standard.** Intentionally **non-prescriptive**: vendors define their own memory
map via a vendor-specific linker script rather than a single standardized map.
The requirement is that a vendor *provides* such a script (and, when linking
against libc, defines heap boundaries and region demarcations), and that programs
are linked with it.

**ZisK.** ZisK defines its map in `core/src/mem.rs` and ships the matching guest
linker script (`ziskasm/lang/c/example/zisk_guest.ld`, mirroring
`ziskbuild/zisk_linker_script.ld`):

| Region | Address |
|--------|---------|
| Free input | `0x4000_0000` |
| ROM (program) | `0x8000_0000` (len `0x0800_0000`) |
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

**ZisK.** The low address space (`0x0`–`0x0FFF`, and everything below `ROM_ADDR =
0x8000_0000`) is not mapped, so any null-pointer access is outside the ZisK
addressable space and aborts — satisfying the null-pointer trap. The stack lives
at the base of RAM (`0xa000_0000 … 0xa040_0000`, 4 MB) and grows down toward
`0xa000_0000`; below that is a large unmapped gap (down to the ROM region), so
stack overflow traps as an out-of-range access.

**Gap.** The stack overflow guard is provided *implicitly* by the unmapped gap
rather than by a *named, contiguous ≥4 kB guard region immediately adjacent to the
stack bottom*, which is how the standard phrases it. Functionally an overflow
aborts; matching the letter of the standard would mean designating an explicit
guard page.

**Assessment: Partial.**

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

**Items to confirm.**
- **Compressed instructions:** the standard says a conforming zkVM must *not*
  support C (to keep `IALIGN = 32`). Parts of the transpiler can decode 2-byte
  (compressed) instructions; whether that constitutes "supporting C" for the
  purpose of this standard, and whether it should be disabled, needs a decision.
- **Unaligned-access visibility:** the standard requires a reported count of
  unaligned accesses. `ziskemu`'s statistics (`-x`/`-X`) should be checked for
  (or extended with) this counter.

**Assessment: Partial** (base + M + little-endian + LP64 + unaligned support are
met; the C-extension and unaligned-counter items are open).

---

## 8. Standard termination semantics

**Standard.** Successful termination halts with a complete/valid trace and a
provable execution; `main` returning `0` is success, non-zero is abnormal
termination carrying that value as the error code. Failure halts and reports the
code to the host; the verifier either rejects failed-execution proofs (Type 1) or
accepts only proofs matching the expected code (Type 2). Runtimes must map
language-level aborts (Rust panic, C `abort()`) onto this interface.

**ZisK.** ZisK programs run from the boot thunk to a defined ROM exit; a normal
return halts the machine and produces the public output, and the `ziskos`
runtime maps panics/aborts to a halt. The zero/non-zero success convention and
end-to-end propagation of a non-zero `main` exit code to the host/verifier layer
should be audited against this standard.

**Assessment: To verify** (halt-and-report is in place; exact exit-code
propagation and the verifier-side Type 1/Type 2 handling to be confirmed).

---

## 9. Static library and linker script

**Standard.** The vendor must ship a `.a` providing `_start` (stack/global-pointer
/ I/O init, C++ constructors, then `main`), the I/O functions, all accelerator
functions, and a GNU ld/LLD-compatible linker script that sets `_start` as the
entry, enforces W^X (executable `.text*`/`.init`/`.fini` separate from read-only
`.rodata*`), and exports `_heap_start`/`_heap_end`. `main` is `int main(void)`.

**ZisK.** ZisK provides the `ziskos` runtime (entry `_start`, boot/IO setup) and,
for the EF C ABI, the `ziskasm/lang/c` binding (accelerator stubs + I/O). The
guest linker script sets `ENTRY(_start)` and lays out clean W^X segments via
`PHDRS`: `text` `FLAGS(5)` (`R+X`), `rodata` `FLAGS(4)` (`R`), and `data`/`bss`
`FLAGS(6)` (`R+W`), plus KEEP'd `.init_array`/`.fini_array` for C++ ctors/dtors.

**Gaps to confirm.** Export of the `_heap_start` / `_heap_end` symbols required by
the standard's application-allocator contract, and packaging the whole surface
(`_start` + I/O + all accelerators) as a single distributable `.a` archive.

**Assessment: Partial.**

---

## 10. Instruction-address-misaligned exception semantics

**Standard.** When the ISA would raise an instruction-address-misaligned
exception (a jump/branch to a misaligned target), the zkVM **must** abnormally
terminate — **no** recovery or continuation, and specifically **no** rounding the
target down to an aligned address. Applies to RISC-V without the C extension
(`IALIGN = 32`).

**ZisK.** Entry points are validated as instruction-aligned at load time (§3).
The remaining requirement is the *runtime* behavior of a computed jump to a
misaligned address during execution: it must abort rather than round or continue.
This path needs to be confirmed (and is coupled to the C-extension/`IALIGN`
decision in §7).

**Assessment: To verify.**

---

## Roadmap

- Confirm the **Partial**/**To verify** items above (memory-op link precedence and
  `memmove`; the explicit stack-guard region; the C-extension/`IALIGN` decision
  and the unaligned-access counter; exit-code propagation; `_heap_*` symbols and
  the packaged `.a`; misaligned-jump runtime behavior).
- Replace ziskethone with **evm-asm** as the conformance vehicle once it is ready,
  re-running the same real-block validation against the C ABI.
