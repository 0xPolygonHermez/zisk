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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_wraps_into_backend_variant_and_is_transparent() {
        let io = std::io::Error::new(std::io::ErrorKind::Other, "disk on fire");
        let err = SdkError::backend(io);
        assert!(matches!(err, SdkError::Backend(_)));
        // `#[error(transparent)]` delegates Display straight to the wrapped error.
        assert_eq!(err.to_string(), "disk on fire");
    }

    #[test]
    fn backend_accepts_anyhow_without_the_sdk_naming_it_in_a_signature() {
        // `anyhow` is a dev-dependency; this mirrors the real boundary where
        // `.map_err(SdkError::backend)` converts an `anyhow::Error`.
        let err = SdkError::backend(anyhow::anyhow!("upstream {}", "boom"));
        assert!(matches!(err, SdkError::Backend(_)));
        assert_eq!(err.to_string(), "upstream boom");
    }

    #[test]
    fn io_error_routes_to_io_variant_not_backend() {
        // The `#[from] std::io::Error` must land in `Io`, not the catch-all.
        let err: SdkError = std::io::Error::new(std::io::ErrorKind::NotFound, "nope").into();
        assert!(matches!(err, SdkError::Io(_)), "io::Error should map to SdkError::Io");
        assert_eq!(err.to_string(), "nope");
    }

    #[tokio::test]
    async fn join_error_maps_to_backend() {
        let handle = tokio::spawn(async {
            std::future::pending::<()>().await;
        });
        handle.abort();
        let join_err = handle.await.expect_err("aborted task yields a JoinError");
        assert!(join_err.is_cancelled());
        let err: SdkError = join_err.into();
        assert!(matches!(err, SdkError::Backend(_)));
    }

    #[test]
    fn structured_variants_format_as_documented() {
        assert_eq!(SdkError::Cancelled.to_string(), "job was cancelled");
        assert_eq!(
            SdkError::Timeout(std::time::Duration::from_secs(3)).to_string(),
            "job timed out after 3s"
        );
        assert_eq!(SdkError::JobFailed("boom".into()).to_string(), "job failed: boom");
    }
}
