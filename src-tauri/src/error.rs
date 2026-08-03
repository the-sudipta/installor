use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstallorError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    #[error("operation failed: {0}")]
    Operation(String),
}

pub type InstallorResult<T> = Result<T, InstallorError>;
