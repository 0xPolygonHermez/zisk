# zisk-coordinator-client

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.
>
> For setup, deployment, and configuration of the distributed system, see the [distributed system README](../../README.md).

`zisk-coordinator-client` is a blocking client for submitting jobs to a running ZisK coordinator
and following them to completion.

## Overview

- **`CoordinatorClient`** — connect to a coordinator and submit setup/execute/prove jobs.
- **`Job` / `WatchHandle`** — a remote job handle for tracking status and awaiting results.
- **`InputSender`** — stream program input to the coordinator.

For a ready-made command-line front end, see `zisk-prove-client`.

## Documentation

- Coordinator docs: <https://0xpolygonhermez.github.io/zisk-docs/references/zisk-coordinator/>
- API reference: <https://docs.rs/zisk-coordinator-client>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
