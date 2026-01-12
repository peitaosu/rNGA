//! API modules.

mod forum;
mod message;
mod notification;
mod post;
mod topic;
mod user;

pub use forum::ForumApi;
pub use message::{
    ConversationBuilder, ConversationResult, MessageApi, MessageListResult, SendMessageBuilder,
};
pub use notification::{NotificationApi, NotificationListBuilder, NotificationListResult};
pub use post::{
    CommentBuilder, CommentResult, CommentsResult, PostApi, ReplyBuilder, ReplyResult,
    UserPost, UserPostsResult, VoteResult,
};
pub use topic::{
    FavoriteTopicsBuilder, Subforum, TopicApi, TopicDetailsBuilder, TopicDetailsResult,
    TopicListBuilder, TopicListResult, TopicSearchBuilder,
};
pub use user::{UserApi, UserSearchResult};
