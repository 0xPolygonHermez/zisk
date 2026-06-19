mod stdin;

pub use stdin::*;

// The stream IO layer (traits + transports + StreamSource + ZiskStreamWriter)
// lives in the `zisk-stream` crate so the guest-side `ziskos` crate can share
// the traits without depending on `zisk-common` (which would create a
// dependency cycle via `zisk-core` → `ziskos-hints`). Re-exported here so
// existing `zisk_common::io::*` consumers keep working unchanged. `zisk-common`
// enables the `quic` feature, so the QUIC transport is available here.
pub use zisk_stream::*;
