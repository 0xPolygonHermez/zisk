# zisk-common

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`zisk-common` collects the cross-cutting types and utilities shared across the ZisK workspace,
so the emulator, executor, state machines, and prover crates build on a single foundation
instead of duplicating them.

## Overview

- **Paths & I/O** (`paths`, `io`) — ZisK path resolution and input/output helpers.
- **Proof & profiling types** (`proof`, `proof_log`, `profiling`, `executor_stats`) — common
  proof metadata and execution statistics.
- **Execution Data bus & components** (`bus`, `component`) — the shared bus abstractions and component
  traits used to wire the state machines together.
- **Planning & counters** (`planner_helpers`, `regular_planner`, `regular_counters`) — helpers
  for planning trace segments and counting instruction multiplicities.
- **Misc primitives** — hints, hash modes, precompile helpers, and the shared `CommonError`.

## Documentation

- ZisK docs home: <https://0xpolygonhermez.github.io/zisk-docs/>
- API reference: <https://docs.rs/zisk-common>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
