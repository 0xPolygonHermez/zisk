# cargo-zisk

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`cargo-zisk` is the command-line interface for ZisK. It provides the `cargo-zisk` and
`cargo-zisk-dev` binaries — the main entry points for building, running, proving, and verifying
ZisK programs.

## Overview

- **`cargo-zisk`** — build a Rust program into a ZisK ELF, run it on the emulator, generate the
  ROM setup and proving keys, prove an execution, and verify the resulting proof.
- **`cargo-zisk-dev`** — additional developer subcommands.

Invoke it as a `cargo` subcommand:

```bash
cargo-zisk --help
```

## Documentation

- Cargo-zisk docs: <https://0xpolygonhermez.github.io/zisk-docs/references/cargo-zisk/>
- Command reference: `cargo-zisk --help`

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
