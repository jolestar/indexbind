use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexbindError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("embedding error: {0}")]
    Embedding(#[from] anyhow::Error),
    #[error("invalid search config: {0}")]
    InvalidSearchConfig(String),
    #[error("artifact metadata missing: {0}")]
    MissingMetadata(&'static str),
}

pub type Result<T> = std::result::Result<T, IndexbindError>;
