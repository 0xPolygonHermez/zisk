# zisk-transpiler-common

Entry point of the ZisK transpilers: turns a guest program into a `ZiskRom`, the instruction ROM
that the ZisK emulator and prover execute. It detects the guest format from the file's magic bytes
and calls the matching transpiler:

| Input | Handled by |
|---|---|
| WebAssembly (`\0asm`) | `zisk_transpiler_wasm::wasm2rom` |
| ziskbin ELF (`e_machine == EM_ZISK`, a ROM prebuilt by `ziskasm`) | `zisk_core::ziskbin::ziskbin2rom` |
| any other ELF (RISC-V) | `zisk_riscv::elf2rom` |

```rust
use zisk_transpiler_common::ZiskTranspiler;

let rom = ZiskTranspiler::new(&program_bytes).run()?;
```

| Item | What it does |
|---|---|
| `program2rom(bytes)` | Transpiles a guest program into a `ZiskRom` |
| `program2romfile(bytes, asm_file, …)` | Same, then saves the ROM as x86-64 assembly |
| `ZiskTranspiler` | Holds the program bytes; `run()` and `runfile(…)` call the two functions above |

It replaces the deprecated `zisk-transpiler-riscv` crate's `Riscv2zisk`; see that crate's README to
migrate.

## Moved API

Up to 1.3 this crate held the RISC-V ELF transpiler. That code is RISC-V specific, so in 1.4 it
moved to `zisk-riscv`, and these items are no longer exported here:

| Removed (`zisk_transpiler_common::…`) | Replacement |
|---|---|
| `elf2rom(elf)` | `zisk_riscv::elf2rom(elf)` (RISC-V ELF only), or `program2rom(bytes)` (any format) |
| `elf2romfile(elf, asm_file, …)` | `program2romfile(bytes, asm_file, …)` (same arguments) |
| `elf_extraction` module | `zisk_riscv::elf_extraction` |
| `elf_extraction::ElfPayload` | `zisk_riscv::elf_extraction::ElfPayload` |
| `elf_extraction::collect_elf_payload_from_bytes` | `zisk_riscv::elf_extraction::collect_elf_payload_from_bytes` |
| `elf_extraction::validate_entry_point` | `zisk_riscv::elf_extraction::validate_entry_point` |
| `elf_extraction::merge_ro_sections` | `zisk_riscv::elf_extraction::merge_ro_sections` |
| `elf_extraction::get_symbol_addresses` | `zisk_riscv::elf_extraction::get_symbol_addresses` |
| `elf_extraction::get_symbol_addresses_from_bytes` | `zisk_riscv::elf_extraction::get_symbol_addresses_from_bytes` |
