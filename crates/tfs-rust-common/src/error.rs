#[derive(Debug, thiserror::Error)]
pub enum TfsRustError {
    #[error("config error: {0}")]
    Config(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("network error: {0}")]
    Network(#[from] std::io::Error),
    #[error("content error in {file}: {message}")]
    Content { file: String, message: String },
    #[error("lua error: {0}")]
    Lua(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("prop stream error: {0}")]
    PropStream(String),
}

pub type Result<T> = std::result::Result<T, TfsRustError>;
