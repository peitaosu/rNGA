//! Shared handler option types for CLI and MCP.

#[derive(Debug, Clone, Default)]
pub struct ListTopicsOptions {
    pub is_stid: bool,
    pub start_page: u32,
    pub num_pages: u32,
    pub order: String,
    pub concurrency: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ReadTopicOptions {
    pub page: u32,
    pub author: Option<String>,
    pub fetch_all: bool,
    pub concurrency: usize,
    pub range: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchTopicsOptions {
    pub is_stid: bool,
    pub page: u32,
    pub search_content: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RecentTopicsOptions {
    pub is_stid: bool,
    pub range: String,
    pub order: String,
    pub with_posts: bool,
    pub concurrency: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SendMessageOptions {
    pub to: String,
    pub subject: String,
    pub content: String,
    pub reply_mid: Option<String>,
}
