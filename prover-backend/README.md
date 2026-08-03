# zisk-prover-backend

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`zisk-prover-backend` ties the execution engine and the STARK prover together behind a small
builder API, turning a program execution into a proof.

## Overview

Build a prover for a program and run it in one of several modes:

- **execute-only** — run the program without proving.
- **witness** — generate the witness only.
- **prove** — produce a proof.
- **verify-constraints** — check the constraints without producing a full proof.

Both emulator backends are supported via the builder: the Rust emulator (`.emu()`) and the
assembly emulator (`.asm()`).

```rust
use zisk_prover_backend::ProverClientBuilder;

// Build a prover for the Rust emulator in prove mode.
let prover = ProverClientBuilder::new().emu().prove().build()?;
```

For a higher-level entry point, see `zisk-sdk`.

## Documentation

- ZisK docs home: <https://0xpolygonhermez.github.io/zisk-docs/>
- API reference: <https://docs.rs/zisk-prover-backend>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
