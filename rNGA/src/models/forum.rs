//! Forum and category models.

use serde::{Deserialize, Serialize};

use super::ForumId;
use crate::client::FORUM_ICON_PATH;

/// Forum identifier that can be either a fid or stid.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ForumIdKind {
    /// Regular forum ID.
    Fid(String),
    /// Subforum/topic collection ID.
    Stid(String),
}

impl ForumIdKind {
    /// Create from fid.
    pub fn fid(id: impl Into<String>) -> Self {
        ForumIdKind::Fid(id.into())
    }
    
    /// Create from stid.
    pub fn stid(id: impl Into<String>) -> Self {
        ForumIdKind::Stid(id.into())
    }
    
    /// Get the underlying ID string.
    pub fn id(&self) -> &str {
        match self {
            ForumIdKind::Fid(s) => s,
            ForumIdKind::Stid(s) => s,
        }
    }
    
    /// Check if this is a fid.
    pub fn is_fid(&self) -> bool {
        matches!(self, ForumIdKind::Fid(_))
    }
    
    /// Check if this is a stid.
    pub fn is_stid(&self) -> bool {
        matches!(self, ForumIdKind::Stid(_))
    }
    
    /// Get the query parameter name for this ID type.
    pub fn param_name(&self) -> &'static str {
        match self {
            ForumIdKind::Fid(_) => "fid",
            ForumIdKind::Stid(_) => "stid",
        }
    }
}

impl From<ForumId> for ForumIdKind {
    fn from(id: ForumId) -> Self {
        ForumIdKind::Fid(id.0)
    }
}

/// A forum on NGA.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Forum {
    /// Forum identifier.
    pub id: Option<ForumIdKind>,
    /// Forum name.
    pub name: String,
    /// Forum description/info.
    pub info: String,
    /// Forum icon URL.
    pub icon_url: String,
    /// Pinned topic ID if any.
    pub topped_topic_id: String,
}

impl Forum {
    /// Create a minimal forum with just ID and name.
    pub fn minimal(id: ForumIdKind, name: impl Into<String>) -> Self {
        let icon_id = id.id();
        let icon_url = format!("{}{}.png", FORUM_ICON_PATH, icon_id);
        
        Self {
            id: Some(id),
            name: name.into(),
            info: String::new(),
            icon_url,
            topped_topic_id: String::new(),
        }
    }
    
    /// Get the forum ID string regardless of type.
    pub fn id_str(&self) -> Option<&str> {
        self.id.as_ref().map(|id| id.id())
    }
}

/// A category of forums.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Category {
    /// Category ID.
    pub id: String,
    /// Category name.
    pub name: String,
    /// Forums in this category.
    pub forums: Vec<Forum>,
}

/// Subforum filter operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubforumFilterOp {
    /// Show.
    Show,
    /// Block.
    Block,
}

impl SubforumFilterOp {
    /// Get the API parameter value.
    pub fn param(&self) -> &'static str {
        match self {
            SubforumFilterOp::Show => "del",
            SubforumFilterOp::Block => "add",
        }
    }
}

/// Favorite forum modification operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FavoriteForumOp {
    /// Add to favorites.
    Add,
    /// Remove from favorites.
    Remove,
}

impl FavoriteForumOp {
    /// Get the API parameter value.
    pub fn param(&self) -> &'static str {
        match self {
            FavoriteForumOp::Add => "add",
            FavoriteForumOp::Remove => "del",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forum_id_kind() {
        let fid = ForumIdKind::fid("123");
        assert!(fid.is_fid());
        assert_eq!(fid.id(), "123");
        assert_eq!(fid.param_name(), "fid");

        let stid = ForumIdKind::stid("456");
        assert!(stid.is_stid());
        assert_eq!(stid.id(), "456");
        assert_eq!(stid.param_name(), "stid");
    }

    #[test]
    fn test_forum_minimal() {
        let forum = Forum::minimal(ForumIdKind::fid("123"), "Test Forum");
        assert_eq!(forum.name, "Test Forum");
        assert!(forum.icon_url.contains("123.png"));
    }
}
