# sm-rom

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`sm-rom` builds the ROM (program) portion of the ZisK witness.

## Overview

- **`RomSM`** — the `ComponentBuilder` that wires the parsed ROM and the per-instruction
  execution counters into a `RomInstance`.
- **`RomInstance`** — computes the ROM multiplicity witness, dispatching to the Rust- or
  ASM-emulator path depending on how `RomSM` was fed.
- **`CustomRom`** — builds the static ROM-ROM trace from ELF bytes (used at setup time, not
  during proving).
- **`RomError` / `RomResult`** — the crate-local error types.

`sm-rom` is one of the state machines orchestrated by the `executor`.

## Documentation

- ZisK docs home: <https://0xpolygonhermez.github.io/zisk-docs/>
- API reference: <https://docs.rs/sm-rom>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
