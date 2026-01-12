//! Type-safe ID wrappers.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            /// Create a new ID from a string.
            pub fn new(id: impl Into<String>) -> Self {
                $name(id.into())
            }
            
            /// Check if this ID is empty or "0".
            pub fn is_empty(&self) -> bool {
                self.0.is_empty() || self.0 == "0"
            }
            
            /// Get the inner string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                $name(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                $name(s.to_owned())
            }
        }
        
        impl From<&String> for $name {
            fn from(s: &String) -> Self {
                $name(s.clone())
            }
        }
        
        impl From<i64> for $name {
            fn from(n: i64) -> Self {
                $name(n.to_string())
            }
        }
        
        impl From<i32> for $name {
            fn from(n: i32) -> Self {
                $name(n.to_string())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name("0".to_owned())
            }
        }
        
        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

define_id!(ForumId, "A forum identifier.");
define_id!(TopicId, "A topic/thread identifier.");
define_id!(PostId, "A post/reply identifier.");
define_id!(UserId, "A user identifier.");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let id = TopicId::new("12345");
        assert_eq!(id.as_str(), "12345");
        assert_eq!(format!("{}", id), "12345");
    }

    #[test]
    fn test_id_from_int() {
        let id = TopicId::from(12345i64);
        assert_eq!(id.as_str(), "12345");
    }

    #[test]
    fn test_id_is_empty() {
        assert!(TopicId::new("").is_empty());
        assert!(TopicId::new("0").is_empty());
        assert!(!TopicId::new("123").is_empty());
    }

    #[test]
    fn test_anonymous_user_id() {
        let user_id = UserId::from("-12345,context");
        assert_eq!(user_id.as_str(), "-12345,context");
        assert!(!user_id.is_empty());
    }
}
