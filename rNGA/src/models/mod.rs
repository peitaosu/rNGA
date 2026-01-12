//! Data models for NGA entities.

mod content;
mod forum;
mod ids;
mod message;
mod notification;
mod post;
mod topic;
mod user;

pub use content::{PostContent, Span, SpanKind, Subject};
pub use forum::{Category, FavoriteForumOp, Forum, ForumIdKind, SubforumFilterOp};
pub use ids::{ForumId, PostId, TopicId, UserId};
pub use message::{ShortMessage, ShortMessagePost};
pub use notification::{Notification, NotificationCounts, NotificationType};
pub use post::{Attachment, AttachmentKind, LightPost, Post, Vote, VoteState};
pub use topic::{FavoriteFolder, FavoriteTopicOp, SearchTimeRange, Topic, TopicOrder, TopicSnapshot, TopicType};
pub use user::{User, UserName};
