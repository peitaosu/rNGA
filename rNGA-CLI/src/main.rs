//! NGA CLI and MCP Server.

mod commands;
mod config;
mod mcp;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{forum, message, notification, post, topic, user};

/// NGA Forum CLI and MCP Server
#[derive(Parser)]
#[command(name = "rnga")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Output format
    #[arg(short, long, global = true, default_value = "plain")]
    format: output::OutputFormat,

    /// Show verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Run as MCP Server
    #[arg(long)]
    mcp: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },

    /// Forum operations
    #[command(alias = "f")]
    Forum {
        #[command(subcommand)]
        action: forum::ForumAction,
    },

    /// Topic operations
    #[command(alias = "t")]
    Topic {
        #[command(subcommand)]
        action: topic::TopicAction,
    },

    /// Post operations
    #[command(alias = "p")]
    Post {
        #[command(subcommand)]
        action: post::PostAction,
    },

    /// User operations
    #[command(alias = "u")]
    User {
        #[command(subcommand)]
        action: user::UserAction,
    },

    /// Notification operations
    #[command(alias = "n")]
    Notification {
        #[command(subcommand)]
        action: notification::NotificationAction,
    },

    /// Message operations
    #[command(alias = "m")]
    Message {
        #[command(subcommand)]
        action: message::MessageAction,
    },

    /// Show current configuration
    Config,
}

#[derive(Subcommand)]
enum AuthAction {
    /// Login with token and uid
    Login {
        /// Access token
        #[arg(short, long)]
        token: String,
        /// User ID
        #[arg(short, long)]
        uid: String,
    },
    /// Logout
    Logout,
    /// Show current auth status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.mcp {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(tracing::Level::INFO.into()),
            )
            .with_writer(std::io::stderr)
            .init();

        return mcp::run_server().await;
    }

    let command = cli.command.ok_or_else(|| {
        anyhow::anyhow!("No command provided. Use --help for usage or --mcp to run as MCP server.")
    })?;

    match command {
        Commands::Auth { action } => handle_auth(action).await,
        Commands::Forum { action } => forum::handle(action, cli.format, cli.verbose).await,
        Commands::Topic { action } => topic::handle(action, cli.format, cli.verbose).await,
        Commands::Post { action } => post::handle(action, cli.format, cli.verbose).await,
        Commands::User { action } => user::handle(action, cli.format, cli.verbose).await,
        Commands::Notification { action } => {
            notification::handle(action, cli.format, cli.verbose).await
        }
        Commands::Message { action } => message::handle(action, cli.format, cli.verbose).await,
        Commands::Config => {
            let cfg = config::load_config()?;
            println!("Config file: {}", config::config_path()?.display());
            println!("Authenticated: {}", cfg.auth.is_some());
            if let Some(auth) = &cfg.auth {
                println!("User ID: {}", auth.uid);
            }
            Ok(())
        }
    }
}

async fn handle_auth(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Login { token, uid } => {
            let mut cfg = config::load_config()?;
            cfg.auth = Some(config::AuthConfig {
                token: token.clone(),
                uid: uid.clone(),
            });
            config::save_config(&cfg)?;
            println!("Logged in as user {}", uid);
            Ok(())
        }
        AuthAction::Logout => {
            let mut cfg = config::load_config()?;
            cfg.auth = None;
            config::save_config(&cfg)?;
            println!("Logged out");
            Ok(())
        }
        AuthAction::Status => {
            let cfg = config::load_config()?;
            if let Some(auth) = &cfg.auth {
                println!("Logged in as user {}", auth.uid);
            } else {
                println!("Not logged in");
            }
            Ok(())
        }
    }
}

