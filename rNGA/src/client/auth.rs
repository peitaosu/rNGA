//! Authentication state management.

/// Authentication information for NGA.
#[derive(Debug, Clone)]
pub struct AuthInfo {
    /// Access token.
    pub token: String,
    /// User ID.
    pub uid: String,
}

impl AuthInfo {
    /// Create new auth info.
    pub fn new(token: impl Into<String>, uid: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            uid: uid.into(),
        }
    }

    /// Check if auth looks valid.
    pub fn is_valid(&self) -> bool {
        !self.token.is_empty() && !self.uid.is_empty() && self.uid != "0"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_info_validity() {
        let valid = AuthInfo::new("token123", "12345");
        assert!(valid.is_valid());

        let empty_token = AuthInfo::new("", "12345");
        assert!(!empty_token.is_valid());

        let empty_uid = AuthInfo::new("token123", "");
        assert!(!empty_uid.is_valid());

        let zero_uid = AuthInfo::new("token123", "0");
        assert!(!zero_uid.is_valid());
    }
}
