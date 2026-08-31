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
    ziskos_keccak(d, n, (uint8_t*)h.bytes);   // redirected to zisklib_keccak
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

## Coverage

Every entry in the `elf2rom` `REDIRECTS` table has a prototype here: `add`
(demo), `keccak`, `sha256`, `blake2b_compress`, the `*256` integer/modular ops,
secp256k1 (ecdsa verify/recover, schnorr), secp256r1 (ecdsa verify), bn254
pairing check, bls12_381 (pairing check, map/hash-to-curve, BLS verify, KZG
proof), and `modexp_u64_c`. Adding a new routine = add a `REDIRECTS` row + a
prototype/stub pair here (and in the Rust binding).

## Status

Scaffold. The header + stubs compile clean for the host and for `rv64ima`
(`riscv*-elf-gcc`). Wiring individual cpp-guest precompiles to these entries, and
validating each against the existing C++ ports, is the next step.
