//! API error types with stable API error codes and gRPC status mapping.
//!
//! Every [`ApiError`] variant maps to a stable numeric code matching the
//! API reference in `book/developer/coordinator_api.md`. Clients that embed the
//! `ApiError` proto in status details can inspect these codes programmatically;
//! all other clients see the human-readable status message.
//!
//! Internal errors are logged at ERROR with a trace_id before the generic
//! "internal server error" message is returned — no server internals leak to
//! clients.

use tonic::{Code, Status};
use uuid::Uuid;

use crate::backend::DomainProofKind;

/// Stable numeric error codes (matches `book/developer/coordinator_api.md`).
pub mod codes {
    /// The requested job does not exist.
    pub const JOB_NOT_FOUND: u32 = 1001;
    /// The requested program is not registered.
    pub const PROGRAM_NOT_FOUND: u32 = 1002;
    /// The program is registered but its setup is not complete.
    pub const PROGRAM_NOT_SETUP: u32 = 1003;
    /// The operation is invalid for the job's current state.
    pub const INVALID_JOB_STATE: u32 = 1004;
    /// The requested proof-kind conversion is not allowed.
    pub const INVALID_PROOF_CONVERSION: u32 = 1005;
    /// The request provided a random seed and the coordinator does not accept client random seeds.
    pub const CLIENT_RANDOM_SEED_NOT_ALLOWED: u32 = 1006;
    /// The request carried job metadata and the coordinator does not accept it.
    pub const JOB_METADATA_NOT_ALLOWED: u32 = 1007;
    /// The pushed input is larger than the guest's input region holds.
    pub const INPUT_TOO_LARGE: u32 = 1009;
    /// No cluster is available to service the request.
    pub const CLUSTER_UNAVAILABLE: u32 = 2001;
    /// An unexpected internal error.
    pub const INTERNAL: u32 = 3001;
}

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

    /// The pushed input is larger than the guest's input region holds.
    #[error("Input too large: the guest's input region holds at most {limit} bytes")]
    InputTooLarge {
        /// The most bytes of input a job can take.
        limit: u64,
    },

    /// No cluster is available to service the request.
    #[error("Cluster unavailable: {reason}")]
    ClusterUnavailable {
        /// Why the cluster is unavailable.
        reason: &'static str,
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
            Self::InputTooLarge { .. } => codes::INPUT_TOO_LARGE,
            Self::ClusterUnavailable { .. } => codes::CLUSTER_UNAVAILABLE,
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
            Self::InputTooLarge { .. } => "INPUT_TOO_LARGE",
            Self::ClusterUnavailable { .. } => "CLUSTER_UNAVAILABLE",
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
            Self::ClusterUnavailable { .. } => Code::Unavailable,
            Self::Internal(_) => Code::Internal,
        }
    }
}

impl From<ApiError> for Status {
    fn from(err: ApiError) -> Self {
        let code = err.tonic_code();

        // For internal errors, log the real cause server-side and return a
        // generic message — never expose internal detail to callers.
        if let ApiError::Internal(ref detail) = err {
            tracing::error!(
                error_code = err.code(),
                error_name = err.name(),
                detail,
                "internal coordinator error"
            );
            return Status::new(code, "An internal error occurred");
        }

        Status::new(code, err.to_string())
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
        assert_eq!(ApiError::InputTooLarge { limit: 16 }.code(), 1009);
        assert_eq!(ApiError::ClusterUnavailable { reason: "test" }.code(), 2001);
        assert_eq!(ApiError::Internal("x".into()).code(), 3001);
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
