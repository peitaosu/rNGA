//! Notification handlers.

use anyhow::{Context, Result};
use colored::Colorize;
use rnga::models::*;
use rnga::NGAClient;
use rust_i18n::t;
use serde::Serialize;

use crate::handlers::meta::ResponseMeta;
use crate::output::{format_relative_time, PlainPrint, TableRow};

/// Notification counts.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationCountsInfo {
    pub replies: i32,
    pub quotes: i32,
    pub mentions: i32,
    pub comments: i32,
    pub system: i32,
    pub messages: i32,
    pub total: i32,
}

/// Notification count row for display.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationCountRow {
    pub kind: String,
    pub count: i32,
}

impl TableRow for NotificationCountRow {
    fn headers() -> Vec<&'static str> {
        vec!["Type", "Count"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.kind.clone(), self.count.to_string()]
    }
}

impl PlainPrint for NotificationCountRow {
    fn plain_print(&self) {
        let count_display = if self.count > 0 {
            self.count.to_string().yellow().to_string()
        } else {
            self.count.to_string().dimmed().to_string()
        };
        println!("{}: {}", self.kind, count_display);
    }
}

impl NotificationCountsInfo {
    /// Convert to display rows.
    pub fn to_rows(&self) -> Vec<NotificationCountRow> {
        vec![
            NotificationCountRow {
                kind: t!("notif_replies").to_string(),
                count: self.replies,
            },
            NotificationCountRow {
                kind: t!("notif_quotes").to_string(),
                count: self.quotes,
            },
            NotificationCountRow {
                kind: t!("notif_mentions").to_string(),
                count: self.mentions,
            },
            NotificationCountRow {
                kind: t!("notif_comments").to_string(),
                count: self.comments,
            },
            NotificationCountRow {
                kind: t!("notif_system").to_string(),
                count: self.system,
            },
            NotificationCountRow {
                kind: t!("notif_messages").to_string(),
                count: self.messages,
            },
        ]
    }
}

/// Notification info.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationInfo {
    pub id: String,
    pub kind: String,
    pub content: String,
    pub from: Option<String>,
    pub from_uid: Option<String>,
    pub time: i64,
    pub topic_id: Option<String>,
    pub post_id: Option<String>,
}

impl TableRow for NotificationInfo {
    fn headers() -> Vec<&'static str> {
        vec!["Type", "Content", "From", "Time"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.kind.clone(),
            self.content.clone(),
            self.from.clone().unwrap_or_default(),
            format_relative_time(self.time),
        ]
    }
}

impl PlainPrint for NotificationInfo {
    fn plain_print(&self) {
        let from_display = if let Some(from) = &self.from {
            format!(" {}", t!("from_user", user = from.green()))
        } else {
            String::new()
        };
        println!(
            "[{}]{} {}",
            self.kind.cyan(),
            from_display,
            format_relative_time(self.time).dimmed()
        );
        if !self.content.is_empty() {
            for line in self.content.lines() {
                if !line.trim().is_empty() {
                    println!("   {}", line);
                }
            }
        }
    }
}

/// Notification list result.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationListResult {
    pub kind: String,
    pub page: u32,
    pub total_pages: u32,
    pub notifications: Vec<NotificationInfo>,
    pub meta: ResponseMeta,
}

/// Mark read result.
#[derive(Debug, Clone, Serialize)]
pub struct MarkReadResult {
    pub id: String,
    pub success: bool,
}

/// Mark all read result.
#[derive(Debug, Clone, Serialize)]
pub struct MarkAllReadResult {
    pub kind: String,
    pub success: bool,
}

/// Parse notification type from string.
pub fn parse_notification_type(kind: &str) -> Result<NotificationType> {
    match kind.to_lowercase().as_str() {
        "reply" | "replies" => Ok(NotificationType::Reply),
        "quote" | "quotes" => Ok(NotificationType::Quote),
        "mention" | "mentions" | "at" => Ok(NotificationType::Mention),
        "comment" | "comments" => Ok(NotificationType::Comment),
        "system" => Ok(NotificationType::System),
        "message" | "messages" | "pm" => Ok(NotificationType::Message),
        other => Err(anyhow::anyhow!("Unknown notification type: {other}")),
    }
}

/// Get notification counts.
pub async fn get_counts(client: &NGAClient) -> Result<NotificationCountsInfo> {
    let counts = client
        .notifications()
        .counts()
        .await
        .context("fetching notification counts")?;
    Ok(NotificationCountsInfo {
        replies: counts.replies,
        quotes: counts.quotes,
        mentions: counts.mentions,
        comments: counts.comments,
        system: counts.system,
        messages: counts.messages,
        total: counts.total(),
    })
}

/// List notifications of a type.
pub async fn list_notifications(
    client: &NGAClient,
    kind: &str,
    page: u32,
) -> Result<NotificationListResult> {
    let noti_type = parse_notification_type(kind)?;
    let result = client
        .notifications()
        .list(noti_type)
        .page(page)
        .send()
        .await
        .context("listing notifications")?;

    Ok(NotificationListResult {
        kind: noti_type.param().to_string(),
        page,
        total_pages: result.total_pages,
        notifications: result
            .notifications
            .iter()
            .map(|n| NotificationInfo {
                id: n.id.clone(),
                kind: n.kind.param().to_string(),
                content: n.content.clone(),
                from: n.from_username.clone(),
                from_uid: n.from_user_id.as_ref().map(|u| u.to_string()),
                time: n.time,
                topic_id: n.topic_id.as_ref().map(|t| t.to_string()),
                post_id: n.post_id.as_ref().map(|p| p.to_string()),
            })
            .collect(),
        meta: ResponseMeta::page_only(page, result.total_pages),
    })
}

/// Mark a notification as read.
pub async fn mark_read(client: &NGAClient, id: &str) -> Result<MarkReadResult> {
    client
        .notifications()
        .mark_read(id)
        .await
        .context("marking notification read")?;
    Ok(MarkReadResult {
        id: id.to_string(),
        success: true,
    })
}

/// Mark all notifications of a type as read.
pub async fn mark_all_read(client: &NGAClient, kind: &str) -> Result<MarkAllReadResult> {
    let noti_type = parse_notification_type(kind)?;
    client
        .notifications()
        .mark_all_read(noti_type)
        .await
        .context("marking all notifications read")?;
    Ok(MarkAllReadResult {
        kind: format!("{:?}", noti_type),
        success: true,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_notification_type;
    use rnga::NotificationType;

    #[test]
    fn test_parse_notification_type_valid() {
        assert_eq!(parse_notification_type("reply").unwrap(), NotificationType::Reply);
        assert_eq!(parse_notification_type("at").unwrap(), NotificationType::Mention);
    }

    #[test]
    fn test_parse_notification_type_unknown() {
        assert!(parse_notification_type("invalid").is_err());
    }

    #[test]
    fn test_notification_kind_uses_api_name() {
        let noti_type = parse_notification_type("reply").unwrap();
        assert_eq!(noti_type.param(), "reply");
    }
}
