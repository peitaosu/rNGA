//! Notification models.

use serde::{Deserialize, Serialize};

use super::{PostId, TopicId, UserId};

/// A notification for the current user.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Notification {
    /// Notification ID.
    pub id: String,
    /// Type of notification.
    pub kind: NotificationType,
    /// Notification content/message.
    pub content: String,
    /// Time when notification was created.
    pub time: i64,
    /// Whether notification has been read.
    pub is_read: bool,
    /// Related topic ID if applicable.
    pub topic_id: Option<TopicId>,
    /// Related post ID if applicable.
    pub post_id: Option<PostId>,
    /// User who triggered the notification.
    pub from_user_id: Option<UserId>,
    /// From username.
    pub from_username: Option<String>,
}

/// Type of notification.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    /// Reply to your post.
    #[default]
    Reply,
    /// Quote of your post.
    Quote,
    /// @ mention.
    Mention,
    /// System message.
    System,
    /// Punishment/warning.
    Punishment,
    /// Short message.
    Message,
    /// Comment on your post.
    Comment,
    /// Other/unknown.
    Other,
}

impl NotificationType {
    /// Parse from NGA notification type number.
    pub fn from_type_id(id: i32) -> Self {
        match id {
            1 => NotificationType::Reply,
            2 => NotificationType::Quote,
            3 => NotificationType::Mention,
            4 => NotificationType::System,
            5 => NotificationType::Punishment,
            6 => NotificationType::Message,
            7 => NotificationType::Comment,
            _ => NotificationType::Other,
        }
    }
    
    /// Get the API parameter value for fetching.
    pub fn param(&self) -> &'static str {
        match self {
            NotificationType::Reply => "reply",
            NotificationType::Quote => "quote",
            NotificationType::Mention => "at",
            NotificationType::System => "system",
            NotificationType::Punishment => "punishment",
            NotificationType::Message => "message",
            NotificationType::Comment => "comment",
            NotificationType::Other => "",
        }
    }
}

/// Notification counts by type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationCounts {
    /// Unread replies.
    pub replies: i32,
    /// Unread quotes.
    pub quotes: i32,
    /// Unread mentions.
    pub mentions: i32,
    /// Unread comments.
    pub comments: i32,
    /// Unread system messages.
    pub system: i32,
    /// Unread PMs.
    pub messages: i32,
}

impl NotificationCounts {
    /// Total unread count.
    pub fn total(&self) -> i32 {
        self.replies + self.quotes + self.mentions + self.comments + self.system + self.messages
    }
    
    /// Check if there are any unread notifications.
    pub fn has_unread(&self) -> bool {
        self.total() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type_from_id() {
        assert_eq!(NotificationType::from_type_id(1), NotificationType::Reply);
        assert_eq!(NotificationType::from_type_id(3), NotificationType::Mention);
        assert_eq!(NotificationType::from_type_id(99), NotificationType::Other);
    }

    #[test]
    fn test_notification_counts() {
        let counts = NotificationCounts {
            replies: 5,
            quotes: 2,
            mentions: 1,
            ..Default::default()
        };
        assert_eq!(counts.total(), 8);
        assert!(counts.has_unread());
    }
}
