# zisk-prove-client

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.
>
> For setup, deployment, and configuration of the distributed system, see the [distributed system README](../../README.md).

`zisk-prove-client` is a command-line client for submitting jobs to the ZisK coordinator. It is
a thin CLI on top of `zisk-coordinator-client`.

## Overview

- Register programs and submit setup, execute, and prove jobs from the terminal.
- Point it at a coordinator with `--coordinator` (or the `ZISK_COORDINATOR_URL` env var;
  defaults to `http://localhost:7000`).

```bash
zisk-prove-client --coordinator http://localhost:7000 register --elf path/to/program.elf
zisk-prove-client --help
```

## Documentation

- Coordinator docs: <https://0xpolygonhermez.github.io/zisk-docs/references/zisk-coordinator/>
- Command reference: `zisk-prove-client --help`

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../../../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
