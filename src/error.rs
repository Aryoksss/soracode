//! Error types shared across soracode.

use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error for soracode.
#[derive(Debug, Error)]
pub enum Error {
    /// Configuration could not be loaded or was invalid.
    #[error("config error: {0}")]
    Config(String),

    /// The upstream LLM provider returned an error.
    #[error("llm error: {0}")]
    Llm(String),

    /// A tool failed during execution.
    #[error("tool error: {0}")]
    Tool(String),

    /// Session persistence failed.
    #[error("session error: {0}")]
    Session(String),

    /// Underlying IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP transport failure.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// Database failure.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// Terminal failure.
    #[error("terminal error: {0}")]
    #[allow(dead_code)] // raised by future terminal-handling paths
    Terminal(String),

    /// Operation cancelled by the user.
    #[error("cancelled")]
    #[allow(dead_code)] // raised by future cancellation-aware paths
    Cancelled,
}
