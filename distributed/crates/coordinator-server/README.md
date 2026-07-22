# zisk-coordinator-server

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.
>
> For setup, deployment, and configuration of the distributed system, see the [distributed system README](../../README.md).

`zisk-coordinator-server` is the public API façade for the ZisK proving system: it exposes the
`ZiskCoordinatorApi` gRPC service and delegates business logic to a pluggable backend.

## Overview

- **gRPC service** — implements the coordinator API defined in `book/developer/coordinator_api.md`.
- **`MockBackend`** — in-memory, no coordinator required; useful for testing.
- **`CoordinatorBackend`** — runs the coordinator (`zisk-coordinator`) in-process.

Clients such as `zisk-coordinator-client` and `zisk-prove-client` talk to the service this
crate serves.

## Documentation

- Coordinator docs: <https://0xpolygonhermez.github.io/zisk-docs/references/zisk-coordinator/>
- API reference: <https://docs.rs/zisk-coordinator-server>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
