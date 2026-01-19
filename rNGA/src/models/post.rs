//! Post and reply models.

use serde::{Deserialize, Serialize};

use super::{PostContent, PostId, TopicId, User};

/// A post/reply in a topic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Post {
    /// Post ID.
    pub id: PostId,
    /// Topic this post belongs to.
    pub topic_id: TopicId,
    /// Post floor number.
    pub floor: i32,
    /// Author of the post.
    pub author: User,
    /// Parsed post content.
    pub content: PostContent,
    /// Post time.
    pub post_date: i64,
    /// Edit time if edited.
    pub edit_date: Option<i64>,
    /// Whether this post has been edited.
    pub is_edited: bool,
    /// Attachments.
    pub attachments: Vec<Attachment>,
    /// Vote state for this post.
    pub vote: VoteState,
    /// Post score/rating.
    pub score: i32,
    /// Whether this post is hidden/collapsed.
    pub is_hidden: bool,
    /// Device/client used to post.
    pub from_device: Option<String>,
    /// Signature line.
    pub signature: Option<String>,
    /// Hot replies under this post.
    pub hot_replies: Vec<LightPost>,
    /// Comments under this post.
    pub comments: Vec<LightPost>,
    /// Comment count.
    pub comment_count: i32,
}

impl Post {
    /// Check if this is the main post.
    pub fn is_main(&self) -> bool {
        self.floor == 0
    }
}

/// A light-weight post representation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LightPost {
    /// Post ID.
    pub id: PostId,
    /// Author.
    pub author: User,
    /// Parsed content.
    pub content: PostContent,
    /// Post time.
    pub post_date: i64,
    /// Post score.
    pub score: i32,
}

/// Attachment on a post.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Attachment URL.
    pub url: String,
    /// Original filename.
    pub name: String,
    /// File size in bytes.
    pub size: i64,
    /// Attachment type.
    pub kind: AttachmentKind,
    /// Thumbnail URL for images.
    pub thumb_url: Option<String>,
    /// Image dimensions if applicable.
    pub dimensions: Option<(u32, u32)>,
}

/// Attachment type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentKind {
    /// Image attachment.
    Image,
    /// Video attachment.
    Video,
    /// Audio attachment.
    Audio,
    /// Other file type.
    File,
}

impl AttachmentKind {
    /// Determine attachment kind from extension or mime type.
    pub fn from_ext(ext: &str) -> Self {
        let ext = ext.to_lowercase();
        match ext.as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" => AttachmentKind::Image,
            "mp4" | "webm" | "mov" | "avi" | "mkv" => AttachmentKind::Video,
            "mp3" | "wav" | "ogg" | "m4a" | "flac" => AttachmentKind::Audio,
            _ => AttachmentKind::File,
        }
    }
}

impl Default for AttachmentKind {
    fn default() -> Self {
        AttachmentKind::File
    }
}

/// Vote state for a post.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteState {
    /// Upvote count.
    pub up: i32,
    /// Downvote count.
    pub down: i32,
    /// Current user's vote.
    pub user_vote: Option<Vote>,
}

impl VoteState {
    /// Calculate net score.
    pub fn net(&self) -> i32 {
        self.up - self.down
    }
}

/// User's vote direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    /// Upvote.
    Up,
    /// Downvote.
    Down,
}

impl Vote {
    /// Get the API parameter value.
    pub fn param(&self) -> &'static str {
        match self {
            Vote::Up => "1",
            Vote::Down => "0",
        }
    }
}

/// Post reply mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ReplyMode {
    /// Reply to topic.
    Topic,
    /// Quote an existing post.
    Quote,
    /// Comment on an existing post.
    Comment,
}

/// Post action type for reporting, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PostAction {
    /// Report post.
    Report,
    /// Delete post.
    Delete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_kind() {
        assert_eq!(AttachmentKind::from_ext("jpg"), AttachmentKind::Image);
        assert_eq!(AttachmentKind::from_ext("MP4"), AttachmentKind::Video);
        assert_eq!(AttachmentKind::from_ext("mp3"), AttachmentKind::Audio);
        assert_eq!(AttachmentKind::from_ext("zip"), AttachmentKind::File);
    }

    #[test]
    fn test_vote_state_net() {
        let state = VoteState {
            up: 10,
            down: 3,
            user_vote: None,
        };
        assert_eq!(state.net(), 7);
    }

    #[test]
    fn test_post_is_main() {
        let main = Post {
            floor: 0,
            ..Default::default()
        };
        assert!(main.is_main());

        let reply = Post {
            floor: 1,
            ..Default::default()
        };
        assert!(!reply.is_main());
    }
}
