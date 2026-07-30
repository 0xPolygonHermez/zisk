# zisk-coordinator

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.
>
> For setup, deployment, and configuration of the distributed system, see the [distributed system README](../../README.md).

`zisk-coordinator` implements the in-process coordinator: it accepts proof jobs, distributes
work to connected workers, and drives jobs to completion.

## Overview

- **Worker pool & scheduling** (`workers_pool`) — tracks connected workers and assigns tasks.
- **Job lifecycle** (`job_events`) — job state transitions and event emission.
- **gRPC glue & hint relaying** (`coordinator_grpc`, `hints_relay`).
- **Operations** — configuration, metrics, hooks, and graceful shutdown.

It is typically embedded by `zisk-coordinator-server` rather than used directly.

## Documentation

- Coordinator docs: <https://0xpolygonhermez.github.io/zisk-docs/references/zisk-coordinator/>
- API reference: <https://docs.rs/zisk-coordinator>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
