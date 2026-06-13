//! Error type for the ZisK common crate.
//!
//! [`CommonError`] is the typed error returned across `zisk-common`'s public
//! surface, replacing the previous `anyhow::Error`. Each variant mirrors the
//! message text the crate produced before, so error output is unchanged; the
//! dynamic detail (and any stringified cause) is carried in the payload.

/// Errors produced by the ZisK common crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommonError {
    /// A hint stream carried an unrecognized or malformed code.
    #[error("{0}")]
    InvalidHint(String),

    /// A buffer was too short or an index was out of bounds.
    #[error("Slice too short or index out of bounds")]
    OutOfBounds,

    /// A value could not be serialized.
    #[error("Serialization failed: {0}")]
    Serialization(String),

    /// A value could not be deserialized.
    #[error("Deserialization failed: {0}")]
    Deserialization(String),

    /// ABI decoding failed.
    #[error("ABI decoding failed: {0}")]
    AbiDecoding(String),

    /// Proof verification did not succeed.
    #[error("Zisk Proof was not verified")]
    NotVerified,

    /// A proof was malformed or of an unexpected kind.
    #[error("{0}")]
    InvalidProof(String),

    /// A filesystem or stdin I/O operation failed. The payload preserves the
    /// original, fully-formatted message (including any stringified cause).
    #[error("{0}")]
    Io(String),

    /// An unrecognized stdin URI scheme.
    #[error("Unknown stdin scheme: {0}")]
    UnknownScheme(String),

    /// A precondition or argument was invalid (e.g. an unsatisfiable memory
    /// reinterpretation). The payload preserves the original message.
    #[error("{0}")]
    Invalid(String),
}

/// Common-crate result type: [`Result`](core::result::Result) defaulting its
/// error half to [`CommonError`].
pub type Result<T, E = CommonError> = core::result::Result<T, E>;
