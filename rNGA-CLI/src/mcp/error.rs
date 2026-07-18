//! MCP error mapping.

use rnga::Error;
use rmcp::ErrorData as McpError;

const AUTH_REQUIRED_PREFIX: &str = "AUTH_REQUIRED:";

pub fn map_error(error: Error) -> McpError {
    if error.is_auth_error() || matches!(error, Error::AuthRequired) {
        return McpError::internal_error(
            format!("{AUTH_REQUIRED_PREFIX} {error}"),
            None,
        );
    }

    match error {
        Error::InvalidArgument(message) => McpError::invalid_params(message, None),
        Error::MissingField(field) => McpError::invalid_params(format!("Missing field: {field}"), None),
        Error::NotImplemented(message) => McpError::internal_error(message, None),
        Error::NGAApi { code, message } => {
            McpError::internal_error(format!("NGA API error [{code}]: {message}"), None)
        }
        other => McpError::internal_error(other.to_string(), None),
    }
}

pub fn map_anyhow(error: anyhow::Error) -> McpError {
    if let Some(rnga_error) = error.downcast_ref::<Error>() {
        return map_error(match rnga_error {
            Error::AuthRequired => Error::AuthRequired,
            Error::InvalidArgument(message) => Error::InvalidArgument(message.clone()),
            Error::MissingField(field) => Error::MissingField(field.clone()),
            Error::NotImplemented(message) => Error::NotImplemented(message.clone()),
            Error::NGAApi { code, message } => Error::nga(code.clone(), message.clone()),
            other => return McpError::internal_error(other.to_string(), None),
        });
    }

    McpError::internal_error(error.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_auth_error() {
        let mapped = map_error(Error::AuthRequired);
        assert_eq!(mapped.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
        assert!(mapped.message.contains(AUTH_REQUIRED_PREFIX));
    }

    #[test]
    fn test_map_invalid_argument() {
        let mapped = map_error(Error::InvalidArgument("bad input".into()));
        assert_eq!(mapped.code, rmcp::model::ErrorCode::INVALID_PARAMS);
    }
}
