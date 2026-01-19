//! Notification commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use rust_i18n::t;

use crate::config::build_authed_client;
use crate::handlers::notification as handlers;
use crate::output::{print_table, OutputFormat};

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

pub async fn handle(
    action: NotificationAction,
    format: OutputFormat,
    _verbose: bool,
) -> Result<()> {
    match action {
        NotificationAction::Counts => show_counts(format).await,
        NotificationAction::List { kind, page } => list_notifications(&kind, page, format).await,
        NotificationAction::Read { id } => mark_read(&id).await,
        NotificationAction::ReadAll { kind } => mark_all_read(&kind).await,
    }
}

async fn show_counts(format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let counts = handlers::get_counts(&client).await?;

    print_table(counts.to_rows(), format);

    if matches!(format, OutputFormat::Plain) {
        if counts.total > 0 {
            println!(
                "\n{}",
                t!("total_unread", count = counts.total)
                    .to_string()
                    .yellow()
            );
        }
    }

    Ok(())
}

async fn list_notifications(kind: &str, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::list_notifications(&client, kind, page).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{}\n",
            t!(
                "notifications_list",
                kind = result.kind,
                page = page,
                total = result.total_pages
            )
        );
    }

    print_table(result.notifications, format);
    Ok(())
}

async fn mark_read(id: &str) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::mark_read(&client, id).await?;

    println!("{}", t!("marked_notification_read", id = result.id));
    Ok(())
}

async fn mark_all_read(kind: &str) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::mark_all_read(&client, kind).await?;

    println!(
        "{}",
        t!("marked_all_notifications_read", kind = result.kind)
    );
    Ok(())
}
