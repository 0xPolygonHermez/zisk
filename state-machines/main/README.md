# sm-main

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`sm-main` implements the main state machine of the ZisK proving pipeline — the component that
handles the program's core execution trace.

## Overview

- **`MainPlanner`** — emits a `Plan` for each segment of the main trace.
- **`MainInstance`** — computes the witness for a single segment of the main trace.

The main trace is split into segments so it can be planned and witnessed in parallel. `sm-main`
is one of the state machines orchestrated by the `executor`.

## Documentation

- ZisK docs home: <https://0xpolygonhermez.github.io/zisk-docs/>
- API reference: <https://docs.rs/sm-main>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
