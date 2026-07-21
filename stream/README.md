# zisk-stream

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`zisk-stream` is ZisK's stream I/O layer: the read/write traits and every transport that
implements them.

## Overview

- **Traits** — `StreamRead` / `StreamWrite`, plus the `StreamSource` reader multiplexer and the
  `ZiskStreamWriter` producer.
- **`std`-only transports** — file, Unix socket, in-memory, and channel. No async/networking
  dependencies, so guest-side consumers (e.g. `ziskos`) can depend on this crate without pulling
  in the prover stack.
- **QUIC transport** — behind the `quic` feature (`quinn` + `tokio` + `rustls`).

### Features

- **default** — traits + `std`-only transports + `StreamSource` / `ZiskStreamWriter`.
- **`quic`** — adds the QUIC transport. Enabled by `zisk-common`; left off by `ziskos`.

## Documentation

- ZisK docs home: <https://0xpolygonhermez.github.io/zisk-docs/>
- API reference: <https://docs.rs/zisk-stream>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
