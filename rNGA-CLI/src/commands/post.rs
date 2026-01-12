//! Post commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use rnga::models::*;

use crate::config::{build_authed_client, build_client};
use crate::output::{print_table, LightPostRow, OutputFormat};

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
    let result = client.posts().vote(topic_id, post_id, vote).await?;

    let direction = match vote {
        Vote::Up => "Upvoted".green(),
        Vote::Down => "Downvoted".red(),
    };

    println!(
        "{} post {}. Score: {} up, {} down",
        direction, post_id, result.state.up, result.state.down
    );

    Ok(())
}

async fn hot_replies(topic_id: &str, post_id: &str, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let replies = client.posts().hot_replies(topic_id, post_id).await?;

    if replies.is_empty() {
        if matches!(format, OutputFormat::Plain) {
            println!("No hot replies");
        }
        return Ok(());
    }

    if matches!(format, OutputFormat::Plain) {
        println!("{} hot replies\n", replies.len());
    }

    let rows: Vec<LightPostRow> = replies.iter().map(LightPostRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn comments(topic_id: &str, post_id: &str, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let result = client.posts().comments(topic_id, post_id, page).await?;

    if result.comments.is_empty() {
        if matches!(format, OutputFormat::Plain) {
            println!("No comments");
        }
        return Ok(());
    }

    if matches!(format, OutputFormat::Plain) {
        println!(
            "Comments (page {}/{})\n",
            page, result.total_pages
        );
    }

    let rows: Vec<LightPostRow> = result.comments.iter().map(LightPostRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn reply(
    topic_id: &str,
    content: &str,
    quote: Option<String>,
    anonymous: bool,
) -> Result<()> {
    let client = build_authed_client()?;

    let mut builder = client.posts().reply(topic_id).content(content);

    if let Some(quote_id) = quote {
        builder = builder.quote(quote_id);
    }

    if anonymous {
        builder = builder.anonymous(true);
    }

    let result = builder.send().await?;

    println!(
        "{} Posted reply (post ID: {})",
        "✓".green(),
        result.post_id
    );

    Ok(())
}

async fn comment(topic_id: &str, post_id: &str, content: &str) -> Result<()> {
    let client = build_authed_client()?;

    client
        .posts()
        .comment(topic_id, post_id)
        .content(content)
        .send()
        .await?;

    println!("{} Posted comment", "✓".green());

    Ok(())
}

async fn fetch_quote(topic_id: &str, post_id: &str) -> Result<()> {
    let client = build_authed_client()?;
    let content = client.posts().fetch_quote_content(topic_id, post_id).await?;

    println!("{}", content);

    Ok(())
}

