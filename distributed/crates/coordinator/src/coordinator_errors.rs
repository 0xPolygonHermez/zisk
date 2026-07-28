use tonic::{Code, Status};

/// Coordinator-specific error types with proper security boundaries
#[derive(Debug, thiserror::Error)]
pub enum CoordinatorError {
    // // Client-safe errors - can be exposed to gRPC clients
    /// The request was malformed or invalid.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// The requested resource does not exist or is not accessible.
    #[error("Invalid or inaccessible resource")]
    NotFoundOrInaccessible,

    /// The named program has not been uploaded.
    #[error("Program not found: {0}. Did you call upload() before setup()?")]
    ProgramNotFound(String),

    /// An argument value was invalid.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Not enough compute capacity is available to start the job.
    #[error("Insufficient compute capacity available")]
    InsufficientCapacity,

    /// Workers are connected but still completing setup.
    #[error("Workers are connected but still running setup; retry shortly")]
    WorkersSettingUp,

    /// Workers are occupied with another job.
    #[error("Workers are busy running another job; retry shortly")]
    WorkersBusy,

    /// Workers are connected but setup has not been run yet.
    #[error("Workers are connected but setup has not been done; call setup() first")]
    WorkersNotSetup,

    // Internal errors - logged but not exposed to clients
    /// An unexpected internal error (logged, not exposed to clients).
    #[error("Internal service error: {0}")]
    Internal(String),

    /// An error reported by a worker.
    #[error("Worker error: {0}")]
    WorkerError(String),
}

impl From<CoordinatorError> for Status {
    fn from(err: CoordinatorError) -> Self {
        tracing::error!("{:#}", err);

        match err {
            CoordinatorError::InvalidRequest(msg) => Status::new(Code::InvalidArgument, msg),
            CoordinatorError::NotFoundOrInaccessible => {
                Status::new(Code::NotFound, "Resource not found or inaccessible")
            }
            CoordinatorError::ProgramNotFound(ref hash_id) => Status::new(
                Code::NotFound,
                format!("Program not found: {hash_id}. Did you call upload() before setup()?"),
            ),
            CoordinatorError::InvalidArgument(msg) => Status::new(Code::InvalidArgument, msg),
            CoordinatorError::InsufficientCapacity => {
                Status::new(Code::ResourceExhausted, "Insufficient compute capacity")
            }
            CoordinatorError::WorkersSettingUp => {
                Status::new(Code::Unavailable, "Workers are setting up; retry shortly")
            }
            CoordinatorError::WorkersBusy => {
                Status::new(Code::Unavailable, "Workers are busy; retry shortly")
            }
            CoordinatorError::WorkersNotSetup => Status::new(
                Code::FailedPrecondition,
                "Workers connected but setup not done; call setup() first",
            ),
            // All internal errors return generic messages
            CoordinatorError::Internal(_) => {
                Status::new(Code::Internal, "An internal error occurred")
            }
            CoordinatorError::WorkerError(msg) => {
                Status::new(Code::Internal, format!("Worker error: {msg}"))
            }
        }
    }
}

/// Type alias for Results using CoordinatorError
pub type CoordinatorResult<T> = Result<T, CoordinatorError>;
