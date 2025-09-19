use tonic::{Code, Status};

/// Coordinator-specific error types with proper security boundaries
#[derive(Debug, thiserror::Error)]
pub enum CoordinatorError {
    // // Client-safe errors - can be exposed to gRPC clients
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Invalid or inaccessible resource")]
    NotFoundOrInaccessible,

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Insufficient compute capacity available")]
    InsufficientCapacity,

    // Internal errors - logged but not exposed to clients
    #[error("Internal service error")]
    Internal(String),

    #[error("Prover error: {0}")]
    ProverError(String),
}

impl From<CoordinatorError> for Status {
    fn from(err: CoordinatorError) -> Self {
        tracing::error!("{:#}", err);

        match err {
            CoordinatorError::InvalidRequest(msg) => Status::new(Code::InvalidArgument, msg),
            CoordinatorError::NotFoundOrInaccessible => {
                Status::new(Code::Internal, "An internal error occurred")
            }
            CoordinatorError::InvalidArgument(msg) => Status::new(Code::InvalidArgument, msg),
            CoordinatorError::InsufficientCapacity => {
                Status::new(Code::ResourceExhausted, "Insufficient compute capacity")
            }
            // All internal errors return generic messages
            CoordinatorError::Internal(_) => {
                Status::new(Code::Internal, "An internal error occurred")
            }
            CoordinatorError::ProverError(msg) => {
                Status::new(Code::Internal, format!("Prover error: {msg}"))
            }
        }
    }
}

/// Type alias for Results using CoordinatorError
pub type CoordinatorResult<T> = Result<T, CoordinatorError>;
