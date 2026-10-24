//! Error type for the ZisK SDK.

use std::time::Duration;

use zisk_coordinator_client::ApiError;

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

    /// A recurser request was misconfigured or produced an inconsistent
    /// result (bad guest set, duplicate or ill-formed program VK, coordinator
    /// `recurser_id` mismatch, unexpected verkey length).
    #[error("recurser: {0}")]
    Recurser(String),

    /// The coordinator refused the call. [`ApiError::code`] tells which error it was, and
    /// [`ApiError::is_retryable`] whether the same call can succeed later.
    #[error("coordinator refused the call: {0}")]
    Coordinator(ApiError),

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

    /// Wrap the error of a coordinator call: [`SdkError::Coordinator`] when the coordinator
    /// answered it, [`SdkError::Backend`] when the call failed before that.
    pub(crate) fn coordinator(err: impl Into<BoxError>) -> Self {
        let err = err.into();
        match ApiError::from_error(&*err) {
            Some(api) => Self::Coordinator(api),
            None => Self::Backend(err),
        }
    }

    /// Whether the same call can succeed later: the coordinator refused it as busy, setting up
    /// or without ready workers, or could not be reached.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Coordinator(api) if api.is_retryable())
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
        let io = std::io::Error::other("disk on fire");
        let err = SdkError::backend(io);
        assert!(matches!(err, SdkError::Backend(_)));
        // `#[error(transparent)]` delegates Display straight to the wrapped error.
        assert_eq!(err.to_string(), "disk on fire");
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
    fn a_refusal_from_the_coordinator_keeps_what_it_said() {
        // As a coordinator client call returns it: the status behind the call's context.
        let refused = Err::<(), _>(tonic::Status::unavailable("workers are busy"));
        let err = SdkError::coordinator(
            anyhow::Context::context(refused, "JobRequest RPC failed").unwrap_err(),
        );
        match &err {
            SdkError::Coordinator(api) => assert_eq!(api.message(), "workers are busy"),
            other => panic!("expected Coordinator, got {other:?}"),
        }
        assert!(err.is_retryable());

        let err = SdkError::coordinator(tonic::Status::failed_precondition("cluster too small"));
        assert!(matches!(err, SdkError::Coordinator(_)));
        assert!(!err.is_retryable());
    }

    #[test]
    fn a_call_that_failed_before_the_coordinator_answered_stays_backend() {
        let err = SdkError::coordinator(std::io::Error::other("no route to host"));
        assert!(matches!(err, SdkError::Backend(_)));
        assert!(!err.is_retryable());
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
