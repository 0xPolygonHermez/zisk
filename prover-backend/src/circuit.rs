use std::borrow::Cow;
use std::fs;
use std::path::Path;

use anyhow::Result;

/// A Circom template body, embedded at compile time via [`load_circuit!`]
/// or loaded at runtime via [`CircomCircuit::from_path`].
///
/// This is the circuit-side sibling of [`crate::GuestProgram`]: a typed,
/// client-independent handle to user-supplied Circom source (e.g. the
/// recurser's `AggregatePublics` / `NormalizePublics` bodies).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircomCircuit {
    /// Display name for diagnostics — the load path or file stem.
    pub name: Cow<'static, str>,
    /// The Circom template source, injected verbatim into generated circuits.
    pub source: Cow<'static, str>,
}

impl CircomCircuit {
    /// Create from static strings (const-compatible; used by [`load_circuit!`]).
    pub const fn new_static(name: &'static str, source: &'static str) -> Self {
        Self { name: Cow::Borrowed(name), source: Cow::Borrowed(source) }
    }

    /// Read a circuit body from a file at runtime.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let source = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Error reading circom file {}: {}", path.display(), e))?;
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
        Ok(Self { name: Cow::Owned(name), source: Cow::Owned(source) })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Lets builder APIs take `impl Into<CircomCircuit>` so callers can pass a
/// `static` circuit by reference without an explicit `.clone()`.
impl From<&CircomCircuit> for CircomCircuit {
    fn from(c: &CircomCircuit) -> Self {
        c.clone()
    }
}

/// Embed a Circom template body at compile time.
///
/// The path is resolved relative to the file containing the invocation
/// (the same rule as [`include_str!`]).
///
/// # Example
/// ```ignore
/// use zisk_sdk::{load_circuit, CircomCircuit};
///
/// static AGGREGATE: CircomCircuit = load_circuit!("circuits/aggregate_publics.circom");
/// ```
///
/// For dynamic loading from a file path, use [`CircomCircuit::from_path`].
#[macro_export]
macro_rules! load_circuit {
    ($path:literal) => {
        $crate::CircomCircuit::new_static($path, include_str!($path))
    };
}
