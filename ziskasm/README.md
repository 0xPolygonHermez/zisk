# ziskasm — ZisK assembly language & toolchain

`ziskasm` lets you write ZisK programs directly as `.zisk` assembly (bypassing the
Rust → RISC-V → ZisK path), assemble them into a `ZiskRom`, and run them on the
emulator. This document records the design, decisions, and roadmap; it is meant
to be kept current as the feature evolves.

## Layout

| Path | What it is |
|------|-----------|
| [`ziskasm.md`](ziskasm.md) | The language **specification** (syntax, sources, storage, jump/setpc, call/ret, definitions, labels). |
| [`src/parser.rs`](src/parser.rs) | Line-oriented `.zisk` parser → instruction AST. |
| [`src/assembler.rs`](src/assembler.rs) | Two-pass assembler: AST → `ZiskRom`. |
| [`bin/zisk2zisk.rs`](bin/zisk2zisk.rs) | Binary: `.zisk` → x86-64 NASM (the fast-emulator source), mirroring `riscv2zisk`. |
| [`examples/doubler/`](examples/doubler/) | Worked example: `ziskos.zisk` (explicit launcher) + `doubler.zisk` (program) + `input.bin`. |
| [`examples/doubler-min/`](examples/doubler-min/) | Same program as a single `main:` file — the assembler synthesizes the launcher. |
| [`tests/doubler.rs`](tests/doubler.rs) | End-to-end tests: assemble each example, run it, assert output. |

## How to run

```
cargo run -p ziskemu -- -z ziskasm/examples/doubler -i ziskasm/examples/doubler/input.bin -c
```

`-z` accepts a single `.zisk` file or a directory of `.zisk` files. It is mutually
exclusive with `-e` (ELF) and `-r` (ROM). Directory order does not matter — the
assembler resolves the `_start` entry by label.

To compile `.zisk` source to the x86-64 fast-emulator assembly (what `riscv2zisk`
does for ELFs), use the `zisk2zisk` binary — same arguments as `riscv2zisk`,
except the input may be a file or a directory:

```
cargo run -p ziskasm --bin zisk2zisk -- ziskasm/examples/doubler out.asm --gen=0
```

## Architecture

The assembler mirrors `transpilers/common/src/elf2rom.rs`, but the program
instructions come from the `.zisk` parser instead of the RISC-V transpiler:

1. Parse each file → a flat list of instructions (labels attached to the
   instruction they precede).
2. **Pass 1**: assign each instruction a ROM address (`ROM_ADDR + 4*index`) and
   build the label → address table; the `_start` label is the program entry.
3. **Pass 2**: encode each instruction through `ZiskInstBuilder` (the *same*
   encoder the RISC-V transpiler uses, so emulator compatibility is guaranteed),
   resolving `j(label)`/`call label` targets to pc-relative offsets.
4. Build the `ZiskRom`: `add_end_and_lib` → insert program insts → 
   `add_entry_exit_jmp(_start)` → `optimize_instruction_lookup`.

## Key design decisions

- **ROM built directly from `.zisk`**, like `.elf` transpilation — no ROM binary
  serialization / on-disk format.
- **Input is seeded by the emulator** (`emu_context`): `INPUT_ADDR` has an 8-byte
  zero header then the raw input file at `INPUT_ADDR+8` (8-byte aligned).
- **Output** is read from `OUTPUT_ADDR` (32 u64 = 64 u32 public words) by the BIOS
  finalization / `get_output_*`.
- **Entry point**: the `_start` label. The assembler places the file that defines
  `_start` first, so `_start` lands at `ROM_ADDR` (0x80000000) — matching the ELF
  convention that the entry point is the program base, and what the fast emulator
  expects. Source files may be given/collected in any order (a `-z <dir>` run
  sorts them by name), so this reordering is done automatically; the `_start`
  file must *begin* with the `_start` label.
- **Automatic launcher**: if a program defines no `_start`, the assembler
  synthesizes one around its entry label (`main`, else `_zisk_main`) — set gp/sp,
  `call` the entry, `ret_to_bios` — mirroring `ziskos::_start`. So a program can be
  just its own code plus a `main:` label (see `examples/doubler-min`).
- **Instruction stride is fixed at 4 bytes** (the assembler owns addresses).
- **Exit via a static jump to the BIOS** (the intended final model): the BIOS
  enters `_start` with the address of its output-finalization code in `r1` (a
  `call`), then reads `OUTPUT_ADDR` and ends once the program jumps back there.
  The launcher reaches it with the `ret_to_bios` pseudo-instruction — a **static**
  jump to that address — *not* a dynamic `ret` through `r1`: the x86 generator's
  dynamic-jump fast path assumes high (`>= ROM_ADDR`) targets, so a `ret` to the
  low BIOS address is unsupported; a static jump to a constant compiles to a direct
  `jmp` and works for any address. The assembler **derives** the BIOS address
  (`next_init_inst_addr + 0x14`, the `:0014` block of `add_entry_exit_jmp`; = `0x101c`
  in practice) rather than hard-coding it. Program-internal `ret`s are fine — they
  target high addresses.
- **`call`/`ret` follow the RISC-V convention** (return address in `r1`):
  `call LABEL` = `flag` op + `store_pc → r1` + `j(LABEL)`; `ret` =
  `and(0xfffffffffffffffe, r1)` + `setpc(0)`.
- **Pseudo-instructions** (`ziskasm.md` → "Pseudo-instructions"): `call`, `ret`,
  `jump(target)` (unconditional static jump to a label or absolute address), and
  `ret_to_bios`.

## Status

- Language spec (`ziskasm.md`): drafted, incl. sources/storage/jump/setpc/sp/end
  and the `call`/`ret` section.
- Assembler (`ziskasm` crate): **done** — parses and builds a `ZiskRom`.
- Pseudo-instructions: **done** — `call`, `ret`, `jump(target)`, `ret_to_bios`;
  plus an automatic launcher synthesized from `main:` / `_zisk_main:`.
- Emulator integration: **done** — `ziskemu -z <file|dir>`.
- `zisk2zisk` binary: **done** — `.zisk` → x86-64 NASM, mirroring `riscv2zisk`
  (reuses `ZiskRom2Asm::save_to_asm_file`; `--gen=0/1/2/7`).
- Verified: both `doubler` (explicit launcher) and `doubler-min` (auto launcher)
  run end-to-end and output `[2,4,…,16]` for the input `[1..8]`, on the interpreted
  and fast (x86) emulators.

## Roadmap / TODO

- Parser gap that currently errors: the `sp` modifier (the `step` a-source is
  supported).
- **Round-trip test** using the ROM→text decoder (`ZiskInst::to_zisk_asm`) as an
  oracle: assemble → decode → re-assemble.
- Multi-file: `include`/`import` (currently: pass a file list or a directory) and a
  **calling convention** (which registers a callee must preserve).
- Instruction-size convention: the spec still lists it as open; the assembler
  hard-codes `+4`. Pin it.
- **Proving** fidelity (beyond emulation).
- Deferred refactor: move `to_zisk_asm` (the decoder, currently in `zisk-core`)
  into `ziskasm`. Blocked earlier by a dependency cycle (`zisk-core` calls it from
  `zisk_rom_2_asm.rs`); the clean fix is to make `ziskasm` the low-level owner of
  the `SRC_/STORE_` encoding constants and have `zisk-core` depend on `ziskasm`.

## Notes

- `ziskasm` keeps a dev-dependency on `ziskemu` (for its E2E test) while `ziskemu`
  depends on `ziskasm` — a **dev-dependency cycle**, which Cargo permits.
