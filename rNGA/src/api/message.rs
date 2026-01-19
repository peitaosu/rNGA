//! Message API.

use std::sync::Arc;

use crate::{
    client::NGAClientInner,
    error::{Error, Result},
    models::{ShortMessage, ShortMessagePost, UserId},
    parser::{parse_content, XmlDocument},
};

/// API for short message operations.
pub struct MessageApi {
    client: Arc<NGAClientInner>,
}

impl MessageApi {
    pub(crate) fn new(client: Arc<NGAClientInner>) -> Self {
        Self { client }
    }

    /// Get list of message conversations.
    pub async fn list(&self, page: u32) -> Result<MessageListResult> {
        let page_str = page.to_string();

        let xml = self
            .client
            .post_authed(
                "nuke.php",
                &[("__lib", "pm"), ("__act", "list"), ("page", &page_str)],
                &[],
            )
            .await?;

        parse_message_list(&xml, self.client.auth.as_ref().map(|a| a.uid.as_str()))
    }

    /// Get messages in a conversation.
    pub fn conversation(&self, mid: impl Into<String>) -> ConversationBuilder {
        ConversationBuilder {
            client: self.client.clone(),
            mid: mid.into(),
            page: 1,
        }
    }

    /// Send a new message.
    pub fn send_new(&self) -> SendMessageBuilder {
        SendMessageBuilder {
            client: self.client.clone(),
            to_username: String::new(),
            subject: String::new(),
            content: String::new(),
            reply_mid: None,
        }
    }

    /// Reply to an existing conversation.
    pub fn reply(&self, mid: impl Into<String>) -> SendMessageBuilder {
        SendMessageBuilder {
            client: self.client.clone(),
            to_username: String::new(),
            subject: String::new(),
            content: String::new(),
            reply_mid: Some(mid.into()),
        }
    }
}

/// Result of message list request.
#[derive(Debug, Clone, Default)]
pub struct MessageListResult {
    /// Conversations.
    pub conversations: Vec<ShortMessage>,
    /// Total pages.
    pub total_pages: u32,
    /// Current page.
    pub page: u32,
}

/// Builder for conversation message requests.
pub struct ConversationBuilder {
    client: Arc<NGAClientInner>,
    mid: String,
    page: u32,
}

impl ConversationBuilder {
    /// Set the page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<ConversationResult> {
        let page_str = self.page.to_string();

        let xml = self
            .client
            .post_authed(
                "nuke.php",
                &[
                    ("__lib", "pm"),
                    ("__act", "read"),
                    ("mid", &self.mid),
                    ("page", &page_str),
                ],
                &[],
            )
            .await?;

        let current_uid = self.client.auth.as_ref().map(|a| a.uid.as_str());
        parse_conversation(&xml, current_uid)
    }
}

/// Result of conversation request.
#[derive(Debug, Clone, Default)]
pub struct ConversationResult {
    /// Messages in conversation.
    pub messages: Vec<ShortMessagePost>,
    /// Other participant username.
    pub other_username: String,
    /// Other participant user ID.
    pub other_user_id: UserId,
    /// Total pages.
    pub total_pages: u32,
    /// Current page.
    pub page: u32,
}

/// Builder for sending messages.
pub struct SendMessageBuilder {
    client: Arc<NGAClientInner>,
    to_username: String,
    subject: String,
    content: String,
    reply_mid: Option<String>,
}

impl SendMessageBuilder {
    /// Set the recipient username.
    pub fn to(mut self, username: impl Into<String>) -> Self {
        self.to_username = username.into();
        self
    }

    /// Set the message subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Set the message content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<()> {
        if self.content.trim().is_empty() {
            return Err(Error::InvalidArgument(
                "Message content cannot be empty".into(),
            ));
        }

        let is_reply = self.reply_mid.is_some();

        if !is_reply && self.to_username.is_empty() {
            return Err(Error::InvalidArgument(
                "Recipient username is required".into(),
            ));
        }

        let mut form = vec![
            ("to", self.to_username.as_str()),
            ("subject", self.subject.as_str()),
            ("content", self.content.as_str()),
        ];

        if let Some(ref mid) = self.reply_mid {
            form.push(("mid", mid.as_str()));
        }

        self.client
            .post_authed(
                "nuke.php",
                &[
                    ("__lib", "pm"),
                    ("__act", if is_reply { "reply" } else { "send" }),
                ],
                &form,
            )
            .await?;

        Ok(())
    }
}

fn parse_message_list(xml: &str, current_uid: Option<&str>) -> Result<MessageListResult> {
    let doc = XmlDocument::parse(xml)?;
    let mut conversations = Vec::new();

    for node in doc.select("/root/data/item")? {
        if let Some(conv) = parse_conversation_item(&node, current_uid)? {
            conversations.push(conv);
        }
    }

    let total_rows = doc.int_or("/root/__ROWS", 0) as u32;
    let total_pages = if total_rows > 0 {
        (total_rows + 19) / 20
    } else {
        1
    };

    Ok(MessageListResult {
        conversations,
        total_pages,
        page: 1,
    })
}

fn parse_conversation_item(
    node: &crate::parser::XmlNode<'_>,
    current_uid: Option<&str>,
) -> Result<Option<ShortMessage>> {
    let attrs = node.attrs();

    let id = match attrs.get("mid") {
        Some(mid) => mid.clone(),
        None => return Ok(None),
    };

    let from_uid = attrs.get("from_uid").cloned().unwrap_or_default();
    let to_uid = attrs.get("to_uid").cloned().unwrap_or_default();
    let from_username = attrs.get("from_username").cloned().unwrap_or_default();
    let to_username = attrs.get("to_username").cloned().unwrap_or_default();

    let (other_user_id, other_username) = if Some(from_uid.as_str()) == current_uid {
        (to_uid, to_username)
    } else {
        (from_uid, from_username)
    };

    let conv = ShortMessage {
        id,
        other_user_id: other_user_id.into(),
        other_username,
        subject: attrs.get("subject").cloned().unwrap_or_default(),
        last_time: attrs.get("time").and_then(|s| s.parse().ok()).unwrap_or(0),
        is_unread: attrs.get("bit").map(|s| s == "1").unwrap_or(false),
        message_count: attrs.get("count").and_then(|s| s.parse().ok()).unwrap_or(1),
    };

    Ok(Some(conv))
}

fn parse_conversation(xml: &str, current_uid: Option<&str>) -> Result<ConversationResult> {
    let doc = XmlDocument::parse(xml)?;
    let mut messages = Vec::new();

    let other_username = doc.string_opt("/root/__P/0/username").unwrap_or_default();
    let other_user_id = doc
        .string_opt("/root/__P/0/uid")
        .map(|s| s.into())
        .unwrap_or_default();

    for node in doc.select("/root/data/item")? {
        if let Some(msg) = parse_message_post(&node, current_uid)? {
            messages.push(msg);
        }
    }

    let total_rows = doc.int_or("/root/__ROWS", 0) as u32;
    let total_pages = if total_rows > 0 {
        (total_rows + 19) / 20
    } else {
        1
    };

    Ok(ConversationResult {
        messages,
        other_username,
        other_user_id,
        total_pages,
        page: 1,
    })
}

fn parse_message_post(
    node: &crate::parser::XmlNode<'_>,
    current_uid: Option<&str>,
) -> Result<Option<ShortMessagePost>> {
    let attrs = node.attrs();

    let id = match attrs.get("id") {
        Some(id) => id.clone(),
        None => return Ok(None),
    };

    let from_user_id = attrs.get("from_uid").cloned().unwrap_or_default();
    let is_mine = Some(from_user_id.as_str()) == current_uid;

    let content_raw = attrs.get("content").cloned().unwrap_or_default();
    let content = parse_content(&content_raw);

    let msg = ShortMessagePost {
        id,
        from_user_id: from_user_id.into(),
        from_username: attrs.get("from_username").cloned().unwrap_or_default(),
        content,
        time: attrs.get("time").and_then(|s| s.parse().ok()).unwrap_or(0),
        is_mine,
    };

    Ok(Some(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_builder() {
        let _ = SendMessageBuilder {
            client: Arc::new(crate::client::NGAClientInner {
                http: reqwest::Client::new(),
                config: crate::client::HttpConfig::default(),
                auth: None,
                cache: None,
            }),
            to_username: "test".into(),
            subject: "Hello".into(),
            content: "Hi".into(),
            reply_mid: None,
        };
    }
}
