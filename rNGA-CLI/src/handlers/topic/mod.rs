mod favorites;
mod list;
mod read;
mod recent;
mod search;
mod types;
mod util;

pub use favorites::{add_favorite, list_favorites, list_folders, remove_favorite};
pub use list::list_topics;
pub use read::read_topic;
pub use recent::recent_topics;
pub use search::search_topics;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use rnga::models::{Post, PostContent, PostId, TopicId, User};

    #[test]
    fn test_post_info_exposes_stable_ids() {
        let post = Post {
            id: PostId::new("42"),
            topic_id: TopicId::new("100"),
            floor: 3,
            author: User::anonymous("7"),
            content: PostContent::plain("test"),
            ..Default::default()
        };
        let info = PostInfo::from(&post);
        assert_eq!(info.post_id, "42");
        assert_eq!(info.topic_id, "100");
        assert_eq!(info.content, "test");
    }

    #[test]
    fn test_list_pages_cap() {
        use crate::handlers::meta::MAX_LIST_PAGES;
        let requested = 10u32;
        let capped = requested.min(MAX_LIST_PAGES);
        assert_eq!(capped, MAX_LIST_PAGES);
        assert!(requested > MAX_LIST_PAGES);
    }
}
