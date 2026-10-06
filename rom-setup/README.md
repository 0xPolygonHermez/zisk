# rom-setup

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`rom-setup` produces the artifacts the ZisK prover needs from a guest ELF.

## Overview

- **ROM Merkle setup** (`rom_merkle_setup`, `rom_merkle_setup_verkey`) — builds
  the ROM commitment and the program verification key.
- **ASM generation** (`generate_assembly`) — compiles the ELF into the assembly
  binaries used by the ASM execution backend.
- **Path/hash helpers** — content-address ELFs and resolve the cache paths where
  ROM and ASM artifacts are stored.

Artifacts are content-addressed by the ELF hash and the proving key's
`HashMode`, so a given ELF is processed once and reused across runs.

## Documentation

- ZisK docs home: <https://0xpolygonhermez.github.io/zisk-docs/>
- API reference: <https://docs.rs/rom-setup>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
