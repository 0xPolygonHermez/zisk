# zisk-definitions

Shared low-level ZisK definitions, defined **once in Rust** and mechanically emitted to every
toolchain that needs them — Rust, C headers, PIL and assembly (GAS).

This document explains the *constants generator*: the small cluster of crates that
turns an annotated Rust `const` module into committed, multi-language source files.

## The crates

| Crate | Path | Goal |
|---|---|---|
| `zisk-definitions` | `.` | The leaf crate everyone depends on (published). `#![no_std]`, **zero deps**. Holds the hand-written definitions (syscalls, hints, …) and the committed *generated* files. |
| `zisk-definitions-source` | `source/` | The `#[constants]` *source*: one module per group plus the `ZISK_CONSTANTS` registry. Codegen input only (`publish = false`). |
| `zisk-definitions-macros` | `macros/` | The `#[constants]` / `#[emit]` proc-macros. Turn a normal `const` module into the verbatim consts **plus** a metadata table describing each value (`publish = false`). |
| `zisk-definitions-generator` | `generator/` | The rendering engine. Project-agnostic: takes metadata tables and renders/reconciles them to Rust/C/PIL/asm text on disk (`render` / `write` / `check`) (`publish = false`). |
| `zisk-definitions-sync` | `sync/` | Build-only driver (`publish = false`). Its `build.rs` reads the *evaluated* constants and calls the generator to (re)write the committed files. |

Why five crates? Each has one job, and only `zisk-definitions` is published: consumers
get plain generated `const`s and never depend on the codegen machinery. The engine
(`generator`) is deliberately reusable and knows nothing about ZisK. The macros are a
separate `proc-macro` crate (a Cargo requirement). The source lives in its own crate so
the published leaf stays dependency-free. And `sync` must be its own crate because of
the **build-script phase wall**: a crate's own `build.rs` runs *before* its library
compiles, so the code that reads the evaluated constants can't live in the source crate
— it lives in `sync`, which *build-depends* on `zisk-definitions-source` (so the
constants are compiled and evaluated first).

## Workflow

```
  #[constants] modules           #[constants] macro              generator engine
  in source/src/        ───────▶  keeps consts verbatim  ──────▶  renders + reconciles
  (source of truth)               + emits GROUP/EXPORTS            to src/generated/
                                    metadata table                 (driven by `sync`)
```

1. **Author** a `const` module in `source/src/` (one group per file), annotate it
   with `#[constants(...)]` (and per-const `#[emit(...)]`), and register it in the
   `ZISK_CONSTANTS` table in `source/src/lib.rs`.
2. The **`#[constants]` macro** keeps every `const` exactly as written (so `rustc`
   evaluates derived values like `SYS_ADDR = RAM_ADDR + STACK_SIZE`) and *additionally*
   emits a `GROUP` + `EXPORTS` metadata table describing each value.
3. On `cargo build`, the **`zisk-definitions-sync` build script** reads the evaluated
   `ZISK_CONSTANTS` and calls the generator, which renders and writes
   `src/generated/` — the Rust files at the top, `c/`, `pil/`, and `asm/` in subdirs.
   Only changed files are rewritten; files a group no longer produces are deleted.
4. **Downstream consumers** depend on `zisk-definitions` only, so they compile just the
   committed plain `const`s in `src/generated/` — no macros, no dependencies. The
   C / PIL / asm toolchains include the generated files from their respective subdirs
   in the repo (they're excluded from the published crate).
5. **CI** gates drift: it snapshots `definitions/src/generated`, runs
   `cargo build -p zisk-definitions-sync`, and diffs the two. If a constant changed
   without regenerating and committing, the build fails.

> The source and the generated output live in different crates, so regeneration never
> compiles the files it is about to overwrite: a deleted or broken `src/generated/` can
> always be rebuilt. The sync build only reruns when `source/src/` changes, so touch a
> file there to force it.

### Regenerating

```sh
cargo build -p zisk-definitions-sync   # or any full workspace build
```

Then commit whatever changed under `src/generated/`.

## Syntax

Annotate an **inline** `const` module. Every `pub const` stays a real Rust constant;
the attributes only control how it is *emitted*.

```rust
/// Program memory map. Shared by Rust, the C emulator, PIL, and the asm.
#[constants(group = "memory", to(rust, c, pil, asm), hex, fits = 32)]
pub mod memory {
    /// First global RW memory address.
    pub const RAM_ADDR: u64 = 0xa000_0000;

    /// Stack size — feeds SYS_ADDR; itself emitted nowhere.
    #[emit(internal)]
    pub const STACK_SIZE: u64 = 0x40_0000;

    /// First system RW memory address.        (derived: stamped as a literal,
    pub const SYS_ADDR: u64 = RAM_ADDR + STACK_SIZE; // with the expr kept as a comment)

    /// Precompile params — reaches PIL, but skip the C header.
    #[emit(skip(c))]
    pub const EXTRA_PARAMS_ADDR: u64 = SYS_ADDR + 0x0F00;
}
```

### `#[constants(..)]` — module-level defaults

| Argument | Meaning | Default |
|---|---|---|
| `group = "name"` | Logical group name; sets the output file base names. Must be an identifier; groups that emit Rust need distinct, non-keyword names (each is a module) | module ident |
| `to(rust, c, pil, asm)` | Targets to emit to | **required** |
| `hex` / `dec` | Number base for rendered values | `hex` |
| `fits = N` | Assert every value fits in `N` bits (a domain check) | the const's storage width |
| `c_prefix` / `pil_prefix` / `asm_prefix = "..."` | Prefix prepended to names in that target | none |
| `c_file` / `pil_file` / `asm_file = "..."` | Override the output file name: a plain file name ending in `.h` / `.pil` / `.inc` | `<group>.gen.h` / `.gen.pil` / `.gen.inc` |

`to(..)` is **required**. (In the sample, `memory` uses `to(rust, c, pil, asm)` and
produces `memory.gen.inc`; `opcodes` (`to(rust, pil)`) and `execution` (`to(rust, c, pil)`)
name no asm, so they produce none.)

### `#[emit(..)]` — per-const overrides

Overrides only the fields it names; everything else inherits the module defaults.

| Argument | Meaning |
|---|---|
| `internal` | Keep in the DAG (usable by derived consts) but emit to **no** target |
| `to(..)` | Replace the target set for this const |
| `skip(c, pil, ..)` | Remove targets from the inherited set |
| `hex` / `dec` | Radix override |
| `fits = N` | Override the fit bound; `no_fits` disables the check (e.g. a full-width mask) |
| `c_name` / `pil_name` / `asm_name = "..."` | Rename in that target (the group prefix still applies) |

### Supported types & behavior

- Types: `u8..=u128`, `i8..=i128`, `usize`/`isize` (treated as 64-bit — ZisK is a
  fixed 64-bit target — but kept as `usize`/`isize` in the generated Rust), and `&str`. Strings emit to Rust and C only (PIL/asm can't hold
  them); C gets octal escapes for every byte outside printable ASCII.
- **Visibility:** every exported const must be `pub` (the generated Rust is `pub`); a
  private helper const must be `#[emit(internal)]`.
- **Doc comments** on a const become one-line comments in every generated target.
- **Derived values** (any non-literal initializer) are emitted as the computed literal,
  with the source expression carried alongside as a provenance comment.
- Signedness comes from the value's type; the **fit check** rejects a value that
  overflows its `fits` bound (1–128 bits; default the storage width) at generation time.
- C and asm literals are at most 64 bits: a wider value (e.g. a `u128` above
  `u64::MAX`) is rejected for those targets; PIL and Rust take it as is.

### Generated output per target

| Target | File | Form |
|---|---|---|
| Rust | `<group>.rs` (+ a `mod.rs` aggregator) | `pub const NAME: ty = value;` — original ident, no prefix |
| C | `<group>.gen.h` | include-guarded `#define NAME ((type)value)` (`#include <stdint.h>`) |
| PIL | `<group>.gen.pil` | `const int NAME = value;` |
| asm | `<group>.gen.inc` | GAS `.equ NAME, value` |

Groups that share the same `*_file` are merged into one file, separated by
`--- group ---` comment headers.

## Routing outputs

The `sync` build is a list of **jobs** (in `sync/build.rs`), each mapping one source
constant table to its per-target output dirs. Two axes of flexibility:

- **Multiple sources.** Add another unpublished source crate exposing its
  `#[constants]` table (`pub const CONSTANTS: …`), add it as a build-dependency of
  `sync`, and push a `Job`. Keep sources out of published crates: a published crate
  would have to depend on, and so publish, the macros and the engine. Each job needs
  its own dir per target (different targets may share one): a job reconciles a dir
  against only its own files, so the `sync` build refuses jobs that overlap.
- **Multiple destinations.** Each target's dir has a mode:
  - `Exclusive` — a dedicated, generated-only dir (like `src/generated/`); files of
    that extension no longer produced are deleted.
  - `Shared` — a dir that also holds hand-written files of the same extension (e.g.
    emitting an asm include straight into a folder the emulator maintains by hand).
    Only generated files (first line opens with the `@generated by` banner) are ever
    written or removed there, so hand-written siblings are safe.

Drift-checking (`check`) honors the same modes, so a `Shared` dir isn't flagged for
its hand-written files.
