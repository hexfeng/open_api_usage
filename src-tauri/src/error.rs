use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("credential storage error: {0}")]
    Credential(#[from] keyring::Error),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("invalid provider response: {0}")]
    InvalidResponse(String),
    #[error("unsupported provider: {0}")]
    UnsupportedProvider(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
