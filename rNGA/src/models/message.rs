//! Short message models.

use serde::{Deserialize, Serialize};

use super::{PostContent, UserId};

/// A short message conversation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortMessage {
    /// Conversation ID.
    pub id: String,
    /// The other participant's user ID.
    pub other_user_id: UserId,
    /// The other participant's username.
    pub other_username: String,
    /// Subject of the conversation.
    pub subject: String,
    /// Last message time.
    pub last_time: i64,
    /// Whether there are unread messages.
    pub is_unread: bool,
    /// Number of messages in conversation.
    pub message_count: i32,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortMessagePost {
    /// Message ID.
    pub id: String,
    /// Sender user ID.
    pub from_user_id: UserId,
    /// Sender username.
    pub from_username: String,
    /// Message content.
    pub content: PostContent,
    /// Message time.
    pub time: i64,
    /// Whether this message is from current user.
    pub is_mine: bool,
}

impl ShortMessagePost {
    /// Mark this message as from the current user.
    pub fn mark_as_mine(mut self) -> Self {
        self.is_mine = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_message_post() {
        let msg = ShortMessagePost {
            id: "123".into(),
            from_user_id: "456".into(),
            is_mine: false,
            ..Default::default()
        };

        let mine = msg.clone().mark_as_mine();
        assert!(mine.is_mine);
        assert!(!msg.is_mine);
    }
}
