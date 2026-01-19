//! Post commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use rnga::models::*;
use rust_i18n::t;

use crate::config::{build_authed_client, build_client};
use crate::handlers::post as handlers;
use crate::output::{print_table, OutputFormat};

#[derive(Subcommand)]
pub enum PostAction {
    /// Upvote a post
    Up {
        /// Topic ID
        #[arg(short, long)]
        topic: String,
        /// Post ID
        #[arg(short, long)]
        post: String,
    },

    /// Downvote a post
    Down {
        /// Topic ID
        #[arg(short, long)]
        topic: String,
        /// Post ID
        #[arg(short, long)]
        post: String,
    },

    /// View hot replies for a post
    Hot {
        /// Topic ID
        #[arg(short, long)]
        topic: String,
        /// Post ID
        #[arg(short, long)]
        post: String,
    },

    /// View comments on a post
    Comments {
        /// Topic ID
        #[arg(short, long)]
        topic: String,
        /// Post ID
        #[arg(short, long)]
        post: String,
        /// Page number
        #[arg(long, default_value = "1")]
        page: u32,
    },

    /// Reply to a topic
    Reply {
        /// Topic ID
        topic_id: String,
        /// Reply content
        content: String,
        /// Quote a post
        #[arg(short, long)]
        quote: Option<String>,
        /// Post anonymously
        #[arg(short, long)]
        anonymous: bool,
    },

    /// Comment on a post
    Comment {
        /// Topic ID
        #[arg(short, long)]
        topic: String,
        /// Post ID
        #[arg(short, long)]
        post: String,
        /// Comment content
        content: String,
    },

    /// Fetch quote content for a post
    Quote {
        /// Topic ID
        #[arg(short, long)]
        topic: String,
        /// Post ID
        #[arg(short, long)]
        post: String,
    },
}

pub async fn handle(action: PostAction, format: OutputFormat, _verbose: bool) -> Result<()> {
    match action {
        PostAction::Up { topic, post } => vote(&topic, &post, Vote::Up).await,
        PostAction::Down { topic, post } => vote(&topic, &post, Vote::Down).await,
        PostAction::Hot { topic, post } => hot_replies(&topic, &post, format).await,
        PostAction::Comments { topic, post, page } => comments(&topic, &post, page, format).await,
        PostAction::Reply {
            topic_id,
            content,
            quote,
            anonymous,
        } => reply(&topic_id, &content, quote, anonymous).await,
        PostAction::Comment {
            topic,
            post,
            content,
        } => comment(&topic, &post, &content).await,
        PostAction::Quote { topic, post } => fetch_quote(&topic, &post).await,
    }
}

async fn vote(topic_id: &str, post_id: &str, vote: Vote) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::vote(&client, topic_id, post_id, vote).await?;

    let msg = match result.direction.as_str() {
        "up" => t!(
            "upvoted_post",
            id = result.post_id,
            up = result.up,
            down = result.down
        )
        .to_string()
        .green(),
        _ => t!(
            "downvoted_post",
            id = result.post_id,
            up = result.up,
            down = result.down
        )
        .to_string()
        .red(),
    };

    println!("{}", msg);

    Ok(())
}

async fn hot_replies(topic_id: &str, post_id: &str, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let replies = handlers::hot_replies(&client, topic_id, post_id).await?;

    if replies.is_empty() {
        if matches!(format, OutputFormat::Plain) {
            println!("{}", t!("no_hot_replies"));
        }
        return Ok(());
    }

    if matches!(format, OutputFormat::Plain) {
        println!("{}\n", t!("hot_replies_count", count = replies.len()));
    }

    print_table(replies, format);
    Ok(())
}

async fn comments(topic_id: &str, post_id: &str, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let result = handlers::comments(&client, topic_id, post_id, page).await?;

    if result.comments.is_empty() {
        if matches!(format, OutputFormat::Plain) {
            println!("{}", t!("no_comments"));
        }
        return Ok(());
    }

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{}\n",
            t!("comments_page", page = page, total = result.total_pages)
        );
    }

    print_table(result.comments, format);
    Ok(())
}

async fn reply(
    topic_id: &str,
    content: &str,
    quote: Option<String>,
    anonymous: bool,
) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::reply(&client, topic_id, content, quote.as_deref(), anonymous).await?;

    println!("{}", t!("posted_reply", id = result.post_id));

    Ok(())
}

async fn comment(topic_id: &str, post_id: &str, content: &str) -> Result<()> {
    let client = build_authed_client()?;
    handlers::comment(&client, topic_id, post_id, content).await?;

    println!("{}", t!("posted_comment"));

    Ok(())
}

async fn fetch_quote(topic_id: &str, post_id: &str) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::fetch_quote_content(&client, topic_id, post_id).await?;

    println!("{}", result.content);

    Ok(())
}
