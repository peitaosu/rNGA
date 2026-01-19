//! Message commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use rust_i18n::t;

use crate::config::build_authed_client;
use crate::handlers::message as handlers;
use crate::output::{print_table, OutputFormat};

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

pub async fn handle(action: MessageAction, format: OutputFormat, _verbose: bool) -> Result<()> {
    match action {
        MessageAction::List { page } => list_conversations(page, format).await,
        MessageAction::Read { mid, page } => read_conversation(&mid, page, format).await,
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
    let result = handlers::list_conversations(&client, page).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{}\n",
            t!("conversations", page = page, total = result.total_pages)
        );
    }

    print_table(result.conversations, format);
    Ok(())
}

async fn read_conversation(mid: &str, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::read_conversation(&client, mid, page).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{}\n",
            t!(
                "conversation_with",
                user = result.other_username.green(),
                page = page,
                total = result.total_pages
            )
        );
    }

    print_table(result.messages, format);
    Ok(())
}

async fn send_message(to: &str, subject: &str, content: &str) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::send_message(&client, to, subject, content).await?;

    println!("{}", t!("message_sent_to", user = result.to));
    Ok(())
}

async fn reply_message(mid: &str, content: &str) -> Result<()> {
    let client = build_authed_client()?;
    handlers::reply_message(&client, mid, content).await?;

    println!("{}", t!("reply_sent"));
    Ok(())
}
