//! Topic models.

use serde::{Deserialize, Serialize};

use super::{ForumIdKind, Subject, TopicId, User};

/// A topic/thread on NGA.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Topic {
    /// Topic ID.
    pub id: TopicId,
    /// Forum this topic belongs to.
    pub forum_id: Option<ForumIdKind>,
    /// Topic subject/title.
    pub subject: Subject,
    /// Author of the topic.
    pub author: User,
    /// Post date.
    pub post_date: i64,
    /// Last post date.
    pub last_post_date: i64,
    /// Number of replies.
    pub replies: i32,
    /// Last poster name.
    pub last_poster: String,
    /// Whether topic is locked.
    pub is_locked: bool,
    /// Whether topic is bold/highlighted.
    pub is_bold: bool,
    /// Whether topic is assembly.
    pub is_assembly: bool,
    /// Whether topic is pinned/topped.
    pub is_topped: bool,
    /// Topic type.
    pub topic_type: TopicType,
    /// Parent topic ID if this is a subtopic.
    pub parent_id: Option<TopicId>,
    /// Recommendation score.
    pub recommend: i32,
}

impl Topic {
    /// Create a topic with just an ID.
    pub fn with_id(id: impl Into<TopicId>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }
}

/// Topic type enumeration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopicType {
    /// Normal discussion topic.
    #[default]
    Normal,
    /// Poll topic.
    Poll,
    /// Debate topic.
    Debate,
    /// Combined/assembled topic.
    Assembly,
}

impl From<i32> for TopicType {
    fn from(value: i32) -> Self {
        match value {
            1 => TopicType::Poll,
            2 => TopicType::Debate,
            4 | 8 | 16 => TopicType::Assembly,
            _ => TopicType::Normal,
        }
    }
}

/// A snapshot of a topic for history purposes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopicSnapshot {
    /// Topic ID.
    pub id: TopicId,
    /// Topic subject/title.
    pub subject: Subject,
    /// Forum name.
    pub forum_name: String,
    /// When this snapshot was created.
    pub timestamp: i64,
}

impl TopicSnapshot {
    /// Create a new snapshot from a topic.
    pub fn from_topic(topic: &Topic, forum_name: impl Into<String>) -> Self {
        Self {
            id: topic.id.clone(),
            subject: topic.subject.clone(),
            forum_name: forum_name.into(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Topic list sort order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TopicOrder {
    /// Sort by last post date.
    #[default]
    LastPost,
    /// Sort by creation date.
    PostDate,
    /// Sort by recommendation.
    Recommend,
}

impl TopicOrder {
    /// Get the API parameter value.
    pub fn param(&self) -> &'static str {
        match self {
            TopicOrder::LastPost => "",
            TopicOrder::PostDate => "postdate",
            TopicOrder::Recommend => "recommend",
        }
    }
}

/// Topic search time range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchTimeRange {
    /// All time.
    #[default]
    All,
    /// Within one day.
    Day,
    /// Within one week.
    Week,
    /// Within one month.
    Month,
    /// Within one year.
    Year,
}

impl SearchTimeRange {
    /// Get the API parameter value.
    pub fn param(&self) -> &'static str {
        match self {
            SearchTimeRange::All => "",
            SearchTimeRange::Day => "86400",
            SearchTimeRange::Week => "604800",
            SearchTimeRange::Month => "2592000",
            SearchTimeRange::Year => "31536000",
        }
    }
}

/// Favorite folder for topics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FavoriteFolder {
    /// Folder ID.
    pub id: String,
    /// Folder name.
    pub name: String,
    /// Number of topics in folder.
    pub count: i32,
}

/// Favorite topic operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FavoriteTopicOp {
    /// Add to favorites.
    Add,
    /// Remove from favorites.
    Remove,
}

impl FavoriteTopicOp {
    /// Get the API parameter value.
    pub fn param(&self) -> &'static str {
        match self {
            FavoriteTopicOp::Add => "add",
            FavoriteTopicOp::Remove => "del",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_type_from_i32() {
        assert_eq!(TopicType::from(0), TopicType::Normal);
        assert_eq!(TopicType::from(1), TopicType::Poll);
        assert_eq!(TopicType::from(2), TopicType::Debate);
        assert_eq!(TopicType::from(4), TopicType::Assembly);
    }

    #[test]
    fn test_topic_order_param() {
        assert_eq!(TopicOrder::LastPost.param(), "");
        assert_eq!(TopicOrder::PostDate.param(), "postdate");
    }
}
