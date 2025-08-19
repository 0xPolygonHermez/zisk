# ELF Regressions

This directory contains a regression test suite for the zisk emulator and rom-setup. The tests ensure that the emulator correctly processes various types of ELF files and executes RISC-V assembly code under different conditions.

Each test directory contains assembly source files (`test.s`) and any necessary linker scripts.

## Usage

First, build the required binaries:

```bash
cargo build --bin cargo-zisk --bin ziskemu
```

Then navigate to the test directory and run the test suite:

```bash
cd elf-regressions
./scripts/build.sh        # Compile all test cases
./scripts/test.sh rom-setup all  # Run rom-setup on all ELF files
./scripts/test.sh emu all        # Run ziskemu on all ELF files
```

The build script compiles all assembly files into ELF binaries, while the test script validates them using both the rom-setup tool and the zisk emulator.

## Linker Scripts

Some test directories include custom linker scripts (`.ld` files) that control memory layout and linking behavior:

- **With linker script**: When a `.ld` file is present, it defines custom memory regions, entry points, and section layouts. This allows testing specific memory configurations and non-standard ELF structures.
- **Without linker script**: When no linker script is provided, the default linker behavior is used, see the shell script for what this is.