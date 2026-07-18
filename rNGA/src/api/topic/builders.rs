use std::sync::Arc;

use crate::{
    client::NGAClientInner,
    error::Result,
    models::{ForumIdKind, SearchTimeRange, TopicId, TopicOrder},
};

use super::parse::{parse_topic_details_response, parse_topic_list_response};
use super::{TopicDetailsResult, TopicListResult};

pub struct TopicListBuilder {
    pub(crate) client: Arc<NGAClientInner>,
    pub(crate) forum_id: ForumIdKind,
    pub(crate) page: u32,
    pub(crate) order: TopicOrder,
    pub(crate) recommended_only: bool,
}

impl TopicListBuilder {
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    pub fn order(mut self, order: TopicOrder) -> Self {
        self.order = order;
        self
    }

    pub fn recommended_only(mut self, recommended: bool) -> Self {
        self.recommended_only = recommended;
        self
    }

    pub async fn send(self) -> Result<TopicListResult> {
        let page_str = self.page.to_string();
        let recommend_str = if self.recommended_only { "1" } else { "" };

        let xml = self
            .client
            .post(
                "thread.php",
                &[
                    (self.forum_id.param_name(), self.forum_id.id()),
                    ("page", &page_str),
                    ("order_by", self.order.param()),
                    ("recommend", recommend_str),
                ],
                &[],
            )
            .await?;

        let result = parse_topic_list_response(&xml, self.page)?;

        Ok(result)
    }
}

pub struct TopicDetailsBuilder {
    pub(crate) client: Arc<NGAClientInner>,
    pub(crate) topic_id: TopicId,
    pub(crate) page: u32,
    pub(crate) fav: Option<String>,
    pub(crate) post_id: Option<String>,
    pub(crate) author_id: Option<String>,
    pub(crate) anonymous_only: bool,
}

impl TopicDetailsBuilder {
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    pub fn fav(mut self, fav: impl Into<String>) -> Self {
        self.fav = Some(fav.into());
        self
    }

    pub fn post(mut self, post_id: impl Into<String>) -> Self {
        self.post_id = Some(post_id.into());
        self
    }

    pub fn author(mut self, author_id: impl Into<String>) -> Self {
        self.author_id = Some(author_id.into());
        self
    }

    pub fn anonymous_only(mut self, only: bool) -> Self {
        self.anonymous_only = only;
        self
    }

    pub async fn send(self) -> Result<TopicDetailsResult> {
        let page_str = self.page.to_string();
        let opt = if self.anonymous_only { "512" } else { "" };

        let xml = self
            .client
            .post(
                "read.php",
                &[
                    ("tid", self.topic_id.as_str()),
                    ("page", &page_str),
                    ("fav", self.fav.as_deref().unwrap_or("")),
                    ("pid", self.post_id.as_deref().unwrap_or("")),
                    ("authorid", self.author_id.as_deref().unwrap_or("")),
                    ("opt", opt),
                ],
                &[],
            )
            .await?;

        parse_topic_details_response(&xml, self.page)
    }
}

pub struct TopicSearchBuilder {
    pub(crate) client: Arc<NGAClientInner>,
    pub(crate) forum_id: ForumIdKind,
    pub(crate) keyword: String,
    pub(crate) page: u32,
    pub(crate) search_content: bool,
    pub(crate) recommended_only: bool,
    pub(crate) time_range: SearchTimeRange,
}

impl TopicSearchBuilder {
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    pub fn search_content(mut self, search: bool) -> Self {
        self.search_content = search;
        self
    }

    pub fn recommended_only(mut self, recommended: bool) -> Self {
        self.recommended_only = recommended;
        self
    }

    pub fn time_range(mut self, range: SearchTimeRange) -> Self {
        self.time_range = range;
        self
    }

    pub async fn send(self) -> Result<TopicListResult> {
        let page_str = self.page.to_string();
        let content_str = if self.search_content { "1" } else { "" };
        let recommend_str = if self.recommended_only { "1" } else { "" };

        let xml = self
            .client
            .post(
                "thread.php",
                &[
                    (self.forum_id.param_name(), self.forum_id.id()),
                    ("key", &self.keyword),
                    ("page", &page_str),
                    ("content", content_str),
                    ("recommend", recommend_str),
                ],
                &[],
            )
            .await?;

        parse_topic_list_response(&xml, self.page)
    }
}

pub struct FavoriteTopicsBuilder {
    pub(crate) client: Arc<NGAClientInner>,
    pub(crate) folder_id: Option<String>,
    pub(crate) page: u32,
}

impl FavoriteTopicsBuilder {
    pub fn folder(mut self, folder_id: impl Into<String>) -> Self {
        self.folder_id = Some(folder_id.into());
        self
    }

    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    pub async fn send(self) -> Result<TopicListResult> {
        let page_str = self.page.to_string();
        let folder = self.folder_id.as_deref().unwrap_or("");

        let xml = self
            .client
            .post_authed("thread.php", &[("favor", folder), ("page", &page_str)], &[])
            .await?;

        parse_topic_list_response(&xml, self.page)
    }
}
