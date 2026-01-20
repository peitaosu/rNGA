//! MCP Server implementation for NGA.

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config;
use crate::handlers::{forum, post, topic, user};

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
        config::build_client().map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    fn to_toon<T: Serialize>(value: &T) -> Result<String, McpError> {
        let json_value =
            serde_json::to_value(value).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        toon_format::encode_default(&json_value)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    fn ok(text: String) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KeywordParam {
    /// Search keyword
    pub keyword: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicListParams {
    /// Forum ID (fid)
    pub forum_id: String,
    /// Treat ID as stid instead of fid
    #[serde(default)]
    pub stid: bool,
    /// Page number (default: 1)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Number of pages to fetch (default: 1)
    #[serde(default = "default_one")]
    pub pages: u32,
    /// Sort order: lastpost, postdate, recommend
    #[serde(default = "default_order")]
    pub order: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicReadParams {
    /// Topic ID
    pub topic_id: String,
    /// Page number (default: 1)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Filter by author ID
    pub author: Option<String>,
    /// Fetch all pages
    #[serde(default)]
    pub all: bool,
    /// Time range filter (e.g., 1h, 30m, 1d)
    pub range: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicSearchParams {
    /// Forum ID (fid)
    pub forum_id: String,
    /// Treat ID as stid instead of fid
    #[serde(default)]
    pub stid: bool,
    /// Search keyword
    pub keyword: String,
    /// Page number (default: 1)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Search in content
    #[serde(default)]
    pub content: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecentTopicsParams {
    /// Forum ID (fid)
    pub forum_id: String,
    /// Treat ID as stid instead of fid
    #[serde(default)]
    pub stid: bool,
    /// Time range (e.g., 1h, 30m, 1d)
    #[serde(default = "default_range")]
    pub range: String,
    /// Include individual posts/comments
    #[serde(default)]
    pub with_posts: bool,
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
pub struct PostCommentsParams {
    /// Topic ID
    pub topic_id: String,
    /// Post ID
    pub post_id: String,
    /// Page number (default: 1)
    #[serde(default = "default_page")]
    pub page: u32,
}

fn default_page() -> u32 {
    1
}
fn default_one() -> u32 {
    1
}
fn default_order() -> String {
    "lastpost".to_string()
}
fn default_range() -> String {
    "1h".to_string()
}

#[tool_router]
impl NGAMCPServer {
    #[tool(description = "List all forum categories and their forums")]
    async fn forum_list(&self) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let categories = forum::list_categories(&client)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&categories)?)
    }

    #[tool(description = "Search forums by name")]
    async fn forum_search(
        &self,
        params: Parameters<KeywordParam>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let forums = forum::search_forums(&client, &params.0.keyword)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&forums)?)
    }

    #[tool(description = "List topics in a forum with optional multi-page fetching")]
    async fn topic_list(
        &self,
        params: Parameters<TopicListParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let options = topic::ListTopicsOptions {
            is_stid: params.0.stid,
            start_page: params.0.page,
            num_pages: params.0.pages,
            order: params.0.order,
            concurrency: 4,
        };
        let result = topic::list_topics(&client, &params.0.forum_id, options)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&result)?)
    }

    #[tool(description = "Read a topic with its posts, optionally fetching all pages")]
    async fn topic_read(
        &self,
        params: Parameters<TopicReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let options = topic::ReadTopicOptions {
            page: params.0.page,
            author: params.0.author,
            fetch_all: params.0.all,
            concurrency: 4,
            range: params.0.range,
        };
        let result = topic::read_topic(&client, &params.0.topic_id, options)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&result)?)
    }

    #[tool(description = "Search topics in a forum by keyword")]
    async fn topic_search(
        &self,
        params: Parameters<TopicSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let options = topic::SearchTopicsOptions {
            is_stid: params.0.stid,
            page: params.0.page,
            search_content: params.0.content,
        };
        let result = topic::search_topics(&client, &params.0.forum_id, &params.0.keyword, options)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&result)?)
    }

    #[tool(description = "Get recent topics/posts in a forum within a time range")]
    async fn topic_recent(
        &self,
        params: Parameters<RecentTopicsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let options = topic::RecentTopicsOptions {
            is_stid: params.0.stid,
            range: params.0.range,
            order: "lastpost".to_string(),
            with_posts: params.0.with_posts,
            concurrency: 4,
        };
        let result = topic::recent_topics(&client, &params.0.forum_id, options)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&result)?)
    }

    #[tool(description = "Get hot replies for a post")]
    async fn post_hot_replies(
        &self,
        params: Parameters<PostCommentsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let replies = post::hot_replies(&client, &params.0.topic_id, &params.0.post_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&replies)?)
    }

    #[tool(description = "Get comments on a post")]
    async fn post_comments(
        &self,
        params: Parameters<PostCommentsParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let result = post::comments(
            &client,
            &params.0.topic_id,
            &params.0.post_id,
            params.0.page,
        )
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&result)?)
    }

    #[tool(description = "Get user profile by ID")]
    async fn user_get(&self, params: Parameters<UserIdParam>) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let user_info = user::get_user(&client, &params.0.user_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&user_info)?)
    }

    #[tool(description = "Get user profile by username")]
    async fn user_by_name(
        &self,
        params: Parameters<UsernameParam>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let user_info = user::get_user_by_name(&client, &params.0.username)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&user_info)?)
    }

    #[tool(description = "Search users by keyword")]
    async fn user_search(
        &self,
        params: Parameters<KeywordParam>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client()?;
        let results = user::search_users(&client, &params.0.keyword)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_toon(&results)?)
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
