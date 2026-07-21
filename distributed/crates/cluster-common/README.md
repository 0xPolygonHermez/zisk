# zisk-cluster-common

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.
>
> For setup, deployment, and configuration of the distributed system, see the [distributed system README](../../README.md).

`zisk-cluster-common` holds the types and utilities shared between the coordinator, workers,
and clients of the ZisK distributed proving system.

## Overview

- **`dto`** — data-transfer objects exchanged across the cluster.
- **`types`** — common domain types.
- **`tracing`** — shared tracing/logging setup.

It is an internal building block for the other `distributed/` crates rather than a user-facing
library.

## Documentation

- ZisK docs home: <https://0xpolygonhermez.github.io/zisk-docs/>
- API reference: <https://docs.rs/zisk-cluster-common>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
