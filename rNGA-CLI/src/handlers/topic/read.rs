use anyhow::Context;
use chrono::Local;
use rnga::models::Post;
use rnga::NGAClient;

use crate::handlers::concurrent::fetch_pages_concurrent;
use crate::handlers::meta::{ResponseMeta, MAX_RANGE_PAGES, MAX_READ_PAGES};
use crate::handlers::options::ReadTopicOptions;

use super::types::{CliTopicDetailsResult, PostInfo};
use super::util::{effective_concurrency, parse_time_range};

pub async fn read_topic(
    client: &NGAClient,
    topic_id: &str,
    options: ReadTopicOptions,
) -> anyhow::Result<CliTopicDetailsResult> {
    let cutoff_time = if let Some(ref range) = options.range {
        let now = Local::now().timestamp();
        let (time_range_seconds, _) =
            parse_time_range(range).unwrap_or((3600, "hour".to_string()));
        Some(now - time_range_seconds)
    } else {
        None
    };

    let mut builder = client.topics().details(topic_id).page(1);
    if let Some(ref author_id) = options.author {
        builder = builder.author(author_id.clone());
    }

    let first_result = builder.send().await.context("fetching topic details")?;

    let topic = &first_result.topic;
    let forum_name = first_result.forum_name.clone();
    let subject = topic.subject.content.clone();
    let tags = topic.subject.tags.clone();
    let author = topic.author.name.display().to_string();
    let author_id = topic.author.id.to_string();
    let replies = topic.replies;
    let post_date = topic.post_date;
    let total_pages = first_result.total_pages;

    if !options.fetch_all && total_pages == 1 {
        let posts: Vec<PostInfo> = if let Some(cutoff) = cutoff_time {
            first_result
                .posts
                .iter()
                .filter(|p| p.post_date >= cutoff)
                .map(PostInfo::from)
                .collect()
        } else {
            first_result.posts.iter().map(PostInfo::from).collect()
        };

        return Ok(details_result(
            topic_id,
            forum_name,
            subject,
            tags,
            author,
            author_id,
            replies,
            post_date,
            options.page,
            total_pages,
            posts,
            ResponseMeta::page_only(options.page, total_pages),
        ));
    }

    if !options.fetch_all && cutoff_time.is_none() {
        let page = options.page.max(1);

        let page_result = if page == 1 {
            first_result
        } else {
            let mut builder = client.topics().details(topic_id).page(page);
            if let Some(ref aid) = options.author {
                builder = builder.author(aid.clone());
            }
            builder
                .send()
                .await
                .context("fetching topic page")?
        };

        return Ok(details_result(
            topic_id,
            forum_name,
            subject,
            tags,
            author,
            author_id,
            replies,
            post_date,
            page,
            total_pages,
            page_result.posts.iter().map(PostInfo::from).collect(),
            ResponseMeta::page_only(page, total_pages),
        ));
    }

    let mut all_posts: Vec<Post>;
    let meta;

    if cutoff_time.is_some() {
        all_posts = Vec::new();
        let mut current_page = total_pages;
        let mut pages_scanned = 0u32;
        let mut truncated = false;

        loop {
            pages_scanned += 1;
            if pages_scanned > MAX_RANGE_PAGES {
                truncated = true;
                break;
            }

            let details = if current_page == 1 {
                first_result.clone()
            } else {
                let mut builder = client.topics().details(topic_id).page(current_page);
                if let Some(ref aid) = options.author {
                    builder = builder.author(aid.clone());
                }
                builder
                    .send()
                    .await
                    .context("fetching topic page for time range")?
            };

            let mut found_any_recent = false;
            for post in details.posts {
                if post.post_date >= cutoff_time.unwrap() {
                    found_any_recent = true;
                    all_posts.push(post);
                }
            }

            if !found_any_recent || current_page == 1 {
                break;
            }

            current_page -= 1;
        }

        meta = ResponseMeta {
            page: Some(1),
            total_pages: Some(total_pages),
            fetched_pages: Some(pages_scanned),
            warnings: Vec::new(),
            truncated,
        };
    } else {
        let fetch_pages = total_pages.min(MAX_READ_PAGES);
        let truncated = total_pages > MAX_READ_PAGES;
        let client = client.clone();
        let author_clone = options.author.clone();
        let topic_id_owned = topic_id.to_string();
        let (page_results, warnings) = fetch_pages_concurrent(
            1,
            fetch_pages,
            effective_concurrency(options.concurrency),
            move |page| {
                let client = client.clone();
                let author_id = author_clone.clone();
                let topic_id = topic_id_owned.clone();
                async move {
                    let mut builder = client.topics().details(&topic_id).page(page);
                    if let Some(ref aid) = author_id {
                        builder = builder.author(aid.clone());
                    }
                    builder
                        .send()
                        .await
                        .context("fetching topic page")
                }
            },
        )
        .await;

        all_posts = first_result.posts;
        for page_result in page_results {
            all_posts.extend(page_result.posts);
        }

        meta = ResponseMeta::list(1, total_pages, fetch_pages, warnings, truncated);
    }

    all_posts.sort_by_key(|p| p.floor);

    Ok(details_result(
        topic_id,
        forum_name,
        subject,
        tags,
        author,
        author_id,
        replies,
        post_date,
        1,
        total_pages,
        all_posts.iter().map(PostInfo::from).collect(),
        meta,
    ))
}

fn details_result(
    topic_id: &str,
    forum_name: String,
    subject: String,
    tags: Vec<String>,
    author: String,
    author_id: String,
    replies: i32,
    post_date: i64,
    page: u32,
    total_pages: u32,
    posts: Vec<PostInfo>,
    meta: ResponseMeta,
) -> CliTopicDetailsResult {
    CliTopicDetailsResult {
        topic_id: topic_id.to_string(),
        forum_name,
        subject,
        tags,
        author,
        author_id,
        replies,
        post_date,
        page,
        total_pages,
        posts,
        meta,
    }
}
