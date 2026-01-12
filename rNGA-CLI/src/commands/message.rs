//! Message commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;

use crate::config::build_authed_client;
use crate::output::{print_table, MessagePostRow, MessageRow, OutputFormat};

#[derive(Subcommand)]
pub enum MessageAction {
    /// List message conversations
    #[command(alias = "ls")]
    List {
        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,
    },

    /// View messages in a conversation
    Read {
        /// Conversation/message ID
        mid: String,
        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,
    },

    /// Send a new message
    Send {
        /// Recipient username
        #[arg(short, long)]
        to: String,
        /// Message subject
        #[arg(short, long)]
        subject: String,
        /// Message content
        content: String,
    },

    /// Reply to a conversation
    Reply {
        /// Conversation/message ID
        mid: String,
        /// Reply content
        content: String,
    },
}

pub async fn handle(action: MessageAction, format: OutputFormat, verbose: bool) -> Result<()> {
    match action {
        MessageAction::List { page } => list_conversations(page, format).await,
        MessageAction::Read { mid, page } => read_conversation(&mid, page, format, verbose).await,
        MessageAction::Send {
            to,
            subject,
            content,
        } => send_message(&to, &subject, &content).await,
        MessageAction::Reply { mid, content } => reply_message(&mid, &content).await,
    }
}

async fn list_conversations(page: u32, format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let result = client.messages().list(page).await?;

    if matches!(format, OutputFormat::Plain) {
        println!("Conversations (page {}/{})\n", page, result.total_pages);
    }

    let rows: Vec<MessageRow> = result.conversations.iter().map(MessageRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn read_conversation(
    mid: &str,
    page: u32,
    format: OutputFormat,
    _verbose: bool,
) -> Result<()> {
    let client = build_authed_client()?;
    let result = client.messages().conversation(mid).page(page).send().await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "Conversation with {} (page {}/{})\n",
            result.other_username.green(),
            page,
            result.total_pages
        );
    }

    let rows: Vec<MessagePostRow> = result.messages.iter().map(MessagePostRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn send_message(to: &str, subject: &str, content: &str) -> Result<()> {
    let client = build_authed_client()?;

    client
        .messages()
        .send_new()
        .to(to)
        .subject(subject)
        .content(content)
        .send()
        .await?;

    println!("{} Message sent to {}", "✓".green(), to);

    Ok(())
}

async fn reply_message(mid: &str, content: &str) -> Result<()> {
    let client = build_authed_client()?;

    client.messages().reply(mid).content(content).send().await?;

    println!("{} Reply sent", "✓".green());

    Ok(())
}

