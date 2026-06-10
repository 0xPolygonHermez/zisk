use thiserror::Error;

pub type Result<T> = std::result::Result<T, RecurserError>;

#[derive(Debug, Error)]
pub enum RecurserError {
    #[error("Tera template error: {0}")]
    Template(#[from] tera::Error),

    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
}
