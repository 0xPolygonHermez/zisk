use thiserror::Error;

pub type NodeResult<T> = std::result::Result<T, NodeError>;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("Configuration error: {0}")]
    Config(#[from] anyhow::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cluster not found: {0}")]
    ClusterNotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("No coordinator configured")]
    NoCoordinator,

    #[error("Coordinator error ({code}): {message}")]
    CoordinatorError { code: String, message: String },

    #[error("Coordinator returned an invalid response: {0}")]
    InvalidCoordinatorResponse(String),

    /// Propagated transport-level error from a coordinator RPC call.
    #[error("Coordinator RPC error: {0}")]
    CoordinatorRpc(#[from] tonic::Status),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

impl From<zisk_distributed_grpc_api::ErrorResponse> for NodeError {
    fn from(e: zisk_distributed_grpc_api::ErrorResponse) -> Self {
        NodeError::CoordinatorError { code: e.code, message: e.message }
    }
}

impl From<NodeError> for tonic::Status {
    fn from(e: NodeError) -> Self {
        match e {
            NodeError::ClusterNotFound(_) => tonic::Status::not_found(e.to_string()),
            NodeError::NotImplemented(_) => tonic::Status::unimplemented(e.to_string()),
            NodeError::Validation(_) => tonic::Status::invalid_argument(e.to_string()),
            NodeError::NotFound(_) => tonic::Status::not_found(e.to_string()),
            NodeError::NoCoordinator => tonic::Status::unavailable(e.to_string()),
            NodeError::CoordinatorError { ref code, .. } => match code.as_str() {
                "NOT_FOUND" => tonic::Status::not_found(e.to_string()),
                "INVALID_ARGUMENT" => tonic::Status::invalid_argument(e.to_string()),
                _ => tonic::Status::internal(e.to_string()),
            },
            NodeError::CoordinatorRpc(s) => s,
            NodeError::InvalidCoordinatorResponse(_) => tonic::Status::internal(e.to_string()),
            // Config, Io, Yaml fall through here
            _ => tonic::Status::internal(e.to_string()),
        }
    }
}
