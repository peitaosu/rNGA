//! Post handlers.

use anyhow::Result;
use colored::Colorize;
use rnga::models::*;
use rnga::NGAClient;
use rust_i18n::t;
use serde::Serialize;

use crate::output::{format_relative_time, PlainPrint, TableRow};

/// Vote result.
#[derive(Debug, Clone, Serialize)]
pub struct VoteResultInfo {
    pub post_id: String,
    pub direction: String,
    pub up: i32,
    pub down: i32,
}

/// Light post information.
#[derive(Debug, Clone, Serialize)]
pub struct LightPostInfo {
    pub author: String,
    pub author_id: String,
    pub content: String,
    pub score: i32,
    pub post_date: i64,
}

impl From<&LightPost> for LightPostInfo {
    fn from(p: &LightPost) -> Self {
        Self {
            author: p.author.name.display().to_string(),
            author_id: p.author.id.to_string(),
            content: p.content.to_plain_text(),
            score: p.score,
            post_date: p.post_date,
        }
    }
}

impl TableRow for LightPostInfo {
    fn headers() -> Vec<&'static str> {
        vec!["Author", "Content", "Score", "Time"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.author.clone(),
            self.content.clone(),
            self.score.to_string(),
            format_relative_time(self.post_date),
        ]
    }
}

impl PlainPrint for LightPostInfo {
    fn plain_print(&self) {
        println!(
            "{} {} {}{}",
            self.author.green(),
            t!("uid_label", id = &self.author_id).to_string().dimmed(),
            format_relative_time(self.post_date).dimmed(),
            if self.score != 0 {
                format!(" (+{})", self.score).yellow().to_string()
            } else {
                String::new()
            }
        );
        for line in self.content.lines() {
            if !line.trim().is_empty() {
                println!("   {}", line);
            }
        }
        println!();
    }
}

/// Comments result.
#[derive(Debug, Clone, Serialize)]
pub struct CommentsResultInfo {
    pub topic_id: String,
    pub post_id: String,
    pub page: u32,
    pub total_pages: u32,
    pub comments: Vec<LightPostInfo>,
}

/// Reply result.
#[derive(Debug, Clone, Serialize)]
pub struct ReplyResultInfo {
    pub post_id: String,
}

/// Comment result.
#[derive(Debug, Clone, Serialize)]
pub struct CommentResultInfo {
    pub success: bool,
}

/// Quote content result.
#[derive(Debug, Clone, Serialize)]
pub struct QuoteContentInfo {
    pub topic_id: String,
    pub post_id: String,
    pub content: String,
}

/// Vote on a post.
pub async fn vote(
    client: &NGAClient,
    topic_id: &str,
    post_id: &str,
    vote: Vote,
) -> Result<VoteResultInfo> {
    let result = client.posts().vote(topic_id, post_id, vote).await?;
    Ok(VoteResultInfo {
        post_id: post_id.to_string(),
        direction: match vote {
            Vote::Up => "up".to_string(),
            Vote::Down => "down".to_string(),
        },
        up: result.state.up,
        down: result.state.down,
    })
}

/// Get hot replies for a post.
pub async fn hot_replies(
    client: &NGAClient,
    topic_id: &str,
    post_id: &str,
) -> Result<Vec<LightPostInfo>> {
    let replies = client.posts().hot_replies(topic_id, post_id).await?;
    Ok(replies.iter().map(LightPostInfo::from).collect())
}

/// Get comments on a post.
pub async fn comments(
    client: &NGAClient,
    topic_id: &str,
    post_id: &str,
    page: u32,
) -> Result<CommentsResultInfo> {
    let result = client.posts().comments(topic_id, post_id, page).await?;
    Ok(CommentsResultInfo {
        topic_id: topic_id.to_string(),
        post_id: post_id.to_string(),
        page,
        total_pages: result.total_pages,
        comments: result.comments.iter().map(LightPostInfo::from).collect(),
    })
}

/// Reply to a topic.
pub async fn reply(
    client: &NGAClient,
    topic_id: &str,
    content: &str,
    quote_post_id: Option<&str>,
    anonymous: bool,
) -> Result<ReplyResultInfo> {
    let mut builder = client.posts().reply(topic_id).content(content);

    if let Some(quote_id) = quote_post_id {
        builder = builder.quote(quote_id);
    }

    if anonymous {
        builder = builder.anonymous(true);
    }

    let result = builder.send().await?;
    Ok(ReplyResultInfo {
        post_id: result.post_id.to_string(),
    })
}

/// Comment on a post.
pub async fn comment(
    client: &NGAClient,
    topic_id: &str,
    post_id: &str,
    content: &str,
) -> Result<CommentResultInfo> {
    client
        .posts()
        .comment(topic_id, post_id)
        .content(content)
        .send()
        .await?;

    Ok(CommentResultInfo { success: true })
}

/// Fetch quote content for a post.
pub async fn fetch_quote_content(
    client: &NGAClient,
    topic_id: &str,
    post_id: &str,
) -> Result<QuoteContentInfo> {
    let content = client
        .posts()
        .fetch_quote_content(topic_id, post_id)
        .await?;
    Ok(QuoteContentInfo {
        topic_id: topic_id.to_string(),
        post_id: post_id.to_string(),
        content,
    })
}
