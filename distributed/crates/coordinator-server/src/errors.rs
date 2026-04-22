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
    pub const JOB_NOT_FOUND: u32 = 1001;
    pub const PROGRAM_NOT_FOUND: u32 = 1002;
    pub const PROGRAM_NOT_SETUP: u32 = 1003;
    pub const INVALID_JOB_STATE: u32 = 1004;
    pub const INVALID_PROOF_CONVERSION: u32 = 1005;
    pub const CLUSTER_UNAVAILABLE: u32 = 2001;
    pub const INTERNAL: u32 = 3001;
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Job not found: {0}")]
    JobNotFound(Uuid),

    #[error("Program not found: {0}")]
    ProgramNotFound(String),

    #[error("Program exists but setup is not complete: {0}")]
    ProgramNotSetup(String),

    #[error("Invalid job state: {reason}")]
    InvalidJobState { reason: String },

    #[error("Invalid proof conversion: {from:?} → {to:?}")]
    InvalidProofConversion { from: DomainProofKind, to: DomainProofKind },

    #[error("Cluster unavailable: {reason}")]
    ClusterUnavailable { reason: &'static str },

    #[error("Internal error")]
    Internal(String),
}

impl ApiError {
    pub fn code(&self) -> u32 {
        match self {
            Self::JobNotFound(_) => codes::JOB_NOT_FOUND,
            Self::ProgramNotFound(_) => codes::PROGRAM_NOT_FOUND,
            Self::ProgramNotSetup(_) => codes::PROGRAM_NOT_SETUP,
            Self::InvalidJobState { .. } => codes::INVALID_JOB_STATE,
            Self::InvalidProofConversion { .. } => codes::INVALID_PROOF_CONVERSION,
            Self::ClusterUnavailable { .. } => codes::CLUSTER_UNAVAILABLE,
            Self::Internal(_) => codes::INTERNAL,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::JobNotFound(_) => "JOB_NOT_FOUND",
            Self::ProgramNotFound(_) => "PROGRAM_NOT_FOUND",
            Self::ProgramNotSetup(_) => "PROGRAM_NOT_SETUP",
            Self::InvalidJobState { .. } => "INVALID_JOB_STATE",
            Self::InvalidProofConversion { .. } => "INVALID_PROOF_CONVERSION",
            Self::ClusterUnavailable { .. } => "CLUSTER_UNAVAILABLE",
            Self::Internal(_) => "INTERNAL",
        }
    }

    fn tonic_code(&self) -> Code {
        match self {
            Self::JobNotFound(_) | Self::ProgramNotFound(_) | Self::ProgramNotSetup(_) => {
                Code::NotFound
            }
            Self::InvalidJobState { .. } | Self::InvalidProofConversion { .. } => {
                Code::InvalidArgument
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
                "internal coordinator error {}", detail
            );
            return Status::new(code, "An internal error occurred");
        }

        Status::new(code, err.to_string())
    }
}

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
