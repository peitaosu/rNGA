//! Topic handlers.

use anyhow::Result;
use colored::Colorize;
use futures::stream::{self, StreamExt};
use rnga::models::*;
use rnga::NGAClient;
use rust_i18n::t;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::output::{format_relative_time, PlainPrint, TableRow};

/// Topic information.
#[derive(Debug, Clone, Serialize)]
pub struct TopicInfo {
    pub id: String,
    pub subject: String,
    pub tags: Vec<String>,
    pub author: String,
    pub author_id: String,
    pub replies: i32,
    pub post_date: i64,
    pub last_post_date: i64,
}

impl From<&Topic> for TopicInfo {
    fn from(t: &Topic) -> Self {
        Self {
            id: t.id.to_string(),
            subject: t.subject.content.clone(),
            tags: t.subject.tags.clone(),
            author: t.author.name.display().to_string(),
            author_id: t.author.id.to_string(),
            replies: t.replies,
            post_date: t.post_date,
            last_post_date: t.last_post_date,
        }
    }
}

impl TableRow for TopicInfo {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Subject", "Author", "Replies", "Last Post"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.subject.clone(),
            self.author.clone(),
            self.replies.to_string(),
            format_relative_time(self.last_post_date),
        ]
    }
}

impl PlainPrint for TopicInfo {
    fn plain_print(&self) {
        println!(
            "{} {}",
            t!("topic_label", id = &self.id).to_string().cyan(),
            self.subject.bold()
        );
        println!(
            "   {} {} | {} | {}",
            t!("by_label", author = self.author.green()),
            t!("uid_label", id = &self.author_id).to_string().dimmed(),
            format_relative_time(self.last_post_date).dimmed(),
            t!("replies_label", count = self.replies)
        );
    }
}

/// Post information.
#[derive(Debug, Clone, Serialize)]
pub struct PostInfo {
    pub floor: i32,
    pub post_id: String,
    pub topic_id: String,
    pub author: String,
    pub author_id: String,
    pub content: String,
    pub score: i32,
    pub post_date: i64,
    pub comment_count: i32,
}

impl From<&Post> for PostInfo {
    fn from(p: &Post) -> Self {
        Self {
            floor: p.floor,
            post_id: p.id.to_string(),
            topic_id: p.topic_id.to_string(),
            author: p.author.name.display().to_string(),
            author_id: p.author.id.to_string(),
            content: p.content.to_plain_text(),
            score: p.score,
            post_date: p.post_date,
            comment_count: p.comment_count,
        }
    }
}

impl TableRow for PostInfo {
    fn headers() -> Vec<&'static str> {
        vec!["#", "Author", "Content", "Score", "Time"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.floor.to_string(),
            self.author.clone(),
            self.content.clone(),
            self.score.to_string(),
            format_relative_time(self.post_date),
        ]
    }
}

impl PlainPrint for PostInfo {
    fn plain_print(&self) {
        println!(
            "{} {} {} {}{}",
            format!("#{}", self.floor).yellow(),
            self.author.green(),
            t!("uid_label", id = &self.author_id).to_string().dimmed(),
            format_relative_time(self.post_date).dimmed(),
            if self.score != 0 {
                format!(" {}", t!("score_label", score = self.score))
                    .dimmed()
                    .to_string()
            } else {
                String::new()
            }
        );
        for line in self.content.lines() {
            if !line.trim().is_empty() {
                println!("     {}", line);
            }
        }
        println!();
    }
}

/// Topic list result.
#[derive(Debug, Clone, Serialize)]
pub struct TopicListResult {
    pub forum_name: Option<String>,
    pub start_page: u32,
    pub end_page: u32,
    pub total_pages: u32,
    pub topics: Vec<TopicInfo>,
}

/// Topic details result.
#[derive(Debug, Clone, Serialize)]
pub struct TopicDetailsResult {
    pub forum_name: String,
    pub subject: String,
    pub tags: Vec<String>,
    pub author: String,
    pub author_id: String,
    pub replies: i32,
    pub post_date: i64,
    pub page: u32,
    pub total_pages: u32,
    pub posts: Vec<PostInfo>,
}

/// Topic search result.
#[derive(Debug, Clone, Serialize)]
pub struct TopicSearchResult {
    pub keyword: String,
    pub page: u32,
    pub total_pages: u32,
    pub topics: Vec<TopicInfo>,
}

/// Folder information.
#[derive(Debug, Clone, Serialize)]
pub struct FolderInfo {
    pub id: String,
    pub name: String,
    pub count: i32,
}

impl From<&FavoriteFolder> for FolderInfo {
    fn from(f: &FavoriteFolder) -> Self {
        Self {
            id: f.id.clone(),
            name: f.name.clone(),
            count: f.count,
        }
    }
}

impl TableRow for FolderInfo {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Name", "Topics"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.id.clone(), self.name.clone(), self.count.to_string()]
    }
}

impl PlainPrint for FolderInfo {
    fn plain_print(&self) {
        println!(
            "[{}] {} {}",
            self.id.cyan(),
            self.name.bold(),
            t!("topics_count", count = self.count).to_string().dimmed()
        );
    }
}

/// Favorite topics result.
#[derive(Debug, Clone, Serialize)]
pub struct FavoriteTopicsResult {
    pub folder: Option<String>,
    pub page: u32,
    pub total_pages: u32,
    pub topics: Vec<TopicInfo>,
}

/// Favorite modification result.
#[derive(Debug, Clone, Serialize)]
pub struct FavoriteModifyResult {
    pub topic_id: String,
    pub action: String,
}

/// Recent post information.
#[derive(Debug, Clone, Serialize)]
pub struct RecentPostInfo {
    pub topic_id: String,
    pub topic_subject: String,
    #[serde(rename = "type")]
    pub post_type: String,
    pub post_id: String,
    pub floor: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub post_date: i64,
    pub score: i32,
}

/// Recent topics/posts result.
#[derive(Debug, Clone, Serialize)]
pub struct RecentResult {
    pub forum_name: Option<String>,
    pub range_display: String,
    pub topics: Vec<TopicInfo>,
    pub posts: Vec<RecentPostInfo>,
}

/// Options for listing topics.
#[derive(Debug, Clone, Default)]
pub struct ListTopicsOptions {
    pub is_stid: bool,
    pub start_page: u32,
    pub num_pages: u32,
    pub order: String,
    pub concurrency: usize,
}

/// Options for reading topic.
#[derive(Debug, Clone, Default)]
pub struct ReadTopicOptions {
    pub page: u32,
    pub author: Option<String>,
    pub fetch_all: bool,
    pub concurrency: usize,
}

/// Options for searching topics.
#[derive(Debug, Clone, Default)]
pub struct SearchTopicsOptions {
    pub is_stid: bool,
    pub page: u32,
    pub search_content: bool,
}

/// Options for recent topics.
#[derive(Debug, Clone, Default)]
pub struct RecentTopicsOptions {
    pub is_stid: bool,
    pub range: String,
    pub page: u32,
    pub order: String,
    pub with_posts: bool,
    pub concurrency: usize,
}

fn parse_order(order: &str) -> TopicOrder {
    match order {
        "postdate" => TopicOrder::PostDate,
        "recommend" => TopicOrder::Recommend,
        _ => TopicOrder::LastPost,
    }
}

fn effective_concurrency(requested: usize) -> usize {
    let max_concurrency = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(8);
    requested.min(max_concurrency).max(1)
}

/// Parse time range string to seconds and display string.
pub fn parse_time_range(range: &str) -> Option<(i64, String)> {
    let range_lower = range.to_lowercase();

    if range_lower.len() >= 2 {
        let (num_str, unit) = range_lower.split_at(range_lower.len() - 1);
        if let Ok(num) = num_str.parse::<i64>() {
            match unit {
                "s" => {
                    let unit_str = if num != 1 {
                        t!("seconds")
                    } else {
                        t!("second")
                    };
                    return Some((num, format!("{} {}", num, unit_str)));
                }
                "m" => {
                    let unit_str = if num != 1 {
                        t!("minutes")
                    } else {
                        t!("minute")
                    };
                    return Some((num * 60, format!("{} {}", num, unit_str)));
                }
                "h" => {
                    let unit_str = if num != 1 { t!("hours") } else { t!("hour") };
                    return Some((num * 3600, format!("{} {}", num, unit_str)));
                }
                "d" => {
                    let unit_str = if num != 1 { t!("days") } else { t!("day") };
                    return Some((num * 86400, format!("{} {}", num, unit_str)));
                }
                _ => {}
            }
        }
    }

    match range_lower.as_str() {
        "second" | "1s" => Some((1, t!("second").to_string())),
        "minute" | "1m" => Some((60, t!("minute").to_string())),
        "hour" | "1h" => Some((3600, t!("hour").to_string())),
        "day" | "1d" => Some((86400, t!("day").to_string())),
        "week" => Some((604800, t!("week").to_string())),
        "month" => Some((2592000, t!("month").to_string())),
        "year" => Some((31536000, t!("year").to_string())),
        _ => None,
    }
}

/// List topics in a forum with optional multi-page concurrent fetching.
pub async fn list_topics(
    client: &NGAClient,
    forum_id: &str,
    options: ListTopicsOptions,
) -> Result<TopicListResult> {
    let id = if options.is_stid {
        ForumIdKind::stid(forum_id)
    } else {
        ForumIdKind::fid(forum_id)
    };

    let order_by = parse_order(&options.order);
    let start_page = options.start_page.max(1);

    let first_result = client
        .topics()
        .list(id.clone())
        .page(start_page)
        .order(order_by)
        .send()
        .await?;

    let total_pages = first_result.total_pages;
    let actual_pages = options
        .num_pages
        .min(total_pages.saturating_sub(start_page - 1));
    let forum_name = first_result.forum.as_ref().map(|f| f.name.clone());

    if actual_pages <= 1 {
        return Ok(TopicListResult {
            forum_name,
            start_page,
            end_page: start_page,
            total_pages,
            topics: first_result.topics.iter().map(TopicInfo::from).collect(),
        });
    }

    let concurrency = effective_concurrency(options.concurrency);
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let client = Arc::new(client.clone());

    let pages_to_fetch: Vec<u32> = ((start_page + 1)..=(start_page + actual_pages - 1)).collect();

    let fetch_results: Vec<_> = stream::iter(pages_to_fetch)
        .map(|p| {
            let sem = semaphore.clone();
            let client = client.clone();
            let id = id.clone();
            async move {
                let _permit = sem.acquire().await.unwrap();
                (
                    p,
                    client
                        .topics()
                        .list(id)
                        .page(p)
                        .order(order_by)
                        .send()
                        .await,
                )
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let mut all_topics = first_result.topics;

    let mut sorted_results: Vec<_> = fetch_results.into_iter().collect();
    sorted_results.sort_by_key(|(p, _)| *p);

    for (_page_num, result) in sorted_results {
        if let Ok(page_result) = result {
            all_topics.extend(page_result.topics);
        }
    }

    Ok(TopicListResult {
        forum_name,
        start_page,
        end_page: start_page + actual_pages - 1,
        total_pages,
        topics: all_topics.iter().map(TopicInfo::from).collect(),
    })
}

/// Read topic details with optional all-pages concurrent fetching.
pub async fn read_topic(
    client: &NGAClient,
    topic_id: &str,
    options: ReadTopicOptions,
) -> Result<TopicDetailsResult> {
    let page = options.page.max(1);

    let mut builder = client.topics().details(topic_id).page(page);
    if let Some(ref author_id) = options.author {
        builder = builder.author(author_id.clone());
    }

    let first_result = builder.send().await?;

    let topic = &first_result.topic;
    let forum_name = first_result.forum_name.clone();
    let subject = topic.subject.content.clone();
    let tags = topic.subject.tags.clone();
    let author = topic.author.name.display().to_string();
    let author_id = topic.author.id.to_string();
    let replies = topic.replies;
    let post_date = topic.post_date;
    let total_pages = first_result.total_pages;

    if !options.fetch_all || first_result.total_pages <= 1 {
        return Ok(TopicDetailsResult {
            forum_name,
            subject,
            tags,
            author,
            author_id,
            replies,
            post_date,
            page,
            total_pages,
            posts: first_result.posts.iter().map(PostInfo::from).collect(),
        });
    }

    let concurrency = effective_concurrency(options.concurrency);
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let client = Arc::new(client.clone());
    let author_clone = options.author.clone();

    let pages_to_fetch: Vec<u32> = (2..=first_result.total_pages).collect();

    let fetch_results: Vec<_> = stream::iter(pages_to_fetch)
        .map(|p| {
            let sem = semaphore.clone();
            let client = client.clone();
            let author_id = author_clone.clone();
            let tid = topic_id.to_string();
            async move {
                let _permit = sem.acquire().await.unwrap();
                let mut builder = client.topics().details(&tid).page(p);
                if let Some(ref aid) = author_id {
                    builder = builder.author(aid.clone());
                }
                (p, builder.send().await)
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let mut all_posts = first_result.posts;

    let mut sorted_results: Vec<_> = fetch_results.into_iter().collect();
    sorted_results.sort_by_key(|(p, _)| *p);

    for (_page_num, result) in sorted_results {
        if let Ok(page_result) = result {
            all_posts.extend(page_result.posts);
        }
    }

    all_posts.sort_by_key(|p| p.floor);

    Ok(TopicDetailsResult {
        forum_name,
        subject,
        tags,
        author,
        author_id,
        replies,
        post_date,
        page: 1,
        total_pages,
        posts: all_posts.iter().map(PostInfo::from).collect(),
    })
}

/// Search topics in a forum.
pub async fn search_topics(
    client: &NGAClient,
    forum_id: &str,
    keyword: &str,
    options: SearchTopicsOptions,
) -> Result<TopicSearchResult> {
    let id = if options.is_stid {
        ForumIdKind::stid(forum_id)
    } else {
        ForumIdKind::fid(forum_id)
    };

    let result = client
        .topics()
        .search(id, keyword)
        .page(options.page)
        .search_content(options.search_content)
        .send()
        .await?;

    Ok(TopicSearchResult {
        keyword: keyword.to_string(),
        page: options.page,
        total_pages: result.total_pages,
        topics: result.topics.iter().map(TopicInfo::from).collect(),
    })
}

/// List favorite folders.
pub async fn list_folders(client: &NGAClient) -> Result<Vec<FolderInfo>> {
    let folders = client.topics().favorite_folders().await?;
    Ok(folders.iter().map(FolderInfo::from).collect())
}

/// List favorite topics.
pub async fn list_favorites(
    client: &NGAClient,
    folder: Option<&str>,
    page: u32,
) -> Result<FavoriteTopicsResult> {
    let mut builder = client.topics().favorites().page(page);
    if let Some(folder_id) = folder {
        builder = builder.folder(folder_id.to_string());
    }

    let result = builder.send().await?;

    Ok(FavoriteTopicsResult {
        folder: folder.map(|s| s.to_string()),
        page,
        total_pages: result.total_pages,
        topics: result.topics.iter().map(TopicInfo::from).collect(),
    })
}

/// Add topic to favorites.
pub async fn add_favorite(
    client: &NGAClient,
    topic_id: &str,
    folder: Option<&str>,
) -> Result<FavoriteModifyResult> {
    let folder_id = folder.unwrap_or("");
    client
        .topics()
        .modify_favorite(topic_id, folder_id, FavoriteTopicOp::Add)
        .await?;

    Ok(FavoriteModifyResult {
        topic_id: topic_id.to_string(),
        action: "added".to_string(),
    })
}

/// Remove topic from favorites.
pub async fn remove_favorite(
    client: &NGAClient,
    topic_id: &str,
    folder: Option<&str>,
) -> Result<FavoriteModifyResult> {
    let folder_id = folder.unwrap_or("");
    client
        .topics()
        .modify_favorite(topic_id, folder_id, FavoriteTopicOp::Remove)
        .await?;

    Ok(FavoriteModifyResult {
        topic_id: topic_id.to_string(),
        action: "removed".to_string(),
    })
}

/// Get recent topics/posts in a forum.
pub async fn recent_topics(
    client: &NGAClient,
    forum_id: &str,
    options: RecentTopicsOptions,
) -> Result<RecentResult> {
    use chrono::Local;

    let now = Local::now().timestamp();

    let (time_range_seconds, range_display) =
        parse_time_range(&options.range).unwrap_or((3600, "hour".to_string()));

    let cutoff_time = now - time_range_seconds;

    let id = if options.is_stid {
        ForumIdKind::stid(forum_id)
    } else {
        ForumIdKind::fid(forum_id)
    };

    let order_by = parse_order(&options.order);

    let result = client
        .topics()
        .list(id)
        .page(options.page)
        .order(order_by)
        .send()
        .await?;

    let forum_name = result.forum.as_ref().map(|f| f.name.clone());

    let recent_topics_raw: Vec<Topic> = result
        .topics
        .into_iter()
        .filter(|t| {
            let relevant_time = match order_by {
                TopicOrder::PostDate => t.post_date,
                _ => t.last_post_date,
            };
            relevant_time >= cutoff_time
        })
        .collect();

    if !options.with_posts {
        return Ok(RecentResult {
            forum_name,
            range_display,
            topics: recent_topics_raw.iter().map(TopicInfo::from).collect(),
            posts: Vec::new(),
        });
    }

    if recent_topics_raw.is_empty() {
        return Ok(RecentResult {
            forum_name,
            range_display,
            topics: Vec::new(),
            posts: Vec::new(),
        });
    }

    let concurrency = effective_concurrency(options.concurrency);
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let client = Arc::new(client.clone());

    let fetch_results: Vec<_> = stream::iter(recent_topics_raw.iter().cloned())
        .map(|topic| {
            let sem = semaphore.clone();
            let client = client.clone();
            async move {
                let _permit = sem.acquire().await.unwrap();
                fetch_topic_posts(&client, &topic, cutoff_time).await
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let mut all_posts: Vec<RecentPostInfo> = Vec::new();
    for (topic, result) in recent_topics_raw.iter().zip(fetch_results.into_iter()) {
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

    Ok(RecentResult {
        forum_name,
        range_display,
        topics: recent_topics_raw.iter().map(TopicInfo::from).collect(),
        posts: all_posts,
    })
}

async fn fetch_topic_posts(
    client: &NGAClient,
    topic: &Topic,
    cutoff_time: i64,
) -> Result<Vec<(String, String, String, String, String, String, i64, i32)>> {
    let mut results = Vec::new();
    let mut posts_to_check_comments: Vec<Post> = Vec::new();

    let details = client
        .topics()
        .details(topic.id.clone())
        .page(1)
        .send()
        .await?;

    for post in details.posts {
        if post.comment_count > 0 {
            posts_to_check_comments.push(post.clone());
        }
        if post.post_date >= cutoff_time {
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

    if details.total_pages > 1 {
        if let Ok(last_page_details) = client
            .topics()
            .details(topic.id.clone())
            .page(details.total_pages)
            .send()
            .await
        {
            for post in last_page_details.posts {
                if post.comment_count > 0 {
                    posts_to_check_comments.push(post.clone());
                }
                if post.post_date >= cutoff_time {
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
        }
    }

    for post in posts_to_check_comments {
        if let Ok(comments_result) = client.posts().comments(&topic.id, &post.id, 1).await {
            let last_page = comments_result.total_pages;

            let comments_to_check = if last_page > 1 {
                if let Ok(last_comments) = client
                    .posts()
                    .comments(&topic.id, &post.id, last_page)
                    .await
                {
                    last_comments.comments
                } else {
                    comments_result.comments
                }
            } else {
                comments_result.comments
            };

            for comment in comments_to_check {
                if comment.post_date >= cutoff_time {
                    results.push((
                        "comment".to_string(),
                        post.id.to_string(),
                        format!("#{} comment", post.floor),
                        comment.author.name.display().to_string(),
                        comment.author.id.to_string(),
                        comment.content.to_plain_text(),
                        comment.post_date,
                        comment.score,
                    ));
                }
            }
        }
    }

    Ok(results)
}
