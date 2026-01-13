//! MCP Server implementation for NGA.

use rmcp::{
    ServerHandler, tool, tool_handler, tool_router,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config;

/// MCP Server for NGA forum operations.
#[derive(Clone)]
pub struct NGAMCPServer {
    tool_router: ToolRouter<Self>,
}

impl NGAMCPServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    fn build_client() -> Result<rnga::NGAClient, McpError> {
        config::build_client()
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    fn build_authed_client() -> Result<rnga::NGAClient, McpError> {
        config::build_authed_client()
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    fn to_json<T: Serialize>(value: &T) -> Result<String, McpError> {
        serde_json::to_string_pretty(value)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    fn ok(text: String) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

// Parameter structs
#[derive(Debug, Deserialize, JsonSchema)]
pub struct KeywordParam {
    /// Search keyword
    pub keyword: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicListParams {
    /// Forum ID (fid)
    pub forum_id: String,
    /// Page number (default: 1)
    #[serde(default = "default_page")]
    pub page: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicReadParams {
    /// Topic ID
    pub topic_id: String,
    /// Page number (default: 1)
    #[serde(default = "default_page")]
    pub page: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicSearchParams {
    /// Forum ID (fid)
    pub forum_id: String,
    /// Search keyword
    pub keyword: String,
    /// Page number (default: 1)
    #[serde(default = "default_page")]
    pub page: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserIdParam {
    /// User ID
    pub user_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UsernameParam {
    /// Username
    pub username: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PostReplyParams {
    /// Topic ID to reply to
    pub topic_id: String,
    /// Reply content
    pub content: String,
}

fn default_page() -> u32 { 1 }

#[tool_router]
impl NGAMCPServer {
    #[tool(description = "List all forum categories and their forums")]
    async fn forum_list(&self) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let categories = client.forums().list().await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_json(&categories)?)
    }

    #[tool(description = "Search forums by name")]
    async fn forum_search(&self, params: Parameters<KeywordParam>) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let forums = client.forums().search(&params.0.keyword).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_json(&forums)?)
    }

    #[tool(description = "List topics in a forum")]
    async fn topic_list(&self, params: Parameters<TopicListParams>) -> Result<CallToolResult, McpError> {
        use rnga::models::ForumIdKind;

        let client = Self::build_client()?;
        let result = client.topics()
            .list(ForumIdKind::fid(&params.0.forum_id))
            .page(params.0.page)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Output { forum: Option<String>, page: u32, total_pages: u32, topics: Vec<TopicInfo> }
        #[derive(Serialize)]
        struct TopicInfo { id: String, subject: String, author: String, replies: i32 }

        let output = Output {
            forum: result.forum.map(|f| f.name),
            page: params.0.page,
            total_pages: result.total_pages,
            topics: result.topics.iter().map(|t| TopicInfo {
                id: t.id.to_string(),
                subject: t.subject.content.clone(),
                author: t.author.name.display().to_string(),
                replies: t.replies,
            }).collect(),
        };
        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Read a topic with its posts")]
    async fn topic_read(&self, params: Parameters<TopicReadParams>) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let result = client.topics()
            .details(&params.0.topic_id)
            .page(params.0.page)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Output { forum: String, subject: String, author: String, page: u32, total_pages: u32, posts: Vec<PostInfo> }
        #[derive(Serialize)]
        struct PostInfo { floor: i32, author: String, content: String, score: i32 }

        let output = Output {
            forum: result.forum_name,
            subject: result.topic.subject.content.clone(),
            author: result.topic.author.name.display().to_string(),
            page: params.0.page,
            total_pages: result.total_pages,
            posts: result.posts.iter().map(|p| PostInfo {
                floor: p.floor,
                author: p.author.name.display().to_string(),
                content: p.content.to_plain_text(),
                score: p.score,
            }).collect(),
        };
        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Search topics in a forum by keyword")]
    async fn topic_search(&self, params: Parameters<TopicSearchParams>) -> Result<CallToolResult, McpError> {
        use rnga::models::ForumIdKind;

        let client = Self::build_client()?;
        let result = client.topics()
            .search(ForumIdKind::fid(&params.0.forum_id), &params.0.keyword)
            .page(params.0.page)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Output { keyword: String, page: u32, total_pages: u32, topics: Vec<TopicInfo> }
        #[derive(Serialize)]
        struct TopicInfo { id: String, subject: String, author: String, replies: i32 }

        let output = Output {
            keyword: params.0.keyword.clone(),
            page: params.0.page,
            total_pages: result.total_pages,
            topics: result.topics.iter().map(|t| TopicInfo {
                id: t.id.to_string(),
                subject: t.subject.content.clone(),
                author: t.author.name.display().to_string(),
                replies: t.replies,
            }).collect(),
        };
        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get user profile by ID")]
    async fn user_get(&self, params: Parameters<UserIdParam>) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let user = client.users().get(&params.0.user_id).await
            .map_err(|e: rnga::Error| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_json(&user)?)
    }

    #[tool(description = "Get user profile by username")]
    async fn user_by_name(&self, params: Parameters<UsernameParam>) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let user = client.users().get_by_name(&params.0.username).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_json(&user)?)
    }

    #[tool(description = "Reply to a topic (requires authentication via CLI: rnga auth login)")]
    async fn post_reply(&self, params: Parameters<PostReplyParams>) -> Result<CallToolResult, McpError> {
        let client = Self::build_authed_client()?;
        let result = client.posts()
            .reply(&params.0.topic_id)
            .content(&params.0.content)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(format!("Reply posted (post ID: {})", result.post_id))
    }

    #[tool(description = "Get unread notification counts (requires authentication)")]
    async fn notification_counts(&self) -> Result<CallToolResult, McpError> {
        let client = Self::build_authed_client()?;
        let counts = client.notifications().counts().await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Output { replies: i32, mentions: i32, messages: i32, total: i32 }

        let output = Output {
            replies: counts.replies,
            mentions: counts.mentions,
            messages: counts.messages,
            total: counts.total(),
        };
        Self::ok(Self::to_json(&output)?)
    }
}

#[tool_handler]
impl ServerHandler for NGAMCPServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "rnga-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                icons: None,
                website_url: None,
            },
            ..Default::default()
        }
    }
}

/// Run the MCP Server.
pub async fn run_server() -> anyhow::Result<()> {
    use rmcp::transport::io::stdio;

    tracing::info!("Starting rNGA MCP server");

    let server = NGAMCPServer::new();
    let service = rmcp::serve_server(server, stdio()).await?;

    tracing::info!("rNGA MCP server ready");
    service.waiting().await?;

    Ok(())
}
