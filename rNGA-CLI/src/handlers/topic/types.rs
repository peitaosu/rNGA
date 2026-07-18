use colored::Colorize;
use rnga::models::{FavoriteFolder, Post, Topic};
use rust_i18n::t;
use serde::Serialize;

use crate::handlers::meta::content_fields;
use crate::output::{format_relative_time, PlainPrint, TableRow};

#[derive(Debug, Clone, Serialize)]
pub struct TopicSummary {
    pub id: String,
    pub subject: String,
    pub tags: Vec<String>,
    pub author: String,
    pub author_id: String,
    pub replies: i32,
    pub post_date: i64,
    pub last_post_date: i64,
}

impl From<&Topic> for TopicSummary {
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

impl TableRow for TopicSummary {
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

impl PlainPrint for TopicSummary {
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

#[derive(Debug, Clone, Serialize)]
pub struct PostInfo {
    pub floor: i32,
    pub post_id: String,
    pub topic_id: String,
    pub author: String,
    pub author_id: String,
    pub content: String,
    pub content_raw: String,
    pub content_parse_error: Option<String>,
    pub score: i32,
    pub post_date: i64,
    pub comment_count: i32,
}

impl From<&Post> for PostInfo {
    fn from(p: &Post) -> Self {
        let (content, content_raw, content_parse_error) = content_fields(&p.content);
        Self {
            floor: p.floor,
            post_id: p.id.to_string(),
            topic_id: p.topic_id.to_string(),
            author: p.author.name.display().to_string(),
            author_id: p.author.id.to_string(),
            content,
            content_raw,
            content_parse_error,
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

#[derive(Debug, Clone, Serialize)]
pub struct CliTopicListResult {
    pub forum_name: Option<String>,
    pub start_page: u32,
    pub end_page: u32,
    pub total_pages: u32,
    pub topics: Vec<TopicSummary>,
    pub meta: crate::handlers::meta::ResponseMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct CliTopicDetailsResult {
    pub topic_id: String,
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
    pub meta: crate::handlers::meta::ResponseMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct CliTopicSearchResult {
    pub keyword: String,
    pub page: u32,
    pub total_pages: u32,
    pub topics: Vec<TopicSummary>,
    pub meta: crate::handlers::meta::ResponseMeta,
}

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

#[derive(Debug, Clone, Serialize)]
pub struct CliFavoriteTopicsResult {
    pub folder: Option<String>,
    pub page: u32,
    pub total_pages: u32,
    pub topics: Vec<TopicSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FavoriteModifyResult {
    pub topic_id: String,
    pub action: String,
}

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

#[derive(Debug, Clone, Serialize)]
pub struct CliRecentResult {
    pub forum_name: Option<String>,
    pub range_display: String,
    pub topics: Vec<TopicSummary>,
    pub posts: Vec<RecentPostInfo>,
    pub meta: crate::handlers::meta::ResponseMeta,
}
