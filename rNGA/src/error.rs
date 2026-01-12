//! Error types.

use thiserror::Error;

/// The main error type for rNGA operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Network-related error
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// NGA API returned an error response.
    #[error("NGA API error [{code}]: {message}")]
    NGAApi { code: String, message: String },

    /// Failed to parse response data.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Operation requires authentication but none was provided.
    #[error("Authentication required")]
    AuthRequired,

    /// A required field was missing in the response.
    #[error("Missing field: {0}")]
    MissingField(String),

    /// Invalid argument passed to an API method.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Cache storage error.
    #[error("Cache error: {0}")]
    Cache(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing error.
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// XML parsing error.
    #[error("XML error: {0}")]
    Xml(String),

    /// XPath evaluation error.
    #[error("XPath error: {0}")]
    XPath(String),

    /// Feature not implemented.
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

impl Error {
    /// Create NGA API error.
    pub fn nga(code: impl Into<String>, message: impl Into<String>) -> Self {
        Error::NGAApi {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create a parse error.
    pub fn parse(msg: impl Into<String>) -> Self {
        Error::Parse(msg.into())
    }

    /// Create a missing field error.
    pub fn missing(field: impl Into<String>) -> Self {
        Error::MissingField(field.into())
    }

    /// Check if this error is potentially retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Network(_) => true,
            Error::NGAApi { code, .. } => code == "-4",
            _ => false,
        }
    }

    /// Check if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        match self {
            Error::AuthRequired => true,
            Error::NGAApi { code, .. } => code == "2",
            _ => false,
        }
    }
}

/// Result type alias for rNGA operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let e = Error::nga("1", "test message");
        assert_eq!(format!("{}", e), "NGA API error [1]: test message");
    }

    #[test]
    fn test_retryable() {
        assert!(Error::nga("-4", "blocked").is_retryable());
        assert!(!Error::nga("1", "not blocked").is_retryable());
    }
}
