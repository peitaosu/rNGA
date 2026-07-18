//! Message handlers.

use anyhow::{Context, Result};
use colored::Colorize;
use rnga::NGAClient;
use rust_i18n::t;
use serde::Serialize;

use crate::handlers::meta::ResponseMeta;
use crate::output::{format_relative_time, PlainPrint, TableRow};

/// Message conversation info.
#[derive(Debug, Clone, Serialize)]
pub struct MessageInfo {
    pub id: String,
    pub other_user: String,
    pub other_uid: String,
    pub subject: String,
    pub last_time: i64,
    pub is_unread: bool,
}

impl TableRow for MessageInfo {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "With", "Subject", "Last", "Unread"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.other_user.clone(),
            self.subject.clone(),
            format_relative_time(self.last_time),
            if self.is_unread {
                "●".to_string()
            } else {
                "".to_string()
            },
        ]
    }
}

impl PlainPrint for MessageInfo {
    fn plain_print(&self) {
        let unread_marker = if self.is_unread {
            "● ".red().to_string()
        } else {
            String::new()
        };
        println!(
            "{}[{}] {} {}",
            unread_marker,
            self.id.cyan(),
            self.other_user.green(),
            format_relative_time(self.last_time).dimmed()
        );
        println!("   {}", self.subject.bold());
    }
}

/// Message list result.
#[derive(Debug, Clone, Serialize)]
pub struct MessageListResult {
    pub page: u32,
    pub total_pages: u32,
    pub conversations: Vec<MessageInfo>,
    pub meta: ResponseMeta,
}

/// Message post info.
#[derive(Debug, Clone, Serialize)]
pub struct MessagePostInfo {
    pub id: String,
    pub from: String,
    pub from_uid: String,
    pub is_mine: bool,
    pub content: String,
    pub time: i64,
}

impl TableRow for MessagePostInfo {
    fn headers() -> Vec<&'static str> {
        vec!["From", "Content", "Time"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.from.clone(),
            self.content.clone(),
            format_relative_time(self.time),
        ]
    }
}

impl PlainPrint for MessagePostInfo {
    fn plain_print(&self) {
        let from_display = if self.is_mine {
            t!("you_label").to_string().green().to_string()
        } else {
            self.from.clone()
        };
        println!(
            "{} {}",
            from_display,
            format_relative_time(self.time).dimmed()
        );
        for line in self.content.lines() {
            if !line.trim().is_empty() {
                println!("   {}", line);
            }
        }
        println!();
    }
}

/// Conversation result.
#[derive(Debug, Clone, Serialize)]
pub struct ConversationResult {
    pub mid: String,
    pub other_username: String,
    pub page: u32,
    pub total_pages: u32,
    pub messages: Vec<MessagePostInfo>,
    pub meta: ResponseMeta,
}

/// Send message result.
#[derive(Debug, Clone, Serialize)]
pub struct SendMessageResult {
    pub to: String,
    pub success: bool,
}

/// Reply message result.
#[derive(Debug, Clone, Serialize)]
pub struct ReplyMessageResult {
    pub mid: String,
    pub success: bool,
}

/// List message conversations.
pub async fn list_conversations(client: &NGAClient, page: u32) -> Result<MessageListResult> {
    let result = client
        .messages()
        .list(page)
        .await
        .context("listing message conversations")?;
    Ok(MessageListResult {
        page,
        total_pages: result.total_pages,
        conversations: result
            .conversations
            .iter()
            .map(|m| MessageInfo {
                id: m.id.clone(),
                other_user: m.other_username.clone(),
                other_uid: m.other_user_id.to_string(),
                subject: m.subject.clone(),
                last_time: m.last_time,
                is_unread: m.is_unread,
            })
            .collect(),
        meta: ResponseMeta::page_only(page, result.total_pages),
    })
}

/// Read a conversation.
pub async fn read_conversation(
    client: &NGAClient,
    mid: &str,
    page: u32,
) -> Result<ConversationResult> {
    let result = client
        .messages()
        .conversation(mid)
        .page(page)
        .send()
        .await
        .context("reading message conversation")?;
    Ok(ConversationResult {
        mid: mid.to_string(),
        other_username: result.other_username,
        page,
        total_pages: result.total_pages,
        messages: result
            .messages
            .iter()
            .map(|p| MessagePostInfo {
                id: p.id.clone(),
                from: p.from_username.clone(),
                from_uid: p.from_user_id.to_string(),
                is_mine: p.is_mine,
                content: p.content.to_plain_text(),
                time: p.time,
            })
            .collect(),
        meta: ResponseMeta::page_only(page, result.total_pages),
    })
}

/// Send a new message.
pub async fn send_message(
    client: &NGAClient,
    to: &str,
    subject: &str,
    content: &str,
) -> Result<SendMessageResult> {
    client
        .messages()
        .send_new()
        .to(to)
        .subject(subject)
        .content(content)
        .send()
        .await
        .context("sending message")?;

    Ok(SendMessageResult {
        to: to.to_string(),
        success: true,
    })
}

/// Send or reply to a message using shared options.
pub async fn send_with_options(
    client: &NGAClient,
    options: crate::handlers::options::SendMessageOptions,
) -> Result<SendMessageResult> {
    if let Some(mid) = options.reply_mid {
        reply_message(client, &mid, &options.content).await?;
        Ok(SendMessageResult {
            to: options.to,
            success: true,
        })
    } else {
        send_message(
            client,
            &options.to,
            &options.subject,
            &options.content,
        )
        .await
    }
}

/// Reply to a conversation.
pub async fn reply_message(
    client: &NGAClient,
    mid: &str,
    content: &str,
) -> Result<ReplyMessageResult> {
    client.messages().reply(mid).content(content).send().await.context("replying to message")?;

    Ok(ReplyMessageResult {
        mid: mid.to_string(),
        success: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_post_info_keeps_username_in_from_field() {
        let info = MessagePostInfo {
            id: "1".into(),
            from: "alice".into(),
            from_uid: "123".into(),
            is_mine: true,
            content: "hello".into(),
            time: 0,
        };
        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        assert_eq!(json["from"], "alice");
        assert_eq!(json["is_mine"], true);
    }
}
