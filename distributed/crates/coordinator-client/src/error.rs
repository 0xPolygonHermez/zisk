use std::error::Error;
use std::fmt;

use tonic::{Code, Status};
use zisk_coordinator_api::errors::{CODE_KEY, NAME_KEY};

/// The coordinator's answer to a call it refused: the API error code and name it sends in the
/// response metadata, and the gRPC status.
///
/// [`code`](Self::code) tells the errors apart (the table is in
/// `book/developer/coordinator_api.md`); [`is_retryable`](Self::is_retryable) says whether the
/// same call can succeed later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    code: Option<u32>,
    name: Option<String>,
    grpc_code: Code,
    message: String,
}

impl ApiError {
    /// Reads the coordinator's answer from a gRPC status.
    pub fn from_status(status: &Status) -> Self {
        let metadata = status.metadata();
        let text = |key| metadata.get(key).and_then(|value| value.to_str().ok());
        Self {
            code: text(CODE_KEY).and_then(|code| code.parse().ok()),
            name: text(NAME_KEY).map(str::to_owned),
            grpc_code: status.code(),
            message: status.message().to_owned(),
        }
    }

    /// Reads the coordinator's answer from an error a [`CoordinatorClient`] call returned, or
    /// from any error whose sources lead to one. `None` when the call failed before the
    /// coordinator answered it.
    ///
    /// [`CoordinatorClient`]: crate::CoordinatorClient
    pub fn from_error(err: &(dyn Error + 'static)) -> Option<Self> {
        let mut cause = Some(err);
        while let Some(err) = cause {
            if let Some(status) = err.downcast_ref::<Status>() {
                return Some(Self::from_status(status));
            }
            cause = err.source();
        }
        None
    }

    /// The API error code (e.g. `2001`). `None` from a coordinator that predates the codes, for a
    /// request it could not parse, or when the coordinator could not be reached.
    pub fn code(&self) -> Option<u32> {
        self.code
    }

    /// The API error name (e.g. `CLUSTER_UNAVAILABLE`), when [`code`](Self::code) is set.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The gRPC status code.
    pub fn grpc_code(&self) -> Code {
        self.grpc_code
    }

    /// The coordinator's message, for people.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Whether the same call can succeed later: the coordinator is busy, setting up, has no
    /// ready workers yet, or could not be reached. Every other error comes back the same until
    /// the request or the cluster changes.
    ///
    /// A coordinator that predates the codes also answers a missing setup this way.
    pub fn is_retryable(&self) -> bool {
        is_retryable(self.grpc_code)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(name) = &self.name {
            write!(f, " ({name})")?;
        }
        Ok(())
    }
}

impl Error for ApiError {}

/// Whether a call refused with `code` can succeed later.
pub(crate) fn is_retryable(code: Code) -> bool {
    code == Code::Unavailable
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use tonic::metadata::{MetadataMap, MetadataValue};

    use super::*;

    fn refused(code: Code, api_code: u32, name: &'static str) -> Status {
        let mut metadata = MetadataMap::new();
        metadata.insert(CODE_KEY, MetadataValue::from(api_code));
        metadata.insert(NAME_KEY, MetadataValue::from_static(name));
        Status::with_metadata(code, "Cluster unavailable: workers are busy", metadata)
    }

    #[test]
    fn reads_the_code_and_name_the_coordinator_sent() {
        let api = ApiError::from_status(&refused(Code::Unavailable, 2001, "CLUSTER_UNAVAILABLE"));
        assert_eq!(api.code(), Some(2001));
        assert_eq!(api.name(), Some("CLUSTER_UNAVAILABLE"));
        assert!(api.is_retryable());
        assert_eq!(api.to_string(), "Cluster unavailable: workers are busy (CLUSTER_UNAVAILABLE)");
    }

    #[test]
    fn only_unavailable_is_retryable() {
        let api =
            ApiError::from_status(&refused(Code::FailedPrecondition, 2002, "CLUSTER_TOO_SMALL"));
        assert!(!api.is_retryable());
    }

    #[test]
    fn a_status_without_metadata_has_no_code() {
        let api = ApiError::from_status(&Status::unavailable("transport error"));
        assert_eq!(api.code(), None);
        assert_eq!(api.name(), None);
        assert!(api.is_retryable());
    }

    #[test]
    fn found_behind_the_context_a_client_call_adds() {
        let err = Err::<(), _>(refused(Code::Unavailable, 2001, "CLUSTER_UNAVAILABLE"))
            .context("JobRequest RPC failed")
            .unwrap_err();
        assert_eq!(ApiError::from_error(err.as_ref()).and_then(|api| api.code()), Some(2001));

        let local = anyhow::anyhow!("invalid coordinator URL");
        assert_eq!(ApiError::from_error(local.as_ref()), None);
    }
}
