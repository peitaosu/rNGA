//! Notification API.

use std::sync::Arc;

use crate::{
    client::NGAClientInner,
    error::Result,
    models::{Notification, NotificationCounts, NotificationType, PostId, TopicId, UserId},
    parser::XmlDocument,
};

/// API for notification operations.
pub struct NotificationApi {
    client: Arc<NGAClientInner>,
}

impl NotificationApi {
    pub(crate) fn new(client: Arc<NGAClientInner>) -> Self {
        Self { client }
    }

    /// Get unread notification counts.
    pub async fn counts(&self) -> Result<NotificationCounts> {
        let xml = self
            .client
            .post_authed(
                "nuke.php",
                &[("__lib", "noti"), ("__act", "get_all_unread")],
                &[],
            )
            .await?;

        parse_notification_counts(&xml)
    }

    /// Get notifications of a specific type.
    pub fn list(&self, kind: NotificationType) -> NotificationListBuilder {
        NotificationListBuilder {
            client: self.client.clone(),
            kind,
            page: 1,
        }
    }

    /// Mark notification as read.
    pub async fn mark_read(&self, notification_id: impl AsRef<str>) -> Result<()> {
        self.client
            .post_authed(
                "nuke.php",
                &[
                    ("__lib", "noti"),
                    ("__act", "read"),
                    ("id", notification_id.as_ref()),
                ],
                &[],
            )
            .await?;

        Ok(())
    }

    /// Mark all notifications of a type as read.
    pub async fn mark_all_read(&self, kind: NotificationType) -> Result<()> {
        self.client
            .post_authed(
                "nuke.php",
                &[
                    ("__lib", "noti"),
                    ("__act", "read_all"),
                    ("type", kind.param()),
                ],
                &[],
            )
            .await?;

        Ok(())
    }
}

/// Builder for notification list requests.
pub struct NotificationListBuilder {
    client: Arc<NGAClientInner>,
    kind: NotificationType,
    page: u32,
}

impl NotificationListBuilder {
    /// Set the page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<NotificationListResult> {
        let page_str = self.page.to_string();

        let xml = self
            .client
            .post_authed(
                "nuke.php",
                &[
                    ("__lib", "noti"),
                    ("__act", "get_list"),
                    ("type", self.kind.param()),
                    ("page", &page_str),
                ],
                &[],
            )
            .await?;

        parse_notification_list(&xml, self.kind)
    }
}

/// Result of a notification list request.
#[derive(Debug, Clone, Default)]
pub struct NotificationListResult {
    /// Notifications.
    pub notifications: Vec<Notification>,
    /// Total pages.
    pub total_pages: u32,
    /// Current page.
    pub page: u32,
}

fn parse_notification_counts(xml: &str) -> Result<NotificationCounts> {
    let doc = XmlDocument::parse(xml)?;

    Ok(NotificationCounts {
        replies: doc.int_or("/root/data/item/reply", 0) as i32,
        quotes: doc.int_or("/root/data/item/quote", 0) as i32,
        mentions: doc.int_or("/root/data/item/at", 0) as i32,
        comments: doc.int_or("/root/data/item/comment", 0) as i32,
        system: doc.int_or("/root/data/item/system", 0) as i32,
        messages: doc.int_or("/root/data/item/pm", 0) as i32,
    })
}

fn parse_notification_list(xml: &str, kind: NotificationType) -> Result<NotificationListResult> {
    let doc = XmlDocument::parse(xml)?;
    let mut notifications = Vec::new();

    for node in doc.select("/root/data/item")? {
        if let Some(noti) = parse_notification(&node, kind)? {
            notifications.push(noti);
        }
    }

    let total_rows = doc.int_or("/root/__ROWS", 0) as u32;
    let total_pages = if total_rows > 0 {
        (total_rows + 19) / 20
    } else {
        1
    };

    Ok(NotificationListResult {
        notifications,
        total_pages,
        page: 1,
    })
}

fn parse_notification(
    node: &crate::parser::XmlNode<'_>,
    kind: NotificationType,
) -> Result<Option<Notification>> {
    let attrs = node.attrs();

    let id = match attrs.get("id") {
        Some(id) => id.clone(),
        None => return Ok(None),
    };

    let content = attrs
        .get("0")
        .or_else(|| attrs.get("content"))
        .cloned()
        .unwrap_or_default();

    let (topic_id, post_id) = if let Some(url) = attrs.get("1").or(attrs.get("url")) {
        extract_ids_from_url(url)
    } else {
        (None, None)
    };

    let from_user_id = attrs
        .get("from_uid")
        .or_else(|| attrs.get("2"))
        .map(|s| UserId::new(s));
    let from_username = attrs
        .get("from_username")
        .or_else(|| attrs.get("3"))
        .cloned();

    let noti = Notification {
        id,
        kind,
        content: html_escape::decode_html_entities(&content).into_owned(),
        time: attrs
            .get("time")
            .or_else(|| attrs.get("4"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        is_read: false,
        topic_id,
        post_id,
        from_user_id,
        from_username,
    };

    Ok(Some(noti))
}

fn extract_ids_from_url(url: &str) -> (Option<TopicId>, Option<PostId>) {
    use lazy_static::lazy_static;
    use regex::Regex;

    lazy_static! {
        static ref TID_RE: Regex = Regex::new(r"tid=(\d+)").unwrap();
        static ref PID_RE: Regex = Regex::new(r"pid=(\d+)").unwrap();
    }

    let topic_id = TID_RE
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| TopicId::new(m.as_str()));

    let post_id = PID_RE
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| PostId::new(m.as_str()));

    (topic_id, post_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ids_from_url() {
        let url = "read.php?tid=12345&pid=67890";
        let (tid, pid) = extract_ids_from_url(url);

        assert_eq!(tid.unwrap().as_str(), "12345");
        assert_eq!(pid.unwrap().as_str(), "67890");
    }

    #[test]
    fn test_notification_type_param() {
        assert_eq!(NotificationType::Reply.param(), "reply");
        assert_eq!(NotificationType::Mention.param(), "at");
    }
}
