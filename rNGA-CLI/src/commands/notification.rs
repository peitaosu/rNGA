//! Notification commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use rnga::models::*;

use crate::config::build_authed_client;
use crate::output::{print_table, NotificationCountRow, NotificationRow, OutputFormat};

#[derive(Subcommand)]
pub enum NotificationAction {
    /// Show unread notification counts
    Counts,

    /// List notifications
    #[command(alias = "ls")]
    List {
        /// Type: reply, quote, mention, comment, system
        #[arg(short, long, default_value = "reply")]
        kind: String,
        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,
    },

    /// Mark notification as read
    Read {
        /// Notification ID
        id: String,
    },

    /// Mark all notifications of a type as read
    ReadAll {
        /// Type: reply, quote, mention, comment, system
        #[arg(short, long, default_value = "reply")]
        kind: String,
    },
}

pub async fn handle(action: NotificationAction, format: OutputFormat, _verbose: bool) -> Result<()> {
    match action {
        NotificationAction::Counts => show_counts(format).await,
        NotificationAction::List { kind, page } => list_notifications(&kind, page, format).await,
        NotificationAction::Read { id } => mark_read(&id).await,
        NotificationAction::ReadAll { kind } => mark_all_read(&kind).await,
    }
}

async fn show_counts(format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let counts = client.notifications().counts().await?;

    let rows = vec![
        NotificationCountRow {
            kind: "Replies".to_string(),
            count: counts.replies,
        },
        NotificationCountRow {
            kind: "Quotes".to_string(),
            count: counts.quotes,
        },
        NotificationCountRow {
            kind: "Mentions".to_string(),
            count: counts.mentions,
        },
        NotificationCountRow {
            kind: "Comments".to_string(),
            count: counts.comments,
        },
        NotificationCountRow {
            kind: "System".to_string(),
            count: counts.system,
        },
        NotificationCountRow {
            kind: "Messages".to_string(),
            count: counts.messages,
        },
    ];

    print_table(rows, format);

    if matches!(format, OutputFormat::Plain) {
        let total = counts.total();
        if total > 0 {
            println!("\n{}", format!("Total unread: {}", total).yellow());
        }
    }

    Ok(())
}

fn parse_notification_type(kind: &str) -> NotificationType {
    match kind.to_lowercase().as_str() {
        "reply" | "replies" => NotificationType::Reply,
        "quote" | "quotes" => NotificationType::Quote,
        "mention" | "mentions" | "at" => NotificationType::Mention,
        "comment" | "comments" => NotificationType::Comment,
        "system" => NotificationType::System,
        "message" | "messages" | "pm" => NotificationType::Message,
        _ => NotificationType::Reply,
    }
}

async fn list_notifications(kind: &str, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let noti_type = parse_notification_type(kind);

    let result = client
        .notifications()
        .list(noti_type)
        .page(page)
        .send()
        .await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{:?} notifications (page {}/{})\n",
            noti_type, page, result.total_pages
        );
    }

    let rows: Vec<NotificationRow> = result.notifications.iter().map(NotificationRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn mark_read(id: &str) -> Result<()> {
    let client = build_authed_client()?;
    client.notifications().mark_read(id).await?;

    println!("{} Marked notification {} as read", "✓".green(), id);

    Ok(())
}

async fn mark_all_read(kind: &str) -> Result<()> {
    let client = build_authed_client()?;
    let noti_type = parse_notification_type(kind);

    client.notifications().mark_all_read(noti_type).await?;

    println!(
        "{} Marked all {:?} notifications as read",
        "✓".green(),
        noti_type
    );

    Ok(())
}

