use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Engine error: {0}")]
    EngineError(String),

    #[error("API error: {0}")]
    ApiError(String),
}
