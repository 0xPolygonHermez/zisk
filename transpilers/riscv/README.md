# zisk-riscv

The RISC-V transpiler of ZisK: decodes RISC-V instructions, translates them into ZisK instructions,
and turns a RISC-V ELF into a `ZiskRom` (`elf2rom`).

To transpile a guest program of any format (RISC-V ELF, ziskbin ELF or WebAssembly), use
`zisk-transpiler-common` instead: it detects the format and calls this crate for RISC-V.

## Moved API

In 1.4, code that is not RISC-V specific moved out of this crate, and the RISC-V ELF transpiler
moved in from `zisk-transpiler-common`.

These items moved to `zisk-core`, since every ROM producer uses them, and are no longer exported
here:

| Removed (`zisk_riscv::…`) | Replacement |
|---|---|
| `add_entry_exit_jmp(rom, addr)` | `zisk_core::rom_layout::add_entry_exit_jmp(rom, addr)` |
| `add_end_and_lib(rom)` | `zisk_core::rom_layout::add_end_and_lib(rom)` |

These items are new here, moved from `zisk-transpiler-common`:

| New (`zisk_riscv::…`) | Was (`zisk_transpiler_common::…`) |
|---|---|
| `elf2rom(elf)` | `elf2rom(elf)` |
| `elf_extraction` module (`ElfPayload`, `collect_elf_payload_from_bytes`, `validate_entry_point`, `merge_ro_sections`, `get_symbol_addresses`, `get_symbol_addresses_from_bytes`) | `elf_extraction` module |

`elf2romfile` did not move: use `zisk_transpiler_common::program2romfile`, which takes the same
arguments.
