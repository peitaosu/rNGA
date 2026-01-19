//! User handlers.

use anyhow::Result;
use colored::Colorize;
use rnga::NGAClient;
use rust_i18n::t;
use serde::Serialize;

use crate::output::{format_relative_time, format_time, PlainPrint, TableRow};

/// User profile information.
#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub reputation: i32,
    pub posts: i32,
    pub reg_date: String,
    pub reg_timestamp: i64,
}

impl From<&rnga::User> for UserInfo {
    fn from(u: &rnga::User) -> Self {
        Self {
            id: u.id.to_string(),
            name: u.name.display().to_string(),
            reputation: u.reputation,
            posts: u.posts,
            reg_date: format_time(u.reg_date),
            reg_timestamp: u.reg_date,
        }
    }
}

impl TableRow for UserInfo {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Name", "Reputation", "Posts", "Registered"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.reputation.to_string(),
            self.posts.to_string(),
            self.reg_date.clone(),
        ]
    }
}

impl PlainPrint for UserInfo {
    fn plain_print(&self) {
        println!(
            "{} {}",
            t!("uid_label", id = &self.id).to_string().cyan(),
            self.name.bold()
        );
        println!(
            "   {} | {} | {}",
            t!("rep_label", rep = self.reputation),
            t!("posts_label", posts = self.posts),
            t!("registered_label", date = self.reg_date.dimmed())
        );
    }
}

/// User search result.
#[derive(Debug, Clone, Serialize)]
pub struct UserSearchInfo {
    pub id: String,
    pub name: String,
}

impl TableRow for UserSearchInfo {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Name"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.id.clone(), self.name.clone()]
    }
}

impl PlainPrint for UserSearchInfo {
    fn plain_print(&self) {
        println!("{}: {}", self.id, self.name.green());
    }
}

/// User's topics result.
#[derive(Debug, Clone, Serialize)]
pub struct UserTopicsResult {
    pub user_id: String,
    pub page: u32,
    pub total_pages: u32,
    pub topics: Vec<TopicInfo>,
}

/// Topic info for user's topics.
#[derive(Debug, Clone, Serialize)]
pub struct TopicInfo {
    pub id: String,
    pub subject: String,
    pub replies: i32,
    pub post_date: i64,
    pub last_post_date: i64,
}

impl TableRow for TopicInfo {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Subject", "Replies", "Last Post"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.subject.clone(),
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
            "   {} | {}",
            format_relative_time(self.last_post_date).dimmed(),
            t!("replies_label", count = self.replies)
        );
    }
}

/// User's posts result.
#[derive(Debug, Clone, Serialize)]
pub struct UserPostsResult {
    pub user_id: String,
    pub page: u32,
    pub total_pages: u32,
    pub posts: Vec<UserPostInfo>,
}

/// Post info for user's posts.
#[derive(Debug, Clone, Serialize)]
pub struct UserPostInfo {
    pub post_id: String,
    pub topic_id: String,
    pub topic_subject: String,
    pub content_preview: String,
}

impl TableRow for UserPostInfo {
    fn headers() -> Vec<&'static str> {
        vec!["Post ID", "Topic ID", "Subject", "Preview"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.post_id.clone(),
            self.topic_id.clone(),
            self.topic_subject.clone(),
            self.content_preview.clone(),
        ]
    }
}

impl PlainPrint for UserPostInfo {
    fn plain_print(&self) {
        println!(
            "{} {}",
            format!("[{}]", self.post_id).yellow(),
            t!("in_topic", id = &self.topic_id)
        );
        println!("   {}", self.topic_subject.dimmed());
        println!("   {}", self.content_preview);
        println!();
    }
}

/// Get user profile by ID.
pub async fn get_user(client: &NGAClient, user_id: &str) -> Result<UserInfo> {
    let user = client.users().get(user_id).await?;
    Ok(UserInfo::from(&user))
}

/// Get user profile by username.
pub async fn get_user_by_name(client: &NGAClient, username: &str) -> Result<UserInfo> {
    let user = client.users().get_by_name(username).await?;
    Ok(UserInfo::from(&user))
}

/// Get current authenticated user.
pub async fn get_me(client: &NGAClient) -> Result<UserInfo> {
    let user = client.users().me().await?;
    Ok(UserInfo::from(&user))
}

/// Search users by keyword.
pub async fn search_users(client: &NGAClient, keyword: &str) -> Result<Vec<UserSearchInfo>> {
    let results = client.users().search(keyword).await?;
    Ok(results
        .iter()
        .map(|u| UserSearchInfo {
            id: u.id.to_string(),
            name: u.name.clone(),
        })
        .collect())
}

/// Get topics posted by a user.
pub async fn user_topics(client: &NGAClient, user_id: &str, page: u32) -> Result<UserTopicsResult> {
    let result = client.topics().by_user(user_id, page).await?;
    Ok(UserTopicsResult {
        user_id: user_id.to_string(),
        page,
        total_pages: result.total_pages,
        topics: result
            .topics
            .iter()
            .map(|t| TopicInfo {
                id: t.id.to_string(),
                subject: t.subject.content.clone(),
                replies: t.replies,
                post_date: t.post_date,
                last_post_date: t.last_post_date,
            })
            .collect(),
    })
}

/// Get posts by a user.
pub async fn user_posts(client: &NGAClient, user_id: &str, page: u32) -> Result<UserPostsResult> {
    let result = client.posts().by_user(user_id, page).await?;
    Ok(UserPostsResult {
        user_id: user_id.to_string(),
        page,
        total_pages: result.total_pages,
        posts: result
            .posts
            .iter()
            .map(|p| UserPostInfo {
                post_id: p.post_id.to_string(),
                topic_id: p.topic_id.to_string(),
                topic_subject: p.topic_subject.clone(),
                content_preview: p.content_preview.clone(),
            })
            .collect(),
    })
}
