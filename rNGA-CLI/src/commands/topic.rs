//! Topic commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use rust_i18n::t;
use std::collections::HashMap;

use crate::config::{build_authed_client, build_client};
use crate::handlers::topic::{
    self as handlers, ListTopicsOptions, ReadTopicOptions, RecentTopicsOptions, SearchTopicsOptions,
};
use crate::output::{format_relative_time, format_time, print_table, OutputFormat};

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
        /// Time range filter (e.g., 1h, 30m, 1d)
        #[arg(short = 'r', long)]
        range: Option<String>,
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
            range,
            concurrency,
        } => read_topic(&topic_id, page, author, all, range, concurrency, format, verbose).await,
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
            order,
            with_posts,
            concurrency,
        } => {
            recent_topics(
                &forum_id,
                stid,
                &range,
                &order,
                with_posts,
                concurrency,
                format,
            )
            .await
        }
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
    let client = build_client()?;

    let options = ListTopicsOptions {
        is_stid,
        start_page,
        num_pages,
        order: order.to_string(),
        concurrency,
    };

    let result = handlers::list_topics(&client, forum_id, options).await?;

    if matches!(format, OutputFormat::Plain) {
        if let Some(forum_name) = &result.forum_name {
            if result.start_page != result.end_page {
                println!(
                    "{}",
                    t!(
                        "forum_pages_range",
                        name = forum_name.bold(),
                        start = result.start_page,
                        end = result.end_page,
                        total = result.total_pages
                    )
                );
            } else {
                println!(
                    "{}",
                    t!(
                        "forum_page_single",
                        name = forum_name.bold(),
                        page = result.start_page,
                        total = result.total_pages
                    )
                );
            }
            println!();
        }
    }

    print_table(result.topics, format);
    Ok(())
}

async fn read_topic(
    topic_id: &str,
    page: u32,
    author: Option<String>,
    fetch_all: bool,
    range: Option<String>,
    concurrency: usize,
    format: OutputFormat,
    _verbose: bool,
) -> Result<()> {
    let client = build_client()?;

    let options = ReadTopicOptions {
        page,
        author,
        fetch_all,
        concurrency,
        range,
    };

    let result = handlers::read_topic(&client, topic_id, options).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{} {}",
            format!("[{}]", result.forum_name).dimmed(),
            result.subject.bold()
        );
        if !result.tags.is_empty() {
            println!("{}", t!("topic_tags", tags = result.tags.join(", ").cyan()));
        }
        println!(
            "{}",
            t!(
                "topic_by_author",
                author = result.author.green(),
                date = format_time(result.post_date),
                replies = result.replies
            )
        );
        if fetch_all && result.total_pages > 1 {
            println!(
                "{}\n",
                t!("topic_fetched_all_pages", total = result.total_pages)
            );
        } else {
            println!(
                "{}\n",
                t!(
                    "topic_page_info",
                    page = result.page,
                    total = result.total_pages
                )
            );
        }
    }

    print_table(result.posts, format);
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

    let options = SearchTopicsOptions {
        is_stid,
        page,
        search_content,
    };

    let result = handlers::search_topics(&client, forum_id, keyword, options).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{}\n",
            t!(
                "search_results_for",
                keyword = keyword.cyan(),
                page = page,
                total = result.total_pages
            )
        );
    }

    print_table(result.topics, format);
    Ok(())
}

async fn list_folders(format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let folders = handlers::list_folders(&client).await?;

    print_table(folders, format);
    Ok(())
}

async fn list_favorites(folder: Option<String>, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::list_favorites(&client, folder.as_deref(), page).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{}\n",
            t!("favorite_topics", page = page, total = result.total_pages)
        );
    }

    print_table(result.topics, format);
    Ok(())
}

async fn add_favorite(topic_id: &str, folder: Option<String>) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::add_favorite(&client, topic_id, folder.as_deref()).await?;

    println!("{}", t!("added_topic_to_favorites", id = result.topic_id));
    Ok(())
}

async fn remove_favorite(topic_id: &str, folder: Option<String>) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::remove_favorite(&client, topic_id, folder.as_deref()).await?;

    println!(
        "{}",
        t!("removed_topic_from_favorites", id = result.topic_id)
    );
    Ok(())
}

async fn recent_topics(
    forum_id: &str,
    is_stid: bool,
    range: &str,
    order: &str,
    with_posts: bool,
    concurrency: usize,
    format: OutputFormat,
) -> Result<()> {
    let client = build_client()?;

    let options = RecentTopicsOptions {
        is_stid,
        range: range.to_string(),
        order: order.to_string(),
        with_posts,
        concurrency,
    };

    let result = handlers::recent_topics(&client, forum_id, options).await?;

    if matches!(format, OutputFormat::Plain) {
        if let Some(forum_name) = &result.forum_name {
            let content_type = if with_posts {
                t!("recent_posts_type").to_string()
            } else {
                t!("recent_topics_type").to_string()
            };
            println!(
                "{}",
                t!(
                    "recent_content_header",
                    forum = forum_name.bold(),
                    content_type = content_type,
                    range = result.range_display.cyan()
                )
            );
            println!();
        }
    }

    if !with_posts {
        if result.topics.is_empty() {
            if matches!(format, OutputFormat::Plain) {
                println!("{}", t!("no_topics_in_range"));
            }
            return Ok(());
        }
        print_table(result.topics, format);
        return Ok(());
    }

    if result.posts.is_empty() {
        if matches!(format, OutputFormat::Plain) {
            println!("{}", t!("no_posts_in_range"));
        }
        return Ok(());
    }

    if matches!(format, OutputFormat::Plain) {
        let msg = if result.posts.len() != 1 {
            t!("found_recent_posts_plural", count = result.posts.len())
        } else {
            t!("found_recent_posts", count = result.posts.len())
        };
        println!("{}\n", msg);
    }

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result.posts)?);
        }
        OutputFormat::Toon => {
            let json_value = serde_json::to_value(&result.posts)?;
            println!("{}", toon_format::encode_default(&json_value).unwrap_or_default());
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let mut posts_by_topic: HashMap<String, Vec<&handlers::RecentPostInfo>> =
                HashMap::new();
            for post in &result.posts {
                posts_by_topic
                    .entry(post.topic_id.clone())
                    .or_default()
                    .push(post);
            }

            let mut topic_order: Vec<(String, i64)> = posts_by_topic
                .iter()
                .map(|(topic_id, posts)| {
                    let max_time = posts.iter().map(|p| p.post_date).max().unwrap_or(0);
                    (topic_id.clone(), max_time)
                })
                .collect();
            topic_order.sort_by(|a, b| b.1.cmp(&a.1));

            for (topic_id, _) in topic_order {
                let first_post = result
                    .posts
                    .iter()
                    .find(|p| p.topic_id == topic_id)
                    .unwrap();

                println!(
                    "{} {}",
                    t!("topic_label", id = &topic_id).to_string().cyan(),
                    first_post.topic_subject.bold()
                );

                let mut topic_posts: Vec<_> = posts_by_topic[&topic_id].clone();
                topic_posts.sort_by_key(|p| &p.floor);

                for post in topic_posts {
                    let floor_display = if post.post_type == "comment" {
                        format!("{}", post.floor).magenta()
                    } else {
                        format!("{}", post.floor).yellow()
                    };

                    println!(
                        "   {} {} {} {}{}",
                        floor_display,
                        post.author_name.green(),
                        t!("uid_label", id = &post.author_id).to_string().dimmed(),
                        format_relative_time(post.post_date).dimmed(),
                        if post.score != 0 {
                            format!(" {}", t!("score_label", score = post.score))
                                .dimmed()
                                .to_string()
                        } else {
                            String::new()
                        }
                    );

                    let preview = if post.content.len() > 200 {
                        format!("{}...", post.content.chars().take(200).collect::<String>())
                    } else {
                        post.content.clone()
                    };

                    let indent = if post.post_type == "comment" {
                        "          "
                    } else {
                        "        "
                    };
                    for line in preview.lines() {
                        if !line.trim().is_empty() {
                            println!("{}{}", indent, line);
                        }
                    }
                }
                println!();
            }
        }
    }

    Ok(())
}
