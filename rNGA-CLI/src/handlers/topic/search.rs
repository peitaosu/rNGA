use anyhow::Context;
use rnga::models::ForumIdKind;
use rnga::NGAClient;

use crate::handlers::meta::ResponseMeta;
use crate::handlers::options::SearchTopicsOptions;

use super::types::{CliTopicSearchResult, TopicSummary};

pub async fn search_topics(
    client: &NGAClient,
    forum_id: &str,
    keyword: &str,
    options: SearchTopicsOptions,
) -> anyhow::Result<CliTopicSearchResult> {
    let id = ForumIdKind::from_stid_flag(forum_id, options.is_stid);

    let result = client
        .topics()
        .search(id, keyword)
        .page(options.page)
        .search_content(options.search_content)
        .send()
        .await
        .context("searching topics")?;

    Ok(CliTopicSearchResult {
        keyword: keyword.to_string(),
        page: options.page,
        total_pages: result.total_pages,
        topics: result.topics.iter().map(TopicSummary::from).collect(),
        meta: ResponseMeta::page_only(options.page, result.total_pages),
    })
}
