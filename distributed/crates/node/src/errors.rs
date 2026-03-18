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

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

impl From<NodeError> for tonic::Status {
    fn from(e: NodeError) -> Self {
        match e {
            NodeError::ClusterNotFound(_) => tonic::Status::not_found(e.to_string()),
            NodeError::NotImplemented(_) => tonic::Status::unimplemented(e.to_string()),
            NodeError::Validation(_) => tonic::Status::invalid_argument(e.to_string()),
            NodeError::Grpc(s) => s,
            _ => tonic::Status::internal(e.to_string()),
        }
    }
}
