use std::sync::Arc;

use anyhow::Context;
use chrono::Local;
use futures::stream::{self, StreamExt};
use rnga::models::{ForumIdKind, Post, Topic, TopicOrder};
use rnga::NGAClient;
use tokio::sync::Semaphore;

use crate::handlers::meta::{ResponseMeta, MAX_READ_PAGES, MAX_RECENT_TOPICS};
use crate::handlers::options::RecentTopicsOptions;

use super::types::{CliRecentResult, RecentPostInfo, TopicSummary};
use super::util::{effective_concurrency, parse_order, parse_time_range};

pub async fn recent_topics(
    client: &NGAClient,
    forum_id: &str,
    options: RecentTopicsOptions,
) -> anyhow::Result<CliRecentResult> {
    let now = Local::now().timestamp();

    let (time_range_seconds, range_display) =
        parse_time_range(&options.range).unwrap_or((3600, "hour".to_string()));

    let cutoff_time = now - time_range_seconds;

    let id = ForumIdKind::from_stid_flag(forum_id, options.is_stid);

    let order_by = parse_order(&options.order);

    let mut current_page = 1;
    let mut all_recent_topics: Vec<Topic> = Vec::new();
    let mut forum_name: Option<String> = None;

    loop {
        let result = client
            .topics()
            .list(id.clone())
            .page(current_page)
            .order(order_by)
            .send()
            .await
            .context("fetching topic list page for recent topics")?;

        if forum_name.is_none() {
            forum_name = result.forum.as_ref().map(|f| f.name.clone());
        }

        let mut found_any_recent = false;
        let mut all_older_than_cutoff = true;

        for topic in result.topics {
            let relevant_time = match order_by {
                TopicOrder::PostDate => topic.post_date,
                _ => topic.last_post_date,
            };

            if relevant_time >= cutoff_time {
                all_recent_topics.push(topic);
                found_any_recent = true;
                all_older_than_cutoff = false;
            }
        }

        if all_older_than_cutoff || current_page >= result.total_pages {
            break;
        }

        if !found_any_recent {
            break;
        }

        current_page += 1;
    }

    let mut truncated = false;
    if all_recent_topics.len() > MAX_RECENT_TOPICS {
        all_recent_topics.truncate(MAX_RECENT_TOPICS);
        truncated = true;
    }

    if !options.with_posts {
        return Ok(CliRecentResult {
            forum_name,
            range_display,
            topics: all_recent_topics.iter().map(TopicSummary::from).collect(),
            posts: Vec::new(),
            meta: ResponseMeta {
                truncated,
                ..ResponseMeta::empty()
            },
        });
    }

    if all_recent_topics.is_empty() {
        return Ok(CliRecentResult {
            forum_name,
            range_display,
            topics: Vec::new(),
            posts: Vec::new(),
            meta: ResponseMeta::empty(),
        });
    }

    let concurrency = effective_concurrency(options.concurrency);
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let client = Arc::new(client.clone());

    let fetch_results: Vec<_> = stream::iter(all_recent_topics.iter().cloned())
        .map(|topic| {
            let sem = semaphore.clone();
            let client = client.clone();
            async move {
                let _permit = sem.acquire().await.unwrap();
                let result = fetch_topic_posts(&client, &topic, cutoff_time).await;
                (topic, result)
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let mut all_posts: Vec<RecentPostInfo> = Vec::new();
    for (topic, result) in fetch_results {
        if let Ok(posts) = result {
            all_posts.extend(posts.into_iter().map(
                |(post_type, post_id, floor, author_name, author_id, content, post_date, score)| {
                    RecentPostInfo {
                        topic_id: topic.id.to_string(),
                        topic_subject: topic.subject.content.clone(),
                        post_type,
                        post_id,
                        floor,
                        author_id,
                        author_name,
                        content,
                        post_date,
                        score,
                    }
                },
            ));
        }
    }

    all_posts.sort_by(|a, b| b.post_date.cmp(&a.post_date));

    Ok(CliRecentResult {
        forum_name,
        range_display,
        topics: all_recent_topics.iter().map(TopicSummary::from).collect(),
        posts: all_posts,
        meta: ResponseMeta {
            truncated,
            ..ResponseMeta::empty()
        },
    })
}

async fn fetch_topic_posts(
    client: &NGAClient,
    topic: &Topic,
    cutoff_time: i64,
) -> anyhow::Result<Vec<(String, String, String, String, String, String, i64, i32)>> {
    let mut results = Vec::new();
    let mut posts_to_check_comments: Vec<Post> = Vec::new();

    let first_page = client
        .topics()
        .details(topic.id.clone())
        .page(1)
        .send()
        .await
        .context("fetching topic details for recent posts")?;

    let total_pages = first_page.total_pages.min(MAX_READ_PAGES);
    let mut current_page = total_pages;
    let mut pages_scanned = 0u32;

    loop {
        pages_scanned += 1;
        if pages_scanned > MAX_READ_PAGES {
            break;
        }

        let details = if current_page == 1 {
            first_page.clone()
        } else {
            client
                .topics()
                .details(topic.id.clone())
                .page(current_page)
                .send()
                .await
                .context("fetching topic page for recent posts")?
        };

        let mut found_any_recent = false;

        for post in details.posts {
            if post.post_date >= cutoff_time {
                found_any_recent = true;

                if post.comment_count > 0 {
                    posts_to_check_comments.push(post.clone());
                }

                results.push((
                    "post".to_string(),
                    post.id.to_string(),
                    format!("#{}", post.floor),
                    post.author.name.display().to_string(),
                    post.author.id.to_string(),
                    post.content.to_plain_text(),
                    post.post_date,
                    post.score,
                ));
            }
        }

        if !found_any_recent || current_page == 1 {
            break;
        }

        current_page -= 1;
    }

    for post in posts_to_check_comments {
        if let Ok(first_comments) = client.posts().comments(&topic.id, &post.id, 1).await {
            let total_comment_pages = first_comments.total_pages.min(MAX_READ_PAGES);
            let mut comment_page = total_comment_pages;
            let mut comment_pages_scanned = 0u32;

            loop {
                comment_pages_scanned += 1;
                if comment_pages_scanned > MAX_READ_PAGES {
                    break;
                }

                let comments_result = if comment_page == 1 {
                    first_comments.clone()
                } else {
                    match client
                        .posts()
                        .comments(&topic.id, &post.id, comment_page)
                        .await
                    {
                        Ok(result) => result,
                        Err(_) => break,
                    }
                };

                let mut found_any_recent_comment = false;

                for comment in comments_result.comments {
                    if comment.post_date >= cutoff_time {
                        found_any_recent_comment = true;

                        results.push((
                            "comment".to_string(),
                            comment.id.to_string(),
                            format!("#{} comment", post.floor),
                            comment.author.name.display().to_string(),
                            comment.author.id.to_string(),
                            comment.content.to_plain_text(),
                            comment.post_date,
                            comment.score,
                        ));
                    }
                }

                if !found_any_recent_comment || comment_page == 1 {
                    break;
                }

                comment_page -= 1;
            }
        }
    }

    Ok(results)
}
