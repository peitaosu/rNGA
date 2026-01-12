//! Post API.

use std::sync::Arc;

use crate::{
    client::NGAClientInner,
    error::{Error, Result},
    models::{LightPost, PostId, TopicId, User, UserName, Vote, VoteState},
    parser::{parse_content, XmlDocument},
};

/// API for post operations.
pub struct PostApi {
    client: Arc<NGAClientInner>,
}

impl PostApi {
    pub(crate) fn new(client: Arc<NGAClientInner>) -> Self {
        Self { client }
    }

    /// Vote on a post.
    pub async fn vote(
        &self,
        topic_id: impl AsRef<str>,
        post_id: impl AsRef<str>,
        vote: Vote,
    ) -> Result<VoteResult> {
        let xml = self.client.post_authed(
            "nuke.php",
            &[
                ("__lib", "topic_recommend"),
                ("__act", "add"),
                ("raw", "3"),
            ],
            &[
                ("tid", topic_id.as_ref()),
                ("pid", post_id.as_ref()),
                ("value", vote.param()),
            ],
        ).await?;

        let doc = XmlDocument::parse(&xml)?;
        
        let up = doc.int_or("/root/data/item[1]", 0) as i32;
        let down = doc.int_or("/root/data/item[2]", 0) as i32;
        let user_vote = doc.int_or("/root/data/item[3]", 0);

        Ok(VoteResult {
            state: VoteState {
                up,
                down,
                user_vote: match user_vote {
                    1 => Some(Vote::Up),
                    0 => Some(Vote::Down),
                    _ => None,
                },
            },
        })
    }

    /// Get hot replies for a post.
    pub async fn hot_replies(
        &self,
        topic_id: impl AsRef<str>,
        post_id: impl AsRef<str>,
    ) -> Result<Vec<LightPost>> {
        let xml = self.client.post(
            "nuke.php",
            &[
                ("__lib", "post_recommend"),
                ("__act", "get"),
                ("pid", post_id.as_ref()),
                ("tid", topic_id.as_ref()),
            ],
            &[],
        ).await?;

        parse_hot_replies(&xml)
    }

    /// Get comments on a post.
    pub async fn comments(
        &self,
        topic_id: impl AsRef<str>,
        post_id: impl AsRef<str>,
        page: u32,
    ) -> Result<CommentsResult> {
        let page_str = page.to_string();
        
        let xml = self.client.post(
            "nuke.php",
            &[
                ("__lib", "post_comment"),
                ("__act", "get"),
                ("pid", post_id.as_ref()),
                ("tid", topic_id.as_ref()),
                ("page", &page_str),
            ],
            &[],
        ).await?;

        parse_comments(&xml)
    }

    /// Create a reply to a topic.
    pub fn reply(&self, topic_id: impl Into<TopicId>) -> ReplyBuilder {
        ReplyBuilder {
            client: self.client.clone(),
            topic_id: topic_id.into(),
            content: String::new(),
            quote_post_id: None,
            attachments: Vec::new(),
            anonymous: false,
        }
    }

    /// Create a comment on a post.
    pub fn comment(&self, topic_id: impl Into<TopicId>, post_id: impl Into<PostId>) -> CommentBuilder {
        CommentBuilder {
            client: self.client.clone(),
            topic_id: topic_id.into(),
            post_id: post_id.into(),
            content: String::new(),
        }
    }

    /// Fetch content for quoting a post.
    pub async fn fetch_quote_content(
        &self,
        topic_id: impl AsRef<str>,
        post_id: impl AsRef<str>,
    ) -> Result<String> {
        let xml = self.client.post_authed(
            "post.php",
            &[
                ("action", "quote"),
                ("tid", topic_id.as_ref()),
                ("pid", post_id.as_ref()),
            ],
            &[],
        ).await?;

        let doc = XmlDocument::parse(&xml)?;
        let content = doc.string_opt("/root/content").unwrap_or_default();
        
        Ok(html_escape::decode_html_entities(&content).into_owned())
    }

    /// Get posts by a specific user.
    pub async fn by_user(&self, user_id: impl AsRef<str>, page: u32) -> Result<UserPostsResult> {
        let page_str = page.to_string();
        
        let xml = self.client.post(
            "thread.php",
            &[
                ("searchpost", "1"),
                ("authorid", user_id.as_ref()),
                ("page", &page_str),
            ],
            &[],
        ).await?;

        parse_user_posts(&xml)
    }
}

/// Result of a vote operation.
#[derive(Debug, Clone)]
pub struct VoteResult {
    /// Updated vote state.
    pub state: VoteState,
}

/// Result of a comments request.
#[derive(Debug, Clone, Default)]
pub struct CommentsResult {
    /// Comments on the post.
    pub comments: Vec<LightPost>,
    /// Total number of pages.
    pub total_pages: u32,
    /// Current page.
    pub page: u32,
}

/// Result of user posts request.
#[derive(Debug, Clone, Default)]
pub struct UserPostsResult {
    /// Posts by the user.
    pub posts: Vec<UserPost>,
    /// Total number of pages.
    pub total_pages: u32,
    /// Current page.
    pub page: u32,
}

/// A post in user's post history.
#[derive(Debug, Clone)]
pub struct UserPost {
    /// Post ID.
    pub post_id: PostId,
    /// Topic ID.
    pub topic_id: TopicId,
    /// Topic subject.
    pub topic_subject: String,
    /// Post content preview.
    pub content_preview: String,
    /// Post time.
    pub post_date: i64,
}

/// Builder for reply posts.
pub struct ReplyBuilder {
    client: Arc<NGAClientInner>,
    topic_id: TopicId,
    content: String,
    quote_post_id: Option<PostId>,
    attachments: Vec<String>,
    anonymous: bool,
}

impl ReplyBuilder {
    /// Set the reply content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Quote an existing post.
    pub fn quote(mut self, post_id: impl Into<PostId>) -> Self {
        self.quote_post_id = Some(post_id.into());
        self
    }

    /// Add an attachment.
    pub fn attachment(mut self, attachment_id: impl Into<String>) -> Self {
        self.attachments.push(attachment_id.into());
        self
    }

    /// Post anonymously.
    pub fn anonymous(mut self, anon: bool) -> Self {
        self.anonymous = anon;
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<ReplyResult> {
        if self.content.trim().is_empty() {
            return Err(Error::InvalidArgument("Reply content cannot be empty".into()));
        }

        let action = if self.quote_post_id.is_some() { "quote" } else { "reply" };
        let anon = if self.anonymous { "1" } else { "" };
        let attachments = self.attachments.join(",");
        let quote_pid = self.quote_post_id.as_ref().map(|p| p.as_str()).unwrap_or("");

        let xml = self.client.post_authed(
            "post.php",
            &[("action", action)],
            &[
                ("tid", self.topic_id.as_str()),
                ("pid", quote_pid),
                ("post_content", &self.content),
                ("attachs", &attachments),
                ("anony", anon),
            ],
        ).await?;

        let doc = XmlDocument::parse(&xml)?;
        let result = doc.string_opt("/root/data/item[1]");
        
        if let Some(pid) = result {
            Ok(ReplyResult {
                post_id: PostId::new(pid),
            })
        } else {
            let error = doc.string_opt("/root/data/__MESSAGE")
                .or_else(|| doc.string_opt("/root/__MESSAGE"))
                .unwrap_or_else(|| "Unknown error".to_owned());
            Err(Error::nga("post", error))
        }
    }
}

/// Result of a reply post.
#[derive(Debug, Clone)]
pub struct ReplyResult {
    /// ID of the new post.
    pub post_id: PostId,
}

/// Builder for comment posts.
pub struct CommentBuilder {
    client: Arc<NGAClientInner>,
    topic_id: TopicId,
    post_id: PostId,
    content: String,
}

impl CommentBuilder {
    /// Set the comment content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<CommentResult> {
        if self.content.trim().is_empty() {
            return Err(Error::InvalidArgument("Comment content cannot be empty".into()));
        }

        let xml = self.client.post_authed(
            "nuke.php",
            &[
                ("__lib", "post_comment"),
                ("__act", "add"),
            ],
            &[
                ("tid", self.topic_id.as_str()),
                ("pid", self.post_id.as_str()),
                ("content", &self.content),
            ],
        ).await?;

        let doc = XmlDocument::parse(&xml)?;
        let result = doc.string_opt("/root/data");
        
        if result.is_some() {
            Ok(CommentResult { success: true })
        } else {
            let error = doc.string_opt("/root/__MESSAGE")
                .unwrap_or_else(|| "Unknown error".to_owned());
            Err(Error::nga("comment", error))
        }
    }
}

/// Result of a comment post.
#[derive(Debug, Clone)]
pub struct CommentResult {
    /// Whether the comment was posted successfully.
    pub success: bool,
}

// ============================================================================
// Parsing helpers
// ============================================================================

fn parse_hot_replies(xml: &str) -> Result<Vec<LightPost>> {
    let doc = XmlDocument::parse(xml)?;
    let mut replies = Vec::new();

    for node in doc.select("/root/data/item")? {
        if let Some(reply) = parse_light_post(&node)? {
            replies.push(reply);
        }
    }

    Ok(replies)
}

fn parse_comments(xml: &str) -> Result<CommentsResult> {
    let doc = XmlDocument::parse(xml)?;
    let mut comments = Vec::new();

    for node in doc.select("/root/data/item")? {
        if let Some(comment) = parse_light_post(&node)? {
            comments.push(comment);
        }
    }

    let total_pages = doc.int_or("/root/__ROWS", 0) as u32;
    let total_pages = if total_pages > 0 { (total_pages + 19) / 20 } else { 1 };

    Ok(CommentsResult {
        comments,
        total_pages,
        page: 1,
    })
}

fn parse_light_post(node: &crate::parser::XmlNode<'_>) -> Result<Option<LightPost>> {
    let attrs = node.attrs();

    let id = match attrs.get("pid") {
        Some(pid) => pid.clone(),
        None => return Ok(None),
    };

    let author_id = attrs.get("authorid").cloned().unwrap_or_default();
    let author = User {
        id: author_id.into(),
        name: attrs.get("author")
            .map(|s| UserName::parse(s))
            .unwrap_or_default(),
        ..Default::default()
    };

    let content_raw = attrs.get("content").cloned().unwrap_or_default();
    let content = parse_content(&content_raw);

    Ok(Some(LightPost {
        id: id.into(),
        author,
        content,
        post_date: attrs.get("postdate").and_then(|s| s.parse().ok()).unwrap_or(0),
        score: attrs.get("score").and_then(|s| s.parse().ok()).unwrap_or(0),
    }))
}

fn parse_user_posts(xml: &str) -> Result<UserPostsResult> {
    let doc = XmlDocument::parse(xml)?;
    let mut posts = Vec::new();

    for node in doc.select("/root/__T/item")? {
        let attrs = node.attrs();
        
        if let (Some(tid), Some(pid)) = (attrs.get("tid"), attrs.get("pid")) {
            posts.push(UserPost {
                post_id: pid.clone().into(),
                topic_id: tid.clone().into(),
                topic_subject: attrs.get("subject").cloned().unwrap_or_default(),
                content_preview: attrs.get("content").cloned().unwrap_or_default(),
                post_date: attrs.get("postdate").and_then(|s| s.parse().ok()).unwrap_or(0),
            });
        }
    }

    let total_rows = doc.int_or("/root/__ROWS", 0) as u32;
    let total_pages = if total_rows > 0 { (total_rows + 34) / 35 } else { 1 };

    Ok(UserPostsResult {
        posts,
        total_pages,
        page: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vote_param() {
        assert_eq!(Vote::Up.param(), "1");
        assert_eq!(Vote::Down.param(), "0");
    }
}
