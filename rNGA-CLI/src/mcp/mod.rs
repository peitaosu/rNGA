//! MCP Server implementation for NGA.

mod error;

use std::sync::Arc;

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use rnga::models::Vote;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{self, AuthStatus};
use crate::handlers::meta::MAX_LIST_PAGES;
use crate::handlers::options::{
    ListTopicsOptions, ReadTopicOptions, RecentTopicsOptions, SearchTopicsOptions,
    SendMessageOptions,
};
use crate::handlers::{forum, message, notification, post, topic, user};

use self::error::map_anyhow;

#[derive(Clone)]
pub struct NGAMCPServer {
    tool_router: ToolRouter<Self>,
    guest: Arc<rnga::NGAClient>,
}

impl NGAMCPServer {
    pub fn new() -> Result<Self, McpError> {
        Ok(Self {
            tool_router: Self::tool_router(),
            guest: Arc::new(config::build_client().map_err(map_anyhow)?),
        })
    }

    fn encode<T: Serialize>(value: &T) -> Result<String, McpError> {
        if std::env::var("RNGA_MCP_FORMAT").as_deref() == Ok("toon") {
            Ok(crate::output::encode_toon(value))
        } else {
            serde_json::to_string_pretty(value)
                .map_err(|error| McpError::internal_error(error.to_string(), None))
        }
    }

    fn ok(text: String) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KeywordParam {
    pub keyword: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ForumFavoriteParams {
    pub forum_id: String,
    #[serde(default)]
    pub stid: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicListParams {
    pub forum_id: String,
    #[serde(default)]
    pub stid: bool,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_one")]
    #[schemars(range(max = 5))]
    pub pages: u32,
    #[serde(default = "default_order")]
    pub order: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicReadParams {
    pub topic_id: String,
    #[serde(default = "default_page")]
    pub page: u32,
    pub author: Option<String>,
    #[serde(default)]
    pub all: bool,
    pub range: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicSearchParams {
    pub forum_id: String,
    #[serde(default)]
    pub stid: bool,
    pub keyword: String,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default)]
    pub content: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecentTopicsParams {
    pub forum_id: String,
    #[serde(default)]
    pub stid: bool,
    #[serde(default = "default_range")]
    pub range: String,
    #[serde(default)]
    pub with_posts: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserIdParam {
    pub user_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UsernameParam {
    pub username: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PostCommentsParams {
    pub topic_id: String,
    pub post_id: String,
    #[serde(default = "default_page")]
    pub page: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PostReplyParams {
    pub topic_id: String,
    pub content: String,
    pub quote_post_id: Option<String>,
    #[serde(default)]
    pub anonymous: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PostVoteParams {
    pub topic_id: String,
    pub post_id: String,
    #[serde(default = "default_upvote")]
    pub direction: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PostCommentParams {
    pub topic_id: String,
    pub post_id: String,
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NotificationIdParam {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NotificationListParams {
    #[serde(default = "default_notification_kind")]
    pub kind: String,
    #[serde(default = "default_page")]
    pub page: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MessageListParams {
    #[serde(default = "default_page")]
    pub page: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MessageReadParams {
    pub mid: String,
    #[serde(default = "default_page")]
    pub page: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendMessageParams {
    pub to: String,
    pub subject: String,
    pub content: String,
    pub reply_mid: Option<String>,
}

fn default_notification_kind() -> String {
    "reply".to_string()
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
fn default_upvote() -> String {
    "up".to_string()
}

#[tool_router]
impl NGAMCPServer {
    #[tool(description = "Check whether NGA credentials are configured")]
    async fn auth_status(&self) -> Result<CallToolResult, McpError> {
        let status: AuthStatus = config::auth_status();
        Self::ok(Self::encode(&status)?)
    }

    #[tool(description = "List all forum categories and their forums")]
    async fn forum_list(&self) -> Result<CallToolResult, McpError> {
        let result = forum::list_categories(&self.guest)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Search forums by name")]
    async fn forum_search(
        &self,
        params: Parameters<KeywordParam>,
    ) -> Result<CallToolResult, McpError> {
        let result = forum::search_forums(&self.guest, &params.0.keyword)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "List favorite forums (requires auth)")]
    async fn forum_favorites(&self) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = forum::list_favorites(&client)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Add a forum to favorites (requires auth)")]
    async fn forum_favorite_add(
        &self,
        params: Parameters<ForumFavoriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = forum::add_favorite(&client, &params.0.forum_id, params.0.stid)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Remove a forum from favorites (requires auth)")]
    async fn forum_favorite_remove(
        &self,
        params: Parameters<ForumFavoriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = forum::remove_favorite(&client, &params.0.forum_id, params.0.stid)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "List topics in a forum; pages capped at 5")]
    async fn topic_list(
        &self,
        params: Parameters<TopicListParams>,
    ) -> Result<CallToolResult, McpError> {
        let pages = params.0.pages.min(MAX_LIST_PAGES);
        let options = ListTopicsOptions {
            is_stid: params.0.stid,
            start_page: params.0.page,
            num_pages: pages,
            order: params.0.order,
            concurrency: 4,
        };
        let result = topic::list_topics(&self.guest, &params.0.forum_id, options)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Read a topic; all=true fetches up to 20 pages")]
    async fn topic_read(
        &self,
        params: Parameters<TopicReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let options = ReadTopicOptions {
            page: params.0.page,
            author: params.0.author,
            fetch_all: params.0.all,
            concurrency: 4,
            range: params.0.range,
        };
        let result = topic::read_topic(&self.guest, &params.0.topic_id, options)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Search topics in a forum by keyword")]
    async fn topic_search(
        &self,
        params: Parameters<TopicSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let options = SearchTopicsOptions {
            is_stid: params.0.stid,
            page: params.0.page,
            search_content: params.0.content,
        };
        let result = topic::search_topics(
            &self.guest,
            &params.0.forum_id,
            &params.0.keyword,
            options,
        )
        .await
        .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Recent topics in a forum; with_posts is expensive and capped")]
    async fn topic_recent(
        &self,
        params: Parameters<RecentTopicsParams>,
    ) -> Result<CallToolResult, McpError> {
        let options = RecentTopicsOptions {
            is_stid: params.0.stid,
            range: params.0.range,
            order: "lastpost".to_string(),
            with_posts: params.0.with_posts,
            concurrency: 4,
        };
        let result = topic::recent_topics(&self.guest, &params.0.forum_id, options)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Get hot replies for a post")]
    async fn post_hot_replies(
        &self,
        params: Parameters<PostCommentsParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = post::hot_replies(&self.guest, &params.0.topic_id, &params.0.post_id)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Get comments on a post")]
    async fn post_comments(
        &self,
        params: Parameters<PostCommentsParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = post::comments(
            &self.guest,
            &params.0.topic_id,
            &params.0.post_id,
            params.0.page,
        )
        .await
        .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Reply to a topic")]
    async fn post_reply(
        &self,
        params: Parameters<PostReplyParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = post::reply(
            &client,
            &params.0.topic_id,
            &params.0.content,
            params.0.quote_post_id.as_deref(),
            params.0.anonymous,
        )
        .await
        .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Vote on a post")]
    async fn post_vote(
        &self,
        params: Parameters<PostVoteParams>,
    ) -> Result<CallToolResult, McpError> {
        let vote = match params.0.direction.to_lowercase().as_str() {
            "down" => Vote::Down,
            _ => Vote::Up,
        };
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = post::vote(&client, &params.0.topic_id, &params.0.post_id, vote)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Comment on a post")]
    async fn post_comment(
        &self,
        params: Parameters<PostCommentParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = post::comment(
            &client,
            &params.0.topic_id,
            &params.0.post_id,
            &params.0.content,
        )
        .await
        .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Get user profile by ID")]
    async fn user_get(&self, params: Parameters<UserIdParam>) -> Result<CallToolResult, McpError> {
        let user_info = user::get_user(&self.guest, &params.0.user_id)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&user_info)?)
    }

    #[tool(description = "Get user profile by username")]
    async fn user_by_name(
        &self,
        params: Parameters<UsernameParam>,
    ) -> Result<CallToolResult, McpError> {
        let user_info = user::get_user_by_name(&self.guest, &params.0.username)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&user_info)?)
    }

    #[tool(description = "Search users by keyword")]
    async fn user_search(
        &self,
        params: Parameters<KeywordParam>,
    ) -> Result<CallToolResult, McpError> {
        let results = user::search_users(&self.guest, &params.0.keyword)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&results)?)
    }

    #[tool(description = "Get unread notification counts")]
    async fn notification_counts(&self) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let counts = notification::get_counts(&client)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&counts)?)
    }

    #[tool(description = "List notifications by type: reply, quote, mention, comment, system, message")]
    async fn notification_list(
        &self,
        params: Parameters<NotificationListParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = notification::list_notifications(&client, &params.0.kind, params.0.page)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Mark a notification as read")]
    async fn notification_mark_read(
        &self,
        params: Parameters<NotificationIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = notification::mark_read(&client, &params.0.id)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "List private message conversations")]
    async fn message_list(
        &self,
        params: Parameters<MessageListParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = message::list_conversations(&client, params.0.page)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Read a private message conversation")]
    async fn message_read(
        &self,
        params: Parameters<MessageReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = message::read_conversation(&client, &params.0.mid, params.0.page)
            .await
            .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
    }

    #[tool(description = "Send or reply to a private message")]
    async fn message_send(
        &self,
        params: Parameters<SendMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = config::build_authed_client().map_err(map_anyhow)?;
        let result = message::send_with_options(
            &client,
            SendMessageOptions {
                to: params.0.to,
                subject: params.0.subject,
                content: params.0.content,
                reply_mid: params.0.reply_mid,
            },
        )
        .await
        .map_err(map_anyhow)?;
        Self::ok(Self::encode(&result)?)
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

pub async fn run_server() -> anyhow::Result<()> {
    use rmcp::transport::io::stdio;

    tracing::info!("Starting rNGA MCP server");

    let server = NGAMCPServer::new().map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let service = rmcp::serve_server(server, stdio()).await?;

    tracing::info!("rNGA MCP server ready");
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_list_pages_capped_in_schema() {
        assert!(MAX_LIST_PAGES <= 5);
    }
}
