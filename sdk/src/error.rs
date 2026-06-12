//! Error type for the ZisK SDK.

use std::time::Duration;

/// Boxed, thread-safe source error preserved by [`SdkError::Backend`].
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Errors returned by the ZisK SDK.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SdkError {
    /// The request was configured in a way the SDK cannot honor.
    #[error("{0}")]
    InvalidConfig(String),

    /// The requested operation is not supported by the configured executor.
    #[error("{0}")]
    UnsupportedExecutor(String),

    /// A proof or payload could not be (de)serialized.
    #[error("serialization failed: {0}")]
    Serialization(String),

    /// The job did not complete within the configured timeout.
    #[error("job timed out after {0:?}")]
    Timeout(Duration),

    /// The job was cancelled before it completed.
    #[error("job was cancelled")]
    Cancelled,

    /// The job reached a terminal failure state on the coordinator.
    #[error("job failed: {0}")]
    JobFailed(String),

    /// The coordinator returned a result whose kind did not match the request.
    #[error("unexpected coordinator response: {0}")]
    UnexpectedResponse(String),

    /// A filesystem or other standard I/O operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An error surfaced by an underlying ZisK component (prover backend,
    /// coordinator client, ROM setup, stream transport, …). The original
    /// error is preserved as the [`source`](std::error::Error::source).
    #[error(transparent)]
    Backend(BoxError),
}

impl SdkError {
    /// Wrap a foreign error into [`SdkError::Backend`].
    pub(crate) fn backend(err: impl Into<BoxError>) -> Self {
        Self::Backend(err.into())
    }
}

impl From<tokio::task::JoinError> for SdkError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Backend(Box::new(err))
    }
}

/// SDK result type: [`Result`](core::result::Result) defaulting its error half to [`SdkError`].
pub type Result<T, E = SdkError> = core::result::Result<T, E>;
