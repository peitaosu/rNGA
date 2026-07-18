mod builders;
mod parse;

pub use builders::{
    FavoriteTopicsBuilder, TopicDetailsBuilder, TopicListBuilder, TopicSearchBuilder,
};
pub use parse::Subforum;

use std::sync::Arc;

use crate::{
    client::NGAClientInner,
    error::Result,
    models::{
        FavoriteFolder, FavoriteTopicOp, Forum, ForumIdKind, SearchTimeRange, TopicId,
        TopicOrder,
    },
    parser::XmlDocument,
};

use parse::parse_topic_list_response;

pub struct TopicApi {
    client: Arc<NGAClientInner>,
}

impl TopicApi {
    pub(crate) fn new(client: Arc<NGAClientInner>) -> Self {
        Self { client }
    }

    pub fn list(&self, forum_id: ForumIdKind) -> TopicListBuilder {
        TopicListBuilder {
            client: self.client.clone(),
            forum_id,
            page: 1,
            order: TopicOrder::default(),
            recommended_only: false,
        }
    }

    pub fn details(&self, topic_id: impl Into<TopicId>) -> TopicDetailsBuilder {
        TopicDetailsBuilder {
            client: self.client.clone(),
            topic_id: topic_id.into(),
            page: 1,
            fav: None,
            post_id: None,
            author_id: None,
            anonymous_only: false,
        }
    }

    pub fn search(&self, forum_id: ForumIdKind, keyword: &str) -> TopicSearchBuilder {
        TopicSearchBuilder {
            client: self.client.clone(),
            forum_id,
            keyword: keyword.to_owned(),
            page: 1,
            search_content: false,
            recommended_only: false,
            time_range: SearchTimeRange::default(),
        }
    }

    pub fn favorites(&self) -> FavoriteTopicsBuilder {
        FavoriteTopicsBuilder {
            client: self.client.clone(),
            folder_id: None,
            page: 1,
        }
    }

    pub async fn favorite_folders(&self) -> Result<Vec<FavoriteFolder>> {
        let xml = self
            .client
            .post_authed(
                "nuke.php",
                &[
                    ("__lib", "topic_favor_v2"),
                    ("__act", "list_folder"),
                    ("page", "1"),
                ],
                &[],
            )
            .await?;

        let doc = XmlDocument::parse(&xml)?;
        let mut folders = Vec::new();

        for node in doc.select("/root/data/item/item")? {
            let attrs = node.attrs();
            if let Some(id) = attrs.get("id") {
                folders.push(FavoriteFolder {
                    id: id.clone(),
                    name: attrs.get("name").cloned().unwrap_or_default(),
                    count: attrs
                        .get("length")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0),
                });
            }
        }

        Ok(folders)
    }

    pub async fn modify_favorite(
        &self,
        topic_id: impl AsRef<str>,
        folder_id: impl AsRef<str>,
        op: FavoriteTopicOp,
    ) -> Result<()> {
        let (act, tid_key) = match op {
            FavoriteTopicOp::Add => ("add", "tid"),
            FavoriteTopicOp::Remove => ("del", "tidarray"),
        };

        self.client
            .post_authed(
                "nuke.php",
                &[("__lib", "topic_favor_v2"), ("__act", act)],
                &[(tid_key, topic_id.as_ref()), ("folder", folder_id.as_ref())],
            )
            .await?;

        Ok(())
    }

    pub async fn by_user(&self, user_id: impl AsRef<str>, page: u32) -> Result<TopicListResult> {
        let page_str = page.to_string();
        let xml = self
            .client
            .post(
                "thread.php",
                &[("authorid", user_id.as_ref()), ("page", &page_str)],
                &[],
            )
            .await?;

        parse_topic_list_response(&xml, page)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TopicListResult {
    pub topics: Vec<crate::models::Topic>,
    pub forum: Option<Forum>,
    pub subforums: Vec<Subforum>,
    pub total_pages: u32,
    pub page: u32,
}

#[derive(Debug, Clone, Default)]
pub struct TopicDetailsResult {
    pub topic: crate::models::Topic,
    pub posts: Vec<crate::models::Post>,
    pub forum_name: String,
    pub total_pages: u32,
    pub page: u32,
}

#[cfg(test)]
mod tests {
    use crate::models::TopicOrder;

    #[test]
    fn test_topic_order_param() {
        assert_eq!(TopicOrder::LastPost.param(), "");
        assert_eq!(TopicOrder::PostDate.param(), "postdate");
        assert_eq!(TopicOrder::Recommend.param(), "recommend");
    }
}
