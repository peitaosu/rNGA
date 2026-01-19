//! User models.

use serde::{Deserialize, Serialize};

use super::UserId;

/// A user on NGA.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct User {
    /// User ID.
    pub id: UserId,
    /// Display name.
    pub name: UserName,
    /// Avatar URL.
    pub avatar_url: Option<String>,
    /// User reputation/fame.
    pub reputation: i32,
    /// User posts count.
    pub posts: i32,
    /// Registration date.
    pub reg_date: i64,
    /// User signature.
    pub signature: Option<String>,
    /// Whether user is admin.
    pub is_admin: bool,
    /// Whether user is moderator.
    pub is_mod: bool,
    /// Whether user is muted.
    pub is_muted: bool,
    /// User honor/medal.
    pub honor: Option<String>,
}

impl User {
    /// Create an anonymous user with the given ID.
    pub fn anonymous(id: impl Into<UserId>) -> Self {
        Self {
            id: id.into(),
            name: UserName::Anonymous,
            ..Default::default()
        }
    }

    /// Check if this is an anonymous user.
    pub fn is_anonymous(&self) -> bool {
        matches!(self.name, UserName::Anonymous)
    }

    /// Check if this user ID is negative.
    pub fn is_negative_id(&self) -> bool {
        self.id.0.starts_with('-')
    }

    /// Get the anonymous identifier if this is an anonymous post.
    pub fn anon_id(&self) -> Option<&str> {
        if self.is_negative_id() {
            Some(&self.id.0)
        } else {
            None
        }
    }
}

/// User display name handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserName {
    /// Regular username.
    Regular(String),
    /// Anonymous posting.
    Anonymous,
    /// Username with nickname in parentheses.
    WithNickname { name: String, nickname: String },
}

impl Default for UserName {
    fn default() -> Self {
        UserName::Anonymous
    }
}

impl UserName {
    /// Create a regular username.
    pub fn regular(name: impl Into<String>) -> Self {
        UserName::Regular(name.into())
    }

    /// Create a username with nickname.
    pub fn with_nickname(name: impl Into<String>, nickname: impl Into<String>) -> Self {
        UserName::WithNickname {
            name: name.into(),
            nickname: nickname.into(),
        }
    }

    /// Parse username from NGA format.
    pub fn parse(raw: &str) -> Self {
        let raw = raw.trim();

        if raw.starts_with("#anon_") || raw.is_empty() {
            return UserName::Anonymous;
        }

        if let Some(paren_start) = raw.find('(') {
            if raw.ends_with(')') {
                let name = raw[..paren_start].trim().to_owned();
                let nickname = raw[paren_start + 1..raw.len() - 1].trim().to_owned();
                return UserName::WithNickname { name, nickname };
            }
        }

        UserName::Regular(raw.to_owned())
    }

    /// Get the display string.
    pub fn display(&self) -> &str {
        match self {
            UserName::Regular(name) => name,
            UserName::Anonymous => "Anonymous",
            UserName::WithNickname { nickname, .. } => nickname,
        }
    }

    /// Get the primary/real username.
    pub fn primary(&self) -> Option<&str> {
        match self {
            UserName::Regular(name) => Some(name),
            UserName::Anonymous => None,
            UserName::WithNickname { name, .. } => Some(name),
        }
    }
}

/// Parse anonymous ID format used by NGA.
/// Format: "-USERID,CONTEXT_HASH" where USERID is the masked user ID.
/// Returns tuple of negative_user_id and context_hash.
#[allow(dead_code)]
pub fn parse_anon_id(raw: &str) -> Option<(i64, &str)> {
    if !raw.starts_with('-') {
        return None;
    }

    let parts: Vec<&str> = raw.splitn(2, ',').collect();
    if parts.len() != 2 {
        return None;
    }

    let user_id: i64 = parts[0].parse().ok()?;
    let context = parts[1];

    Some((user_id, context))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_username_parse() {
        let regular = UserName::parse("TestUser");
        assert!(matches!(regular, UserName::Regular(ref n) if n == "TestUser"));

        let anon = UserName::parse("#anon_abc123");
        assert!(matches!(anon, UserName::Anonymous));

        let nickname = UserName::parse("RealName(DisplayName)");
        match nickname {
            UserName::WithNickname { name, nickname } => {
                assert_eq!(name, "RealName");
                assert_eq!(nickname, "DisplayName");
            }
            _ => panic!("Expected WithNickname"),
        }
    }

    #[test]
    fn test_anon_id_parse() {
        let result = parse_anon_id("-12345,abc123");
        assert_eq!(result, Some((-12345, "abc123")));

        assert!(parse_anon_id("12345,abc").is_none());
        assert!(parse_anon_id("-12345").is_none());
    }

    #[test]
    fn test_user_is_anonymous() {
        let anon = User::anonymous("-12345");
        assert!(anon.is_anonymous());
        assert!(anon.is_negative_id());
    }
}
