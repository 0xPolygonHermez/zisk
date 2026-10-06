# asm-runner

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`asm-runner` provides the core logic for managing the ZisK assembly-emulator process — the
fast, C++-backed emulator.

## Overview

- **Process management** — launch and supervise the ASM emulator process.
- **Shared memory** — manage the shared-memory region between Rust and the C++ side.
- **Synchronization** — the primitives that coordinate the two processes.
- **Communication** — the protocol used to feed inputs and collect execution output.

It is the Rust bridge that lets the rest of ZisK run a program on the assembly emulator and read
back the resulting trace.

## Documentation

- ZisK docs home: <https://0xpolygonhermez.github.io/zisk-docs/>
- API reference: <https://docs.rs/asm-runner>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
