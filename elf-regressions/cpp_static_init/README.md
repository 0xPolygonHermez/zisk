# cpp_static_init

A C++ guest whose globals have non-trivial constructors and destructors, used to
pin down how the program-segment based ELF interpreter
([`transpilers/common/src/elf_extraction.rs`](../../transpilers/common/src/elf_extraction.rs))
handles the structures that C++ static initialization produces — and that the
constructors actually run, in the right order.

What it covers, beyond what an assembly test can reach:

| Construct | What it exercises |
|---|---|
| `.init_array` / `.fini_array` | placed by the linker script in the **read-only ROM** `PT_LOAD` segment, so the ctor function pointers must be loadable from ROM |
| `init_priority(101/102)` | `SORT_BY_INIT_PRIORITY` ordering survives into the loaded image |
| vtables, `const` tables | `.rodata` reads issued *during* static init |
| pointers between globals | relocated absolute pointers in `.data` |
| function-local `static` | `.bss` guard variable — only works if the writable segment's `p_memsz > p_filesz` zero-fill tail is honoured |
| `__cxa_atexit` destructors | reverse-order teardown after `main` returns |

Each check writes one word to the ZisK output area, so the emulator's output is
a full trace of what ran and when.

## Running it

The verification lives in
[`emulator/tests/cpp_static_init.rs`](../../emulator/tests/cpp_static_init.rs),
which asserts both the extracted ELF payload (code and `.init_array` in ROM,
writable data in RAM) and the exact output trace:

```bash
cargo test -p ziskemu --test cpp_static_init
```

It reads the committed ELF at `../prebuilt-elfs/cpp_static_init.elf`, so no C++
cross toolchain is needed to run the test. The ELF is also picked up by the
shell harness (`./scripts/test.sh emu cpp_static_init`), which only smoke-tests
that it executes.

## Regenerating the ELF

After changing `main.cpp` or `start.S`:

```bash
./build.sh                              # links with rust-lld (production linker)
LD=riscv64-unknown-elf-ld ./build.sh    # cross-check with GNU ld
```

then update `EXPECTED` in the Rust test if the trace changed. `build.sh` needs
`riscv64-unknown-elf-g++` (Debian/Ubuntu: `gcc-riscv64-unknown-elf`) and links
against the real [`ziskbuild/zisk_linker_script.ld`](../../ziskbuild/zisk_linker_script.ld),
so a change to that script is reflected here.

## Why it is self-contained

`start.S` provides its own `_start` and `main.cpp` its own `__cxa_atexit` /
`__cxa_finalize_all`, mirroring what ziskos does
([`ziskos/entrypoint/src/lib.rs`](../../ziskos/entrypoint/src/lib.rs)). The
subject under test is the ELF interpreter's treatment of these segments, so the
guest deliberately does not depend on the ziskos runtime — that keeps the
committed ELF stable and the failure signal unambiguous.

## Known gaps this test does *not* cover

Verified limitations of the current C++ path, none of which are ELF-interpreter
bugs (see [`ziskos-staticlib/README.md`](../../ziskos-staticlib/README.md)):

- `thread_local` fails at link time — the linker script declares no `PT_TLS`.
- `.preinit_array` is never walked by the runtime.
- Legacy `.ctors` / `.dtors` (pre-`.init_array` toolchains) are silently ignored.
- `__cxa_atexit` holds at most 64 destructors; registrations past that are dropped.
