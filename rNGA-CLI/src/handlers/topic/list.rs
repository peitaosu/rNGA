use anyhow::Context;
use rnga::models::ForumIdKind;
use rnga::NGAClient;

use crate::handlers::concurrent::fetch_pages_concurrent;
use crate::handlers::meta::{ResponseMeta, MAX_LIST_PAGES};
use crate::handlers::options::ListTopicsOptions;

use super::types::{CliTopicListResult, TopicSummary};
use super::util::{effective_concurrency, parse_order};

pub async fn list_topics(
    client: &NGAClient,
    forum_id: &str,
    options: ListTopicsOptions,
) -> anyhow::Result<CliTopicListResult> {
    let id = ForumIdKind::from_stid_flag(forum_id, options.is_stid);

    let order_by = parse_order(&options.order);
    let start_page = options.start_page.max(1);

    let first_result = client
        .topics()
        .list(id.clone())
        .page(start_page)
        .order(order_by)
        .send()
        .await
        .context("fetching first topic list page")?;

    let total_pages = first_result.total_pages;
    let requested_pages = options.num_pages.max(1);
    let capped_pages = requested_pages.min(MAX_LIST_PAGES);
    let actual_pages = capped_pages.min(total_pages.saturating_sub(start_page - 1));
    let truncated = requested_pages > MAX_LIST_PAGES;
    let forum_name = first_result.forum.as_ref().map(|f| f.name.clone());

    if actual_pages <= 1 {
        return Ok(CliTopicListResult {
            forum_name,
            start_page,
            end_page: start_page,
            total_pages,
            topics: first_result.topics.iter().map(TopicSummary::from).collect(),
            meta: ResponseMeta::list(start_page, total_pages, 1, Vec::new(), truncated),
        });
    }

    let client = client.clone();
    let (page_results, warnings) = fetch_pages_concurrent(
        start_page,
        actual_pages,
        effective_concurrency(options.concurrency),
        move |page| {
            let client = client.clone();
            let id = id.clone();
            async move {
                client
                    .topics()
                    .list(id)
                    .page(page)
                    .order(order_by)
                    .send()
                    .await
                    .context("fetching topic list page")
            }
        },
    )
    .await;

    let mut all_topics = first_result.topics;
    for page_result in page_results {
        all_topics.extend(page_result.topics);
    }

    Ok(CliTopicListResult {
        forum_name,
        start_page,
        end_page: start_page + actual_pages - 1,
        total_pages,
        topics: all_topics.iter().map(TopicSummary::from).collect(),
        meta: ResponseMeta::list(
            start_page,
            total_pages,
            actual_pages,
            warnings,
            truncated,
        ),
    })
}
