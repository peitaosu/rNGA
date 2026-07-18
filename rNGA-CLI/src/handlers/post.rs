//! Post handlers.

use anyhow::{Context, Result};
use colored::Colorize;
use rnga::models::*;
use rnga::NGAClient;
use rust_i18n::t;
use serde::Serialize;

use crate::handlers::meta::{content_fields, ResponseMeta};
use crate::output::{format_relative_time, PlainPrint, TableRow};

#[derive(Debug, Clone, Serialize)]
pub struct VoteResultInfo {
    pub post_id: String,
    pub direction: String,
    pub up: i32,
    pub down: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct LightPostInfo {
    pub post_id: String,
    pub author: String,
    pub author_id: String,
    pub content: String,
    pub content_raw: String,
    pub content_parse_error: Option<String>,
    pub score: i32,
    pub post_date: i64,
}

impl From<&LightPost> for LightPostInfo {
    fn from(post: &LightPost) -> Self {
        let (content, content_raw, content_parse_error) = content_fields(&post.content);
        Self {
            post_id: post.id.to_string(),
            author: post.author.name.display().to_string(),
            author_id: post.author.id.to_string(),
            content,
            content_raw,
            content_parse_error,
            score: post.score,
            post_date: post.post_date,
        }
    }
}

impl TableRow for LightPostInfo {
    fn headers() -> Vec<&'static str> {
        vec!["Post ID", "Author", "Content", "Score", "Time"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.post_id.clone(),
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
            "{} {} {} {}{}",
            self.post_id.cyan(),
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

#[derive(Debug, Clone, Serialize)]
pub struct HotRepliesResult {
    pub replies: Vec<LightPostInfo>,
    pub meta: ResponseMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommentsResultInfo {
    pub topic_id: String,
    pub post_id: String,
    pub page: u32,
    pub total_pages: u32,
    pub comments: Vec<LightPostInfo>,
    pub meta: ResponseMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplyResultInfo {
    pub post_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommentResultInfo {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuoteContentInfo {
    pub topic_id: String,
    pub post_id: String,
    pub content: String,
}

pub async fn vote(
    client: &NGAClient,
    topic_id: &str,
    post_id: &str,
    vote: Vote,
) -> Result<VoteResultInfo> {
    let result = client
        .posts()
        .vote(topic_id, post_id, vote)
        .await
        .context("voting on post")?;
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

pub async fn hot_replies(
    client: &NGAClient,
    topic_id: &str,
    post_id: &str,
) -> Result<HotRepliesResult> {
    let replies = client
        .posts()
        .hot_replies(topic_id, post_id)
        .await
        .context("fetching hot replies")?;
    Ok(HotRepliesResult {
        replies: replies.iter().map(LightPostInfo::from).collect(),
        meta: ResponseMeta::empty(),
    })
}

pub async fn comments(
    client: &NGAClient,
    topic_id: &str,
    post_id: &str,
    page: u32,
) -> Result<CommentsResultInfo> {
    let result = client
        .posts()
        .comments(topic_id, post_id, page)
        .await
        .context("fetching post comments")?;
    Ok(CommentsResultInfo {
        topic_id: topic_id.to_string(),
        post_id: post_id.to_string(),
        page,
        total_pages: result.total_pages,
        comments: result.comments.iter().map(LightPostInfo::from).collect(),
        meta: ResponseMeta::page_only(page, result.total_pages),
    })
}

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

    let result = builder.send().await.context("posting reply")?;
    Ok(ReplyResultInfo {
        post_id: result.post_id.to_string(),
    })
}

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
        .await
        .context("posting comment")?;

    Ok(CommentResultInfo { success: true })
}

pub async fn fetch_quote_content(
    client: &NGAClient,
    topic_id: &str,
    post_id: &str,
) -> Result<QuoteContentInfo> {
    let content = client
        .posts()
        .fetch_quote_content(topic_id, post_id)
        .await
        .context("fetching quote content")?;
    Ok(QuoteContentInfo {
        topic_id: topic_id.to_string(),
        post_id: post_id.to_string(),
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnga::models::{LightPost, PostContent, PostId, User};

    #[test]
    fn test_light_post_info_includes_content_fields() {
        let post = LightPost {
            id: PostId::new("999"),
            author: User::anonymous("1"),
            content: PostContent::plain("hello"),
            post_date: 0,
            score: 1,
        };
        let info = LightPostInfo::from(&post);
        assert_eq!(info.post_id, "999");
        assert_eq!(info.content, "hello");
        assert_eq!(info.content_raw, "hello");
        assert!(info.content_parse_error.is_none());
    }
}
