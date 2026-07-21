# zisk-worker

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.
>
> For setup, deployment, and configuration of the distributed system, see the [distributed system README](../../README.md).

`zisk-worker` is a worker node for the ZisK distributed proving network. A worker connects to a
coordinator to receive and process proof-generation jobs.

## Overview

- **Configuration management** — worker settings and compute-capacity registration.
- **gRPC communication** — connects to the coordinator and streams results back.
- **Job handling** — witness generation, proving, and aggregation tasks.

Run one or more workers against a coordinator to scale proving horizontally.

## Documentation

- Worker docs: <https://0xpolygonhermez.github.io/zisk-docs/references/zisk-worker/>
- API reference: <https://docs.rs/zisk-worker>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
