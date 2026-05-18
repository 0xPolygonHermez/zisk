pub mod error;
pub mod manifest;
pub mod templates;

#[cfg(feature = "setup")]
pub mod setup;

#[cfg(feature = "prove")]
pub mod prove;

pub use error::{RecurserError, Result};
pub use manifest::{RecurserManifest, RecurserManifestInputs, TemplateHashes};
pub use templates::{gen_aggregator, CircomTemplates, StarkInputBlocks};
