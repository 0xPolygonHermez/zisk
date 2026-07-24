# zisk-coordinator-api

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.
>
> For setup, deployment, and configuration of the distributed system, see the [distributed system README](../../README.md).

`zisk-coordinator-api` defines the wire contract for talking to the ZisK coordinator.

## Overview

- **`grpc`** — the tonic-generated gRPC client/server stubs and message types (from the
  `.proto` definitions).
- **`dto`** — domain-level data-transfer objects (requests, responses, job/proof kinds, input
  chunks) layered on top of the raw gRPC types.

Keeping the generated API in its own crate lets the server and every client share a single
definition of the protocol.

## Documentation

- Coordinator docs: <https://0xpolygonhermez.github.io/zisk-docs/references/zisk-coordinator/>
- API reference: <https://docs.rs/zisk-coordinator-api>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
