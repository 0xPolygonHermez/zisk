# ziskasm/lang/c — C bindings for the ZisK assembly library

C-language binding for the hand-written ZisK assembly routines under
[`ziskasm/zisklib/`](../../zisklib/). It is the C sibling of
[`ziskasm/lang/rust/`](../rust/): the flat ABI, the `ziskos_*` symbol names, and
the redirect mechanism are identical — only the surface language differs.

## What it's for

A guest program calls the `ziskos_*` functions declared in
[`include/zisklib.h`](include/zisklib.h). Each is a raw C-ABI **stub** with a
stable, un-mangled symbol and a placeholder body (in
[`src/zisklib_stubs.c`](src/zisklib_stubs.c)). During transpilation (`elf2rom`),
the stub's entry is **redirected** to the matching `zisklib_*` routine assembled
from `ziskasm/zisklib/*.zisk`, so the ziskasm implementation runs in the guest's
place. The `.zisk` code is injected into the ROM by `elf2rom`; it is **not**
linked into the ELF.

The redirect is keyed purely on the ELF symbol name — see the `REDIRECTS` table
in [`transpilers/common/src/elf2rom.rs`](../../../transpilers/common/src/elf2rom.rs).
That makes it **language-agnostic**: a C or C++ caller of `ziskos_keccak` is
redirected exactly like the Rust binding's caller.

## Why for ziskethone

ziskethone's `cpp-guest/zisk/*_zisk.cpp` files are hand-written C++ *ports* of the
same crypto (secp256k1, secp256r1, bn254, bls12_381, modexp, keccak, sha256, …),
each a "faithful port of zisklib" that talks directly to the ZisK precompile CSRs.
This binding lets the C++ guest instead call the **single shared** `.zisk`
implementation, so those ports can be retired in favour of one source of truth.

## Layout

| Path | Purpose |
|------|---------|
| `include/zisklib.h`   | public prototypes for every redirectable `ziskos_*` entry + ABI notes |
| `src/zisklib_stubs.c` | placeholder stub bodies (one exported symbol each) |
| `CMakeLists.txt`      | builds the `zisklib_c` static library + include dir |

## Integrate into cpp-guest (CMake)

```cmake
# in cpp-guest/zisk/CMakeLists.txt
add_subdirectory(/path/to/zisk/ziskasm/lang/c zisklib_c)
target_link_libraries(zisk_eth_guest PRIVATE zisklib_c)
```

Then replace a port's body with a call, e.g. keccak:

```c
#include <zisklib.h>
// was: syscall_keccakf + evmone sponge in keccak_zisk.cpp
extern "C" union ethash_hash256 ethash_keccak256(const uint8_t* d, size_t n) noexcept {
    union ethash_hash256 h;
    ziskos_keccak(d, n, (uint8_t*)h.bytes);   // redirected to ziskasm_zkvm_keccak256
    return h;
}
```

## Rules that keep the redirect working

- **Stable symbols, real bodies.** Stubs are `__attribute__((noinline, used))`
  and never `static`, so each has an address and a nonzero size for `elf2rom` to
  find and measure.
- **Every argument is touched** (via a `TOUCH()` inline-asm sink). The redirected
  routine reads its arguments from `a0..a7`; a body that ignored an argument could
  let the optimizer drop that register's setup at the call site.
- **Do not `--strip-all` the guest ELF.** `elf2rom` resolves the stubs by name in
  `.symtab`, which must survive to the transpile step.
- **No linker-script change.** The `.zisk` implementation is merged into the ROM
  by `elf2rom`, not linked into the ELF.

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
dist/include/{zisklib.h,zkvm_accelerators.h,zkvm_io.h,zkvm_u256.h}
dist/lib/libzisklib_c.a
dist/share/zisk/zisk_linker_script.ld
```

and then checks the archive really exports `_start`, `read_input`, `write_output`,
a sample accelerator and the four `mem*` routines before declaring success.

A guest then needs nothing from this source tree:

```bash
riscv64-unknown-elf-gcc -march=rv64ima -mabi=lp64 -mcmodel=medany \
    -nostdlib -ffreestanding -O2 \
    -Idist/include -T dist/share/zisk/zisk_linker_script.ld \
    -o guest.elf guest.c dist/lib/libzisklib_c.a
```

Three things about this artifact are worth stating plainly, because none of them
behave like an ordinary static library:

- **It is not standalone-functional.** Every accelerator and I/O symbol in it is a
  stub whose entry `elf2rom` rewrites to a hand-written `.zisk` routine at transpile
  time. Link it and run the result through a `ziskemu`/`cargo-zisk` built *without*
  `--features ziskasm` and you reach the stub bodies, which fail hard by design. A
  clean link proves nothing on its own. The exception is `_start` and the `mem*`
  routines below, which are real code.
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

Do not `--strip` the linked guest: `elf2rom` resolves the stubs by symbol name.

## Coverage

`REDIRECTS` holds **77** entries across three independent symbol families, and
this header covers only the first. The families are siblings, not layers: where
they overlap they target the *same* routine rather than calling through one
another — `ziskos_keccak` and `zkvm_keccak256` both resolve to
`ziskasm_zkvm_keccak256`, likewise `sha256` and `blake2b_compress`/`blake2f`.

| Family | Count | Declared in | Stubs in |
|--------|-------|-------------|----------|
| `ziskos_*` — ZisK flat ABI | 27 | [`zisklib.h`](include/zisklib.h) | [`src/zisklib_stubs.c`](src/zisklib_stubs.c) |
| `zkvm_*` — EF accelerators | 20 | [`zkvm_accelerators.h`](include/zkvm_accelerators.h) | [`src/zkvm_stubs.c`](src/zkvm_stubs.c) |
| `zkvm_u256_*` — EF U256 | 27 | [`zkvm_u256.h`](include/zkvm_u256.h) | [`src/zkvm_stubs.c`](src/zkvm_stubs.c) |

Plus 3 entries outside those three families: the EF I/O pair `read_input` /
`write_output` (declared in [`zkvm_io.h`](include/zkvm_io.h), stubbed in
`src/zkvm_stubs.c`, redirected to `zkvm_io.zisk`) and `modexp_u64_c` (declared in
`zisklib.h`). The library also provides `_start` (`src/_start.s`), which is not a
redirect entry but is part of the surface EF §9 requires the archive to ship, and
the DMA-backed `memcpy`/`memmove`/`memcmp`/`memset` in the same file (EF §2).

The `ziskos_*` set is: `add` (demo), `keccak`, `sha256`, `blake2b_compress`, the
`*256` integer/modular ops, secp256k1 (ecdsa verify/recover, schnorr), secp256r1
(ecdsa verify), bn254 pairing check, and bls12_381 (pairing check,
map/hash-to-curve, BLS verify, KZG proof).

Adding a new routine = a `REDIRECTS` row + a prototype/stub pair **in the header
for that family** (and, for `ziskos_*`, in the Rust binding). A new EF entry does
not belong in `zisklib.h`.

## Status

Scaffold. The header + stubs compile clean for the host and for `rv64ima`
(`riscv*-elf-gcc`). Wiring individual cpp-guest precompiles to these entries, and
validating each against the existing C++ ports, is the next step.
