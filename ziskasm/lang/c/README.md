# ziskasm/lang/c — C bindings for the ZisK assembly library

C-language binding for the hand-written ZisK assembly routines under
[`ziskasm/zisklib/`](../../zisklib/). It is the C sibling of
[`ziskasm/lang/rust/`](../rust/): both reach the same routines through zkvmcalls,
and only the surface language differs.

## What it's for

A guest program calls the functions declared in [`include/`](include/): the
Ethereum Foundation zkVM standard (`zkvm_accelerators.h`, `zkvm_io.h`,
`zkvm_u256.h`) and ZisK's extensions to it (`zkvm_u256_le.h`, `zkvm_mem.h`,
`zkvm_evm.h`, `zkvm_zisklib.h`). Most are zkvmcall thunks
([`src/zkvm_calls.s`](src/zkvm_calls.s)), each a `csrs <id>, x0; ret`: during
transpilation (`elf2rom`) the `csrs` becomes a jump to the matching `.zisk`
routine, which follows the RISC-V calling convention and returns straight to the
caller. The others are expanded in the header itself (see [Coverage](#coverage)).
The `.zisk` code is merged into the ROM by `elf2rom`; it is **not** linked into the
ELF. The transpiler finds the calls by instruction, so the guest ELF may be
stripped.

## Why for ziskethone

ziskethone's C++ guest reaches all of its crypto, its 256-bit EVM arithmetic, its
memory operations and its I/O through these headers (branch `feature/zkvm-abi`),
instead of hand-written C++ ports of the same algorithms that talked to the ZisK
precompile CSRs directly. One `.zisk` implementation serves every guest.

## Layout

| Path | Purpose |
|------|---------|
| `include/`            | the public headers, one per family (see [Coverage](#coverage)) |
| `src/zkvm_calls.s`    | the zkvmcall thunks |
| `src/zkvm_mem.s`      | `zkvm_memset_any` and weak libc `memcpy`/`memmove`/`memcmp`/`memset` on the DMA ops |
| `src/_start.s`        | the guest entry point (EF §9) |
| `CMakeLists.txt`      | builds the `zisklib_c` static library + include dir |
| `package.sh`          | builds and stages the distributable archive |

## Integrate into a CMake guest

```cmake
add_subdirectory(/path/to/zisk/ziskasm/lang/c zisklib_c)
target_link_libraries(my_guest PRIVATE zisklib_c)
```

Then call the functions, e.g. keccak in an evmone-based guest:

```c
#include <zkvm_accelerators.h>
extern "C" union ethash_hash256 ethash_keccak256(const uint8_t* d, size_t n) noexcept {
    union ethash_hash256 h;
    zkvm_keccak256(d, n, reinterpret_cast<zkvm_keccak256_hash*>(h.bytes));
    return h;
}
```

## Packaging the static library

EF zkVM standard §9 asks the vendor to ship a `.a` providing `_start`, the I/O
functions and every accelerator, together with a linker script. `package.sh` builds
exactly that:

```bash
./package.sh                                        # -> dist/
PREFIX=/tmp/out ./package.sh                        # install elsewhere
TARBALL=1 ./package.sh                              # also produce dist.tar.gz
ZISK_TOOLCHAIN_PREFIX=riscv-none-elf- ./package.sh  # xPack toolchain
```

It stages:

```
dist/include/zkvm_*.h
dist/lib/libzisklib_c.a
dist/share/zisk/zisk_linker_script.ld
```

and then checks the archive really exports `_start`, `read_input`, `write_output`,
a sample accelerator and the four `mem*` routines before declaring success.

A guest then needs nothing from this source tree:

```bash
riscv64-unknown-elf-gcc -march=rv64ima_zicsr -mabi=lp64 -mcmodel=medany \
    -nostdlib -ffreestanding -O2 -Wl,--gc-sections \
    -Idist/include -T dist/share/zisk/zisk_linker_script.ld \
    -o guest.elf guest.c dist/lib/libzisklib_c.a
```

`--gc-sections` drops the zkvmcall thunks the guest never calls. Without it every
thunk stays in the ELF, and `elf2rom` then assembles the ZisK library even for a
guest that uses none of them.

Three things about this artifact are worth stating plainly, because none of them
behave like an ordinary static library:

- **It is not standalone-functional.** Every accelerator and I/O function in it is
  a zkvmcall thunk (`csrs <id>, x0; ret`, see [`src/zkvm_calls.s`](src/zkvm_calls.s))
  that the transpiler turns into a jump to a hand-written `.zisk` routine. A
  `ziskemu`/`cargo-zisk` built *without* `--features ziskasm` rejects a guest that
  uses one at transpile time. A clean link proves nothing on its own. The exception is `_start` and the `mem*` routines below, which are
  real code.
- **It defines `memcpy`/`memmove`/`memcmp`/`memset` (EF §2).** They are DMA
  precompile thunks (`memmove` is overlap-safe; it shares `memcpy`'s DMA op, which
  has memmove semantics) and they live in the same object as `_start`. Every guest
  links that object, so they always win symbol resolution regardless of link order.
  If a libc on the command line also contributes its `mem*`, the link fails with a
  multiple-definition error rather than silently falling back to a byte loop. Build
  guests with `-fno-builtin` if you want GCC to keep calls out-of-line instead of
  inlining small copies.
- **The linker script is not optional.** The archive's `_start` depends on symbols
  only the script defines (`_global_pointer`, `_init_stack_top`, `__init_array_*`,
  `_heap_start`/`_heap_end`), which is why the two are installed together.
- **It is built `rv64ima`, deliberately not `rv64imac`.** ZisK decodes the compressed
  extension only under the `compressed` cargo feature, which is off by default, so a
  default ZisK is `IALIGN = 32` and rejects 16-bit instructions. Override with
  `-DZISK_GUEST_ARCH` if you have enabled that feature.

A guest can be stripped: the transpiler finds zkvmcalls by instruction.

## Coverage

The library covers these families:

| Family | Count | Declared in | Implemented by |
|--------|-------|-------------|----------------|
| `zkvm_*` — EF accelerators | 20 | [`zkvm_accelerators.h`](include/zkvm_accelerators.h) | zkvmcall thunks in [`src/zkvm_calls.s`](src/zkvm_calls.s); `zkvm_keccak_f1600` is inline in the header |
| `zkvm_u256_*` — EF U256 | 27 | [`zkvm_u256.h`](include/zkvm_u256.h) | inline zkvmcalls, but for the division family and `exp`, which are calls, to `zkvm/u256.zisk`; every function also has a thunk in [`src/zkvm_calls.s`](src/zkvm_calls.s) (used with `ZKVM_U256_CALLS` or a declarations-only header); with `ZKVM_U256_INLINE`, inline C in the header except the division family |
| `zkvm_u256_le_*` — little-endian U256 (ZisK proposal, not EF) | 27 | [`zkvm_u256_le.h`](include/zkvm_u256_le.h) | inline zkvmcalls, but for the division family and `exp`, which are zkvmcall thunks, to `zkvm/u256_le.zisk`; with `ZKVM_U256_LE_INLINE`, inline C in the header except the division family |
| `read_input`/`write_output` — EF I/O | 2 | [`zkvm_io.h`](include/zkvm_io.h) | zkvmcall thunks in [`src/zkvm_calls.s`](src/zkvm_calls.s) |
| `zkvm_memcpy`/`memset`/`memcmp` — memory (ZisK extension, not EF) | 3 | [`zkvm_mem.h`](include/zkvm_mem.h) | inline in the header: one DMA marker each (one ZisK instruction with a constant size); a run-time memset fill calls `zkvm_memset_any` in [`src/zkvm_mem.s`](src/zkvm_mem.s), which also defines weak libc `memcpy`/`memmove`/`memcmp`/`memset` on the same DMA ops |
| `zkvm_evm_jumpdest_bitmap` — EVM JUMPDEST analysis (ZisK extension, not EF) | 1 | [`zkvm_evm.h`](include/zkvm_evm.h) | inline in the header (the jump_dest precompile marker); `ZKVM_EFAIL` when the precompile cannot take the arguments (unaligned, empty) |
| `zkvm_zisklib_*` — other ZisK library functions (not EF): 256-bit arithmetic on u64[4] limbs, secp256k1/r1 signatures, BN254/BLS12-381 pairings, maps and hashes to curves, BLS and KZG verify, modexp | 24 | [`zkvm_zisklib.h`](include/zkvm_zisklib.h) | zkvmcall thunks in [`src/zkvm_calls.s`](src/zkvm_calls.s), to the `zisklib_*` routines |

The `zkvm_u256_*` functions have three builds under the same ABI, chosen when
including `zkvm_u256.h`:
- by default, every function but the division family (`div`, `mod`, `divmod`,
  `sdiv`, `smod`, `sdivmod`) and `exp` is an inline zkvmcall (see below), and those
  seven are calls;
- with `ZKVM_U256_CALLS`, every function is only declared, as in the EF standard's
  header, and called through its thunk; the thunk of an inline zkvmcall expands the
  same routine body, so it gives the same results for about 10 more steps;
- with `ZKVM_U256_INLINE`, every function but the division family is a
  `static inline` C definition from [`zkvm_u256_inline.h`](include/zkvm_u256_inline.h).

Guest code doesn't change. The `.zisk` routines are the little-endian ones below
with every limb load and store turned into a `rev8` at the mirrored offset, which
costs nothing extra; the functions that feed a precompile or the division hint
convert their operands to little-endian scratch first. So they cost what the
little-endian ones do, plus about 8 to 12 steps for those conversions:
[`example/u256_bench_guest.c`](example/u256_bench_guest.c) measures `add` at 13
steps (C inline 40, thunk 23), the shifts at 30 to 32 (C inline 60 to 63), `div`
at 55 and `exp` at 333 (C inline 3,738).

[`zkvm_u256_le.h`](include/zkvm_u256_le.h) is the same 27 operations on four
little-endian 64-bit limbs instead of 32 big-endian bytes, the layout EVM
interpreters keep their stack in and the ZisK precompiles consume, so most
functions become a single precompile on the operands in place. It has two builds:
the `.zisk` routines by default, inline C with `ZKVM_U256_LE_INLINE`. By default every function but the division family
and `exp` (which stay thunk calls) is an inline zkvmcall: a `csrs` per argument
that the transpiler replaces by the routine's body on the registers the compiler
picked, with no call and no register saves (see `definitions/src/zkvmcall.rs`),
and a constant `ZKVM_EOK` status that the compiler folds away. The shifts expand
to about 100 instructions per call site, the others to 4..60. `add` costs 4 steps
as an inline zkvmcall and 5 as inline C, against 13 and 40 for the big-endian ABI.
[`example/u256_le_guest.c`](example/u256_le_guest.c) checks all 27 against the
big-endian ABI, including aliasing (and so, as long as the little-endian side is
unchanged, checks the big-endian one too), and `u256_bench_guest.c -DU256_LE` measures them
(add `-DZKVM_U256_LE_INLINE` for the inline implementation).

The zkvmcall IDs live in `definitions/src/zkvmcall.rs`.

The library also provides `_start` (`src/_start.s`), which EF §9 requires the
archive to ship, and the DMA-backed `memcpy`/`memmove`/`memcmp`/`memset` in the
same file (EF §2).

Adding a routine = a row in `definitions/src/zkvmcall.rs` (a new ID, never a
reused one), a `ZKVMCALL` line in `src/zkvm_calls.s`, a prototype in the header for
its family, and a `zkvmcall!` in the Rust binding.

## Status

In use: ziskethone's C++ guest runs on it. The benchmark scripts under
[`ziskasm/zisklib/scripts/benchmark/`](../../zisklib/scripts/benchmark/) check every
family against reference values (`check.sh` and the generated vector checks).
