# zisk-sdk

> This crate is part of **[ZisK](https://github.com/0xPolygonHermez/zisk)**. See the [ZisK documentation](https://0xpolygonhermez.github.io/zisk-docs/) for an overview of the system.

`zisk-sdk` is the high-level client library for proving and verifying ZisK programs. It hides
the executor and prover behind a small API and supports both embedded and remote proving.

## Overview

- **`ProverClient`** — embedded proving that runs the prover in-process.
- **`RemoteClient`** — remote proving that submits jobs to a ZisK coordinator.
- **Requests & results** — `ExecuteRequest`, `ProveRequest`, `SetupRequest`, `VerifyBuilder`,
  and their result types.
- **Aggregation & recursion** — `Recurser` and `AggregateProofsRequest` for combining proofs.

If you want to prove and verify ZisK programs from your own Rust application, this is the crate
to depend on.

## Documentation

- SDK docs: <https://0xpolygonhermez.github.io/zisk-docs/references/zisk-sdk/>
- API reference: <https://docs.rs/zisk-sdk>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
