//! API error types with stable API error codes and gRPC status mapping.
//!
//! Every [`ApiError`] variant maps to a stable numeric code matching the
//! API reference in `book/developer/coordinator_api.md`. Its status carries the
//! code and name in the response metadata (see [`zisk_coordinator_api::errors`]),
//! so clients act on the code; people read the status message.
//!
//! Internal errors are logged at ERROR with a trace_id before the generic
//! "internal server error" message is returned — no server internals leak to
//! clients.

use tonic::metadata::{MetadataMap, MetadataValue};
use tonic::{Code, Status};
use uuid::Uuid;
use zisk_coordinator_api::errors::{CODE_KEY, NAME_KEY};

use crate::backend::DomainProofKind;

/// Stable numeric error codes (matches `book/developer/coordinator_api.md`).
pub use zisk_coordinator_api::errors::codes;

/// An API-level error, each variant carrying a stable [`code`](ApiError::code).
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// The requested job does not exist.
    #[error("Job not found: {0}")]
    JobNotFound(Uuid),

    /// The requested program is not registered.
    #[error("Program not found: {0}")]
    ProgramNotFound(String),

    /// The program is registered but setup has not completed.
    #[error("Program exists but setup is not complete: {0}")]
    ProgramNotSetup(String),

    /// The operation is invalid for the job's current state.
    #[error("Invalid job state: {reason}")]
    InvalidJobState {
        /// Why the state is invalid for the operation.
        reason: String,
    },

    /// The requested proof-kind conversion is not allowed.
    #[error("Invalid proof conversion: {from:?} → {to:?}")]
    InvalidProofConversion {
        /// Source proof kind.
        from: DomainProofKind,
        /// Target proof kind.
        to: DomainProofKind,
    },

    /// The request provided a random seed and the coordinator does not accept client random seeds.
    #[error("Client-supplied random seeds are not allowed by this coordinator")]
    ClientRandomSeedNotAllowed,

    /// The request carried job metadata and the coordinator does not accept it.
    #[error("Job metadata is not allowed by this coordinator")]
    JobMetadataNotAllowed,

    /// The request asks for a final SNARK the workers cannot prove: they have no SNARK proving
    /// key, or one of another protocol.
    #[error("Final SNARK unavailable: {0}")]
    SnarkUnavailable(String),

    /// The pushed input is larger than the guest's input region holds.
    #[error("Input too large: the guest's input region holds at most {limit} bytes")]
    InputTooLarge {
        /// The most bytes of input a job can take.
        limit: u64,
    },

    /// The cluster cannot take the job now (busy, setting up, or no ready workers); a retry can
    /// succeed.
    #[error("Cluster unavailable: {reason}")]
    ClusterUnavailable {
        /// Why the cluster is unavailable.
        reason: &'static str,
    },

    /// The cluster has less capacity than the job's minimum, and no worker that is busy,
    /// recovering or waiting for setup would make up the difference: a retry fails the same way.
    #[error(
        "Cluster too small: {available} ready compute units, the job needs at least {minimum}"
    )]
    ClusterTooSmall {
        /// Compute units of the cluster's set-up workers.
        available: u32,
        /// The job's minimum compute units.
        minimum: u32,
    },

    /// An unexpected internal error (detail is logged, not returned to clients).
    #[error("Internal error")]
    Internal(String),
}

impl ApiError {
    /// The stable numeric error code for this error (see [`codes`]).
    pub fn code(&self) -> u32 {
        match self {
            Self::JobNotFound(_) => codes::JOB_NOT_FOUND,
            Self::ProgramNotFound(_) => codes::PROGRAM_NOT_FOUND,
            Self::ProgramNotSetup(_) => codes::PROGRAM_NOT_SETUP,
            Self::InvalidJobState { .. } => codes::INVALID_JOB_STATE,
            Self::InvalidProofConversion { .. } => codes::INVALID_PROOF_CONVERSION,
            Self::ClientRandomSeedNotAllowed => codes::CLIENT_RANDOM_SEED_NOT_ALLOWED,
            Self::JobMetadataNotAllowed => codes::JOB_METADATA_NOT_ALLOWED,
            Self::SnarkUnavailable(_) => codes::SNARK_UNAVAILABLE,
            Self::InputTooLarge { .. } => codes::INPUT_TOO_LARGE,
            Self::ClusterUnavailable { .. } => codes::CLUSTER_UNAVAILABLE,
            Self::ClusterTooSmall { .. } => codes::CLUSTER_TOO_SMALL,
            Self::Internal(_) => codes::INTERNAL,
        }
    }

    /// The stable string name for this error (e.g. `"JOB_NOT_FOUND"`).
    pub fn name(&self) -> &'static str {
        match self {
            Self::JobNotFound(_) => "JOB_NOT_FOUND",
            Self::ProgramNotFound(_) => "PROGRAM_NOT_FOUND",
            Self::ProgramNotSetup(_) => "PROGRAM_NOT_SETUP",
            Self::InvalidJobState { .. } => "INVALID_JOB_STATE",
            Self::InvalidProofConversion { .. } => "INVALID_PROOF_CONVERSION",
            Self::ClientRandomSeedNotAllowed => "CLIENT_RANDOM_SEED_NOT_ALLOWED",
            Self::JobMetadataNotAllowed => "JOB_METADATA_NOT_ALLOWED",
            Self::SnarkUnavailable(_) => "SNARK_UNAVAILABLE",
            Self::InputTooLarge { .. } => "INPUT_TOO_LARGE",
            Self::ClusterUnavailable { .. } => "CLUSTER_UNAVAILABLE",
            Self::ClusterTooSmall { .. } => "CLUSTER_TOO_SMALL",
            Self::Internal(_) => "INTERNAL",
        }
    }

    fn tonic_code(&self) -> Code {
        match self {
            Self::JobNotFound(_) | Self::ProgramNotFound(_) | Self::ProgramNotSetup(_) => {
                Code::NotFound
            }
            Self::InvalidJobState { .. }
            | Self::InvalidProofConversion { .. }
            | Self::InputTooLarge { .. } => Code::InvalidArgument,
            Self::ClientRandomSeedNotAllowed | Self::JobMetadataNotAllowed => {
                Code::PermissionDenied
            }
            Self::SnarkUnavailable(_) | Self::ClusterTooSmall { .. } => Code::FailedPrecondition,
            Self::ClusterUnavailable { .. } => Code::Unavailable,
            Self::Internal(_) => Code::Internal,
        }
    }

    /// The response metadata naming this error, so a client reads its code instead of the
    /// message.
    fn metadata(&self) -> MetadataMap {
        let mut metadata = MetadataMap::new();
        metadata.insert(CODE_KEY, MetadataValue::from(self.code()));
        metadata.insert(NAME_KEY, MetadataValue::from_static(self.name()));
        metadata
    }
}

impl From<ApiError> for Status {
    fn from(err: ApiError) -> Self {
        let code = err.tonic_code();
        let metadata = err.metadata();

        // For internal errors, log the real cause server-side and return a
        // generic message — never expose internal detail to callers.
        if let ApiError::Internal(ref detail) = err {
            tracing::error!(
                error_code = err.code(),
                error_name = err.name(),
                detail,
                "internal coordinator error"
            );
            return Status::with_metadata(code, "An internal error occurred", metadata);
        }

        Status::with_metadata(code, err.to_string(), metadata)
    }
}

/// Result alias for API operations that fail with [`ApiError`].
pub type ApiResult<T> = Result<T, ApiError>;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Convenience: wrap any internal anyhow error without exposing detail.
pub fn internal(msg: impl std::fmt::Display) -> ApiError {
    ApiError::Internal(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Code;

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(ApiError::JobNotFound(Uuid::new_v4()).code(), 1001);
        assert_eq!(ApiError::ProgramNotFound("x".into()).code(), 1002);
        assert_eq!(ApiError::ProgramNotSetup("x".into()).code(), 1003);
        assert_eq!(ApiError::InvalidJobState { reason: "x".into() }.code(), 1004);
        assert_eq!(ApiError::ClientRandomSeedNotAllowed.code(), 1006);
        assert_eq!(ApiError::JobMetadataNotAllowed.code(), 1007);
        assert_eq!(ApiError::SnarkUnavailable("x".into()).code(), 1008);
        assert_eq!(ApiError::InputTooLarge { limit: 16 }.code(), 1009);
        assert_eq!(ApiError::ClusterUnavailable { reason: "test" }.code(), 2001);
        assert_eq!(ApiError::ClusterTooSmall { available: 4, minimum: 8 }.code(), 2002);
        assert_eq!(ApiError::Internal("x".into()).code(), 3001);
    }

    #[test]
    fn status_carries_the_error_code_and_name_in_its_metadata() {
        let status = Status::from(ApiError::ClusterUnavailable { reason: "test" });
        assert_eq!(status.metadata().get(CODE_KEY).unwrap(), "2001");
        assert_eq!(status.metadata().get(NAME_KEY).unwrap(), "CLUSTER_UNAVAILABLE");

        // The generic message of an internal error still names it.
        let status = Status::from(ApiError::Internal("secret detail".into()));
        assert_eq!(status.metadata().get(CODE_KEY).unwrap(), "3001");
        assert_eq!(status.metadata().get(NAME_KEY).unwrap(), "INTERNAL");
    }

    #[test]
    fn cluster_too_small_maps_to_failed_precondition_with_its_capacity() {
        let status = Status::from(ApiError::ClusterTooSmall { available: 4, minimum: 8 });
        assert_eq!(status.code(), Code::FailedPrecondition);
        assert!(status.message().contains("4 ready compute units"), "{}", status.message());
        assert!(status.message().contains("at least 8"), "{}", status.message());
    }

    #[test]
    fn internal_error_returns_generic_status_message() {
        let status = Status::from(ApiError::Internal("secret detail".into()));
        assert_eq!(status.code(), Code::Internal);
        assert!(!status.message().contains("secret detail"));
    }

    #[test]
    fn client_random_seed_not_allowed_maps_to_permission_denied() {
        assert_eq!(
            Status::from(ApiError::ClientRandomSeedNotAllowed).code(),
            Code::PermissionDenied
        );
    }

    #[test]
    fn input_too_large_maps_to_invalid_argument_with_its_limit() {
        let status = Status::from(ApiError::InputTooLarge { limit: 2147483632 });
        assert_eq!(status.code(), Code::InvalidArgument);
        assert!(status.message().contains("2147483632"), "{}", status.message());
    }

    #[test]
    fn job_metadata_not_allowed_maps_to_permission_denied() {
        assert_eq!(Status::from(ApiError::JobMetadataNotAllowed).code(), Code::PermissionDenied);
    }

    #[test]
    fn snark_unavailable_maps_to_failed_precondition_with_its_reason() {
        let status = Status::from(ApiError::SnarkUnavailable("no SNARK key".into()));
        assert_eq!(status.code(), Code::FailedPrecondition);
        assert!(status.message().contains("no SNARK key"));
    }

    #[test]
    fn cluster_unavailable_maps_to_unavailable_code() {
        let status = Status::from(ApiError::ClusterUnavailable { reason: "test" });
        assert_eq!(status.code(), Code::Unavailable);
    }

    #[test]
    fn not_found_errors_map_to_not_found_code() {
        assert_eq!(Status::from(ApiError::ProgramNotFound("h".into())).code(), Code::NotFound);
        assert_eq!(Status::from(ApiError::JobNotFound(Uuid::new_v4())).code(), Code::NotFound);
    }
}
