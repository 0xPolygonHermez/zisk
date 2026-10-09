# zisk-transpiler-riscv

> **Deprecated:** use [`zisk-transpiler-common`](https://crates.io/crates/zisk-transpiler-common)
> instead. The `zisk-transpiler-riscv` binary is **not** deprecated.

This crate was the ZisK transpiler's entry point: `Riscv2zisk` turned a guest program into a
`ZiskRom`. That code now lives in `zisk-transpiler-common`, which detects the guest format (RISC-V
ELF, ziskbin ELF or WebAssembly) and calls the matching transpiler.

The crate keeps its name so existing code still compiles: its library API is a thin layer on top
of `zisk-transpiler-common`, marked `#[deprecated]` since 1.4.0. Every use produces a deprecation
warning, and it will not get new features.

## Deprecated API

To migrate, depend on `zisk-transpiler-common` instead of `zisk-transpiler-riscv`, and replace:

| Deprecated (`zisk_transpiler_riscv::…`) | Replacement (`zisk_transpiler_common::…`) |
|---|---|
| `Riscv2zisk` | `ZiskTranspiler` |
| `Riscv2zisk::new(bytes)` | `ZiskTranspiler::new(bytes)` |
| `Riscv2zisk::run()` | `ZiskTranspiler::run()` |
| `Riscv2zisk::runfile(…)` | `ZiskTranspiler::runfile(…)` (same arguments) |
| `Riscv2zisk::elf` field | `ZiskTranspiler::program` field |
| `program2rom(bytes)` | `program2rom(bytes)` |

```rust
// Before
use zisk_transpiler_riscv::Riscv2zisk;
let rom = Riscv2zisk::new(&elf).run()?;

// After
use zisk_transpiler_common::ZiskTranspiler;
let rom = ZiskTranspiler::new(&elf).run()?;
```

## Behavior changes since 1.3

- `Riscv2zisk::run()` and `Riscv2zisk::runfile()` accept every guest format the replacements do:
  RISC-V ELF, ziskbin ELF and WebAssembly. In 1.3 they only accepted RISC-V ELF.
- The `zisk-transpiler-riscv` binary accepts the same formats:

  ```sh
  zisk-transpiler-riscv <program_file> <x86-64_asm_file> <generation_method>
  ```
