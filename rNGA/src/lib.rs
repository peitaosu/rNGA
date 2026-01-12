//! Rust client library for NGA forum.

pub mod api;
pub mod cache;
pub mod client;
pub mod error;
pub mod models;
pub mod parser;

// Re-export main types
pub use client::{AuthInfo, Device, HttpConfig, NGAClient, NGAClientBuilder};
pub use error::{Error, Result};

// Re-export commonly used models
pub use models::{
    Attachment, AttachmentKind, Category, FavoriteFolder, FavoriteForumOp, FavoriteTopicOp,
    Forum, ForumId, ForumIdKind, LightPost, Notification, NotificationCounts, NotificationType,
    Post, PostContent, PostId, SearchTimeRange, ShortMessage, ShortMessagePost, Span, SpanKind,
    Subject, SubforumFilterOp, Topic, TopicId, TopicOrder, TopicSnapshot, TopicType, User, UserId,
    UserName, Vote, VoteState,
};

// Re-export API types
pub use api::{
    ConversationResult, MessageListResult, NotificationListResult, Subforum, TopicDetailsResult,
    TopicListResult, UserPostsResult, UserSearchResult, VoteResult,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = NGAClient::builder().build();
        assert!(client.is_ok());

        let client = client.unwrap();
        assert!(!client.is_authenticated());
    }

    #[test]
    fn test_client_with_auth() {
        let client = NGAClient::builder()
            .auth("test_token", "12345")
            .build()
            .unwrap();

        assert!(client.is_authenticated());
        assert_eq!(client.current_uid(), Some("12345"));
    }
}
