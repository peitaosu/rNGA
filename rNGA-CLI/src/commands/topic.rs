//! Topic commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use rnga::models::*;

use crate::config::{build_authed_client, build_client};
use crate::output::{format_relative_time, format_time, print_table, FolderRow, OutputFormat, PostRow, TopicRow};

#[derive(Subcommand)]
pub enum TopicAction {
    /// List topics in a forum
    #[command(alias = "ls")]
    List {
        /// Forum ID
        forum_id: String,
        /// Treat ID as stid instead of fid
        #[arg(short, long)]
        stid: bool,
        /// Starting page number
        #[arg(short, long, default_value = "1")]
        page: u32,
        /// Sort order: lastpost, postdate, recommend
        #[arg(short, long, default_value = "lastpost")]
        order: String,
        /// Number of pages to fetch
        #[arg(short = 'n', long, default_value = "1")]
        pages: u32,
        /// Number of concurrent requests
        #[arg(short = 'j', long, default_value = "4")]
        concurrency: usize,
    },

    /// View topic details and posts
    #[command(alias = "view")]
    Read {
        /// Topic ID
        topic_id: String,
        /// Starting page number
        #[arg(short, long, default_value = "1")]
        page: u32,
        /// Filter by author ID
        #[arg(short, long)]
        author: Option<String>,
        /// Fetch all pages
        #[arg(long)]
        all: bool,
        /// Number of concurrent requests
        #[arg(short = 'j', long, default_value = "4")]
        concurrency: usize,
    },

    /// Search topics in a forum
    Search {
        /// Forum ID
        forum_id: String,
        /// Search keyword
        keyword: String,
        /// Treat ID as stid instead of fid
        #[arg(short, long)]
        stid: bool,
        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,
        /// Search in content
        #[arg(short, long)]
        content: bool,
    },

    /// List favorite topic folders
    Folders,

    /// List favorite topics
    Favorites {
        /// Folder ID
        #[arg(short, long)]
        folder: Option<String>,
        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,
    },

    /// Add topic to favorites
    FavAdd {
        /// Topic ID
        topic_id: String,
        /// Folder ID
        #[arg(short, long)]
        folder: Option<String>,
    },

    /// Remove topic from favorites
    FavRemove {
        /// Topic ID
        topic_id: String,
        /// Folder ID
        #[arg(short, long)]
        folder: Option<String>,
    },

    /// List recent topics/posts in a forum
    Recent {
        /// Forum ID
        forum_id: String,
        /// Treat ID as stid instead of fid
        #[arg(short, long)]
        stid: bool,
        /// Time range: second, minute, hour, day, week, month, year
        #[arg(short, long, default_value = "1h")]
        range: String,
        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,
        /// Sort order: lastpost, postdate, recommend
        #[arg(short, long, default_value = "lastpost")]
        order: String,
        /// Show individual posts/replies
        #[arg(long)]
        with_posts: bool,
        /// Number of concurrent requests
        #[arg(short = 'j', long, default_value = "4")]
        concurrency: usize,
    },
}

pub async fn handle(action: TopicAction, format: OutputFormat, verbose: bool) -> Result<()> {
    match action {
        TopicAction::List {
            forum_id,
            stid,
            page,
            order,
            pages,
            concurrency,
        } => list_topics(&forum_id, stid, page, pages, &order, concurrency, format).await,
        TopicAction::Read {
            topic_id,
            page,
            author,
            all,
            concurrency,
        } => read_topic(&topic_id, page, author, all, concurrency, format, verbose).await,
        TopicAction::Search {
            forum_id,
            keyword,
            stid,
            page,
            content,
        } => search_topics(&forum_id, stid, &keyword, page, content, format).await,
        TopicAction::Folders => list_folders(format).await,
        TopicAction::Favorites { folder, page } => list_favorites(folder, page, format).await,
        TopicAction::FavAdd { topic_id, folder } => add_favorite(&topic_id, folder).await,
        TopicAction::FavRemove { topic_id, folder } => remove_favorite(&topic_id, folder).await,
        TopicAction::Recent {
            forum_id,
            stid,
            range,
            page,
            order,
            with_posts,
            concurrency,
        } => recent_topics(&forum_id, stid, &range, page, &order, with_posts, concurrency, format).await,
    }
}

async fn list_topics(
    forum_id: &str,
    is_stid: bool,
    start_page: u32,
    num_pages: u32,
    order: &str,
    concurrency: usize,
    format: OutputFormat,
) -> Result<()> {
    use futures::stream::{self, StreamExt};
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    
    let client = build_client()?;

    let id = if is_stid {
        ForumIdKind::stid(forum_id)
    } else {
        ForumIdKind::fid(forum_id)
    };

    let order_by = match order {
        "postdate" => TopicOrder::PostDate,
        "recommend" => TopicOrder::Recommend,
        _ => TopicOrder::LastPost,
    };

    let first_result = client.topics().list(id.clone()).page(start_page).order(order_by).send().await?;
    
    let total_pages = first_result.total_pages;
    let actual_pages = num_pages.min(total_pages.saturating_sub(start_page - 1));
    
    if matches!(format, OutputFormat::Plain) {
        if let Some(forum) = &first_result.forum {
            if actual_pages > 1 {
                println!("{} (pages {}-{}/{})", 
                    forum.name.bold(), 
                    start_page, 
                    start_page + actual_pages - 1,
                    total_pages
                );
            } else {
                println!("{} (page {}/{})", forum.name.bold(), start_page, total_pages);
            }
            println!();
        }
    }

    if actual_pages <= 1 {
        let rows: Vec<TopicRow> = first_result.topics.iter().map(TopicRow::from).collect();
        print_table(rows, format);
        return Ok(());
    }

    let max_concurrency = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(8);
    let effective_concurrency = concurrency.min(max_concurrency).max(1);
    
    let semaphore = Arc::new(Semaphore::new(effective_concurrency));
    let client = Arc::new(client);
    
    let pages_to_fetch: Vec<u32> = ((start_page + 1)..=(start_page + actual_pages - 1)).collect();
    
    let fetch_results: Vec<_> = stream::iter(pages_to_fetch)
        .map(|p| {
            let sem = semaphore.clone();
            let client = client.clone();
            let id = id.clone();
            async move {
                let _permit = sem.acquire().await.unwrap();
                (p, client.topics().list(id).page(p).order(order_by).send().await)
            }
        })
        .buffer_unordered(effective_concurrency)
        .collect()
        .await;
    
    let mut all_topics = first_result.topics;
    
    let mut sorted_results: Vec<_> = fetch_results.into_iter().collect();
    sorted_results.sort_by_key(|(p, _)| *p);
    
    for (page_num, result) in sorted_results {
        match result {
            Ok(page_result) => {
                all_topics.extend(page_result.topics);
            }
            Err(e) => {
                eprintln!("Warning: Failed to fetch page {}: {}", page_num, e);
            }
        }
    }

    let rows: Vec<TopicRow> = all_topics.iter().map(TopicRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn read_topic(
    topic_id: &str,
    page: u32,
    author: Option<String>,
    fetch_all: bool,
    concurrency: usize,
    format: OutputFormat,
    _verbose: bool,
) -> Result<()> {
    use futures::stream::{self, StreamExt};
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    
    let client = build_client()?;

    let mut builder = client.topics().details(topic_id).page(page);
    if let Some(ref author_id) = author {
        builder = builder.author(author_id.clone());
    }

    let first_result = builder.send().await?;

    let topic = &first_result.topic;
    if matches!(format, OutputFormat::Plain) {
        println!(
            "{} {}",
            format!("[{}]", first_result.forum_name).dimmed(),
            topic.subject.content.bold()
        );
        if !topic.subject.tags.is_empty() {
            println!("Tags: {}", topic.subject.tags.join(", ").cyan());
        }
        println!(
            "By {} | {} | {} replies",
            topic.author.name.display().green(),
            format_time(topic.post_date),
            topic.replies
        );
    }
    
    if fetch_all && first_result.total_pages > 1 {
        let max_concurrency = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(8);
        let effective_concurrency = concurrency.min(max_concurrency).max(1);
        
        if matches!(format, OutputFormat::Plain) {
            println!(
                "Fetching all {} pages (concurrency: {})...\n",
                first_result.total_pages, effective_concurrency
            );
        }
        
        let semaphore = Arc::new(Semaphore::new(effective_concurrency));
        let client = Arc::new(client);
        let author_clone = author.clone();
        
        let pages_to_fetch: Vec<u32> = (2..=first_result.total_pages).collect();
        
        let fetch_results: Vec<_> = stream::iter(pages_to_fetch)
            .map(|p| {
                let sem = semaphore.clone();
                let client = client.clone();
                let author_id = author_clone.clone();
                async move {
                    let _permit = sem.acquire().await.unwrap();
                    let mut builder = client.topics().details(topic_id).page(p);
                    if let Some(ref aid) = author_id {
                        builder = builder.author(aid.clone());
                    }
                    (p, builder.send().await)
                }
            })
            .buffer_unordered(effective_concurrency)
            .collect()
            .await;
        
        let mut all_posts = first_result.posts.clone();
        
        let mut sorted_results: Vec<_> = fetch_results.into_iter().collect();
        sorted_results.sort_by_key(|(p, _)| *p);
        
        for (page_num, result) in sorted_results {
            match result {
                Ok(page_result) => {
                    all_posts.extend(page_result.posts);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to fetch page {}: {}", page_num, e);
                }
            }
        }
        
        all_posts.sort_by_key(|p| p.floor);
        
        let rows: Vec<PostRow> = all_posts.iter().map(PostRow::from).collect();
        print_table(rows, format);
    } else {
        if matches!(format, OutputFormat::Plain) {
            println!(
                "Page {}/{}\n",
                page, first_result.total_pages
            );
        }
        
        let rows: Vec<PostRow> = first_result.posts.iter().map(PostRow::from).collect();
        print_table(rows, format);
    }

    Ok(())
}

async fn search_topics(
    forum_id: &str,
    is_stid: bool,
    keyword: &str,
    page: u32,
    search_content: bool,
    format: OutputFormat,
) -> Result<()> {
    let client = build_client()?;

    let id = if is_stid {
        ForumIdKind::stid(forum_id)
    } else {
        ForumIdKind::fid(forum_id)
    };

    let result = client
        .topics()
        .search(id, keyword)
        .page(page)
        .search_content(search_content)
        .send()
        .await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "Search results for '{}' (page {}/{})\n",
            keyword.cyan(),
            page,
            result.total_pages
        );
    }

    let rows: Vec<TopicRow> = result.topics.iter().map(TopicRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn list_folders(format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let folders = client.topics().favorite_folders().await?;

    let rows: Vec<FolderRow> = folders.iter().map(FolderRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn list_favorites(folder: Option<String>, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;

    let mut builder = client.topics().favorites().page(page);
    if let Some(folder_id) = folder {
        builder = builder.folder(folder_id);
    }

    let result = builder.send().await?;

    if matches!(format, OutputFormat::Plain) {
        println!("Favorite topics (page {}/{})\n", page, result.total_pages);
    }

    let rows: Vec<TopicRow> = result.topics.iter().map(TopicRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn add_favorite(topic_id: &str, folder: Option<String>) -> Result<()> {
    let client = build_authed_client()?;
    let folder_id = folder.unwrap_or_default();

    client
        .topics()
        .modify_favorite(topic_id, &folder_id, FavoriteTopicOp::Add)
        .await?;

    println!("Added topic {} to favorites", topic_id);
    Ok(())
}

async fn remove_favorite(topic_id: &str, folder: Option<String>) -> Result<()> {
    let client = build_authed_client()?;
    let folder_id = folder.unwrap_or_default();

    client
        .topics()
        .modify_favorite(topic_id, &folder_id, FavoriteTopicOp::Remove)
        .await?;

    println!("Removed topic {} from favorites", topic_id);
    Ok(())
}

fn parse_time_range(range: &str) -> Option<(i64, String)> {
    let range_lower = range.to_lowercase();
    
    if range_lower.len() >= 2 {
        let (num_str, unit) = range_lower.split_at(range_lower.len() - 1);
        if let Ok(num) = num_str.parse::<i64>() {
            match unit {
                "s" => return Some((num, format!("{} second{}", num, if num != 1 { "s" } else { "" }))),
                "m" => return Some((num * 60, format!("{} minute{}", num, if num != 1 { "s" } else { "" }))),
                "h" => return Some((num * 3600, format!("{} hour{}", num, if num != 1 { "s" } else { "" }))),
                "d" => return Some((num * 86400, format!("{} day{}", num, if num != 1 { "s" } else { "" }))),
                _ => {}
            }
        }
    }
    
    match range_lower.as_str() {
        "second" | "1s" => Some((1, "second".to_string())),
        "minute" | "1m" => Some((60, "minute".to_string())),
        "hour" | "1h" => Some((3600, "hour".to_string())),
        "day" | "1d" => Some((86400, "day".to_string())),
        "week" => Some((604800, "week".to_string())),
        "month" => Some((2592000, "month".to_string())),
        "year" => Some((31536000, "year".to_string())),
        _ => None,
    }
}

async fn recent_topics(
    forum_id: &str,
    is_stid: bool,
    range: &str,
    page: u32,
    order: &str,
    with_posts: bool,
    concurrency: usize,
    format: OutputFormat,
) -> Result<()> {
    use chrono::Local;
    use futures::stream::{self, StreamExt};
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    
    let now = Local::now().timestamp();
    
    let (time_range_seconds, range_display) = parse_time_range(range)
        .unwrap_or_else(|| {
            eprintln!("Invalid time range '{}', using default (1 hour)", range);
            (3600, "hour".to_string())
        });
    
    let cutoff_time = now - time_range_seconds;
    
    let client = build_client()?;

    let id = if is_stid {
        ForumIdKind::stid(forum_id)
    } else {
        ForumIdKind::fid(forum_id)
    };

    let order_by = match order {
        "postdate" => TopicOrder::PostDate,
        "recommend" => TopicOrder::Recommend,
        _ => TopicOrder::LastPost,
    };

    let result = client.topics().list(id).page(page).order(order_by).send().await?;

    let recent_topics: Vec<Topic> = result
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

    if matches!(format, OutputFormat::Plain) {
        if let Some(forum) = &result.forum {
            let content_type = if with_posts { "posts" } else { "topics" };
            println!(
                "{} - Recent {} in the last {}",
                forum.name.bold(),
                content_type,
                range_display.cyan()
            );
            println!();
        }
    }

    if recent_topics.is_empty() {
        if matches!(format, OutputFormat::Plain) {
            println!("No topics found in the specified time range");
        }
        return Ok(());
    }

    if !with_posts {
        let rows: Vec<TopicRow> = recent_topics.iter().map(TopicRow::from).collect();
        print_table(rows, format);
        return Ok(());
    }

    #[derive(Clone)]
    enum RecentPost {
        Post(Post),
        Comment {
            post_id: PostId,
            post_floor: i32,
            comment: LightPost,
        },
    }
    
    impl RecentPost {
        fn post_date(&self) -> i64 {
            match self {
                RecentPost::Post(p) => p.post_date,
                RecentPost::Comment { comment, .. } => comment.post_date,
            }
        }
    }
    
    let total_topics = recent_topics.len();
    
    let max_concurrency = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(8);
    let effective_concurrency = concurrency.min(max_concurrency).max(1);
    
    if matches!(format, OutputFormat::Plain) {
        println!("Fetching posts from {} recent topic{} (concurrency: {})...\n", 
            total_topics, 
            if total_topics != 1 { "s" } else { "" },
            effective_concurrency
        );
    }

    let semaphore = Arc::new(Semaphore::new(effective_concurrency));
    let client = Arc::new(client);
    
    let fetch_results: Vec<_> = stream::iter(recent_topics.iter().cloned().enumerate())
        .map(|(idx, topic)| {
            let sem = semaphore.clone();
            let client = client.clone();
            async move {
                let _permit = sem.acquire().await.unwrap();
                
                if matches!(format, OutputFormat::Plain) {
                    eprint!("\rScanning topic {}/{}...", idx + 1, total_topics);
                }
                
                fetch_topic_posts(&client, &topic, cutoff_time).await
            }
        })
        .buffer_unordered(effective_concurrency)
        .collect()
        .await;
    
    let mut all_recent_posts: Vec<(Topic, RecentPost)> = Vec::new();
    for (topic, result) in recent_topics.iter().zip(fetch_results.into_iter()) {
        match result {
            Ok(posts) => {
                for post in posts {
                    all_recent_posts.push((topic.clone(), post));
                }
            }
            Err(e) => {
                if matches!(format, OutputFormat::Plain) {
                    eprintln!("\nWarning: Could not fetch posts for topic {}: {}", topic.id, e);
                }
            }
        }
    }
    
    if matches!(format, OutputFormat::Plain) {
        eprintln!("\r{}", " ".repeat(50));
        eprint!("\r");
    }
    
    async fn fetch_topic_posts(
        client: &rnga::NGAClient,
        topic: &Topic,
        cutoff_time: i64,
    ) -> Result<Vec<RecentPost>> {
        let mut results = Vec::new();
        let mut posts_to_check_comments: Vec<Post> = Vec::new();
        
        let details = client.topics().details(topic.id.clone()).page(1).send().await?;
        
        for post in details.posts {
            if post.comment_count > 0 {
                posts_to_check_comments.push(post.clone());
            }
            if post.post_date >= cutoff_time {
                results.push(RecentPost::Post(post));
            }
        }
        
        if details.total_pages > 1 {
            if let Ok(last_page_details) = client.topics().details(topic.id.clone()).page(details.total_pages).send().await {
                for post in last_page_details.posts {
                    if post.comment_count > 0 {
                        posts_to_check_comments.push(post.clone());
                    }
                    if post.post_date >= cutoff_time {
                        results.push(RecentPost::Post(post));
                    }
                }
            }
        }
        
        for post in posts_to_check_comments {
            if let Ok(comments_result) = client.posts().comments(&topic.id, &post.id, 1).await {
                let last_page = comments_result.total_pages;
                
                let comments_to_check = if last_page > 1 {
                    if let Ok(last_comments) = client.posts().comments(&topic.id, &post.id, last_page).await {
                        last_comments.comments
                    } else {
                        comments_result.comments
                    }
                } else {
                    comments_result.comments
                };
                
                for comment in comments_to_check {
                    if comment.post_date >= cutoff_time {
                        results.push(RecentPost::Comment {
                            post_id: post.id.clone(),
                            post_floor: post.floor,
                            comment,
                        });
                    }
                }
            }
        }
        
        Ok(results)
    }

    if all_recent_posts.is_empty() {
        if matches!(format, OutputFormat::Plain) {
            println!("No posts found in the specified time range");
        }
        return Ok(());
    }

    all_recent_posts.sort_by(|a, b| b.1.post_date().cmp(&a.1.post_date()));

    if matches!(format, OutputFormat::Plain) {
        println!("Found {} recent post{}\n", 
            all_recent_posts.len(),
            if all_recent_posts.len() != 1 { "s" } else { "" }
        );
    }

    match format {
        OutputFormat::Json => {
            #[derive(serde::Serialize)]
            struct RecentPostJsonOutput {
                topic_id: String,
                topic_subject: String,
                #[serde(rename = "type")]
                post_type: String,
                post_id: String,
                floor: String,
                author_id: String,
                author_name: String,
                content: String,
                post_date: i64,
                score: i32,
            }
            
            let output: Vec<RecentPostJsonOutput> = all_recent_posts.iter().map(|(topic, recent_post)| {
                match recent_post {
                    RecentPost::Post(post) => RecentPostJsonOutput {
                        topic_id: topic.id.to_string(),
                        topic_subject: topic.subject.content.clone(),
                        post_type: "post".to_string(),
                        post_id: post.id.to_string(),
                        floor: format!("#{}", post.floor),
                        author_id: post.author.id.to_string(),
                        author_name: post.author.name.display().to_string(),
                        content: post.content.to_plain_text(),
                        post_date: post.post_date,
                        score: post.score,
                    },
                    RecentPost::Comment { post_id, post_floor, comment } => RecentPostJsonOutput {
                        topic_id: topic.id.to_string(),
                        topic_subject: topic.subject.content.clone(),
                        post_type: "comment".to_string(),
                        post_id: post_id.to_string(),
                        floor: format!("#{} comment", post_floor),
                        author_id: comment.author.id.to_string(),
                        author_name: comment.author.name.display().to_string(),
                        content: comment.content.to_plain_text(),
                        post_date: comment.post_date,
                        score: comment.score,
                    },
                }
            }).collect();
            
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Table | OutputFormat::Plain => {
            use std::collections::HashMap;
            
            let mut posts_by_topic: HashMap<String, Vec<&RecentPost>> = HashMap::new();
            for (topic, recent_post) in &all_recent_posts {
                posts_by_topic
                    .entry(topic.id.to_string())
                    .or_insert_with(Vec::new)
                    .push(recent_post);
            }
            
            let mut topic_order: Vec<(String, i64)> = posts_by_topic
                .iter()
                .map(|(topic_id, posts)| {
                    let max_time = posts.iter().map(|p| p.post_date()).max().unwrap_or(0);
                    (topic_id.clone(), max_time)
                })
                .collect();
            topic_order.sort_by(|a, b| b.1.cmp(&a.1));
            
            for (topic_id, _) in topic_order {
                let (topic, _) = all_recent_posts
                    .iter()
                    .find(|(t, _)| t.id.to_string() == topic_id)
                    .unwrap();
                
                println!(
                    "{} {}",
                    format!("[Topic {}]", topic.id).cyan(),
                    topic.subject.content.bold()
                );
                
                let mut topic_posts = posts_by_topic[&topic_id].clone();
                topic_posts.sort_by(|a, b| {
                    let floor_a = match a {
                        RecentPost::Post(p) => (p.floor, 0),
                        RecentPost::Comment { post_floor, .. } => (*post_floor, 1),
                    };
                    let floor_b = match b {
                        RecentPost::Post(p) => (p.floor, 0),
                        RecentPost::Comment { post_floor, .. } => (*post_floor, 1),
                    };
                    floor_a.cmp(&floor_b)
                });
                
                for recent_post in topic_posts {
                    match recent_post {
                        RecentPost::Post(post) => {
                            let uid_display = format!("[UID: {}]", post.author.id);
                            println!(
                                "   {} {} {} {}{}",
                                format!("#{}", post.floor).yellow(),
                                post.author.name.display().green(),
                                uid_display.dimmed(),
                                format_relative_time(post.post_date).dimmed(),
                                if post.score != 0 { 
                                    format!(" (score: {})", post.score).dimmed().to_string() 
                                } else { 
                                    String::new() 
                                }
                            );
                            
                            let content = post.content.to_plain_text();
                            let preview = if content.len() > 200 {
                                format!("{}...", content.chars().take(200).collect::<String>())
                            } else {
                                content
                            };
                            
                            for line in preview.lines() {
                                if !line.trim().is_empty() {
                                    println!("        {}", line);
                                }
                            }
                        }
                        RecentPost::Comment { post_floor, comment, .. } => {
                            let uid_display = format!("[UID: {}]", comment.author.id);
                            println!(
                                "   {} {} {} {}{}",
                                format!("#{} comment", post_floor).magenta(),
                                comment.author.name.display().green(),
                                uid_display.dimmed(),
                                format_relative_time(comment.post_date).dimmed(),
                                if comment.score != 0 { 
                                    format!(" (score: {})", comment.score).dimmed().to_string() 
                                } else { 
                                    String::new() 
                                }
                            );
                            
                            let content = comment.content.to_plain_text();
                            let preview = if content.len() > 200 {
                                format!("{}...", content.chars().take(200).collect::<String>())
                            } else {
                                content
                            };
                            
                            for line in preview.lines() {
                                if !line.trim().is_empty() {
                                    println!("          {}", line);
                                }
                            }
                        }
                    }
                }
                println!();
            }
        }
    }

    Ok(())
}

