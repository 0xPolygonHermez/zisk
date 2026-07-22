# executor

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`executor` is the core execution engine of ZisK. Its main entry point, `ZiskExecutor`,
orchestrates the state machines that together produce the witness for proving.

## Overview

- Drives a program's execution and coordinates the per-component state machines (main, ROM,
  memory, arithmetic, binary, precompiles, …).
- Sits between the emulator (which runs the program) and the prover backend (which turns the
  witness into a proof).

The executor is normally used via `zisk-prover-backend` and the higher-level `zisk-sdk` rather
than directly.

## Documentation

- ZisK docs home: <https://0xpolygonhermez.github.io/zisk-docs/>
- API reference: <https://docs.rs/executor>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
