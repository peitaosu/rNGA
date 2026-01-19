//! User commands.

use anyhow::Result;
use clap::Subcommand;
use rust_i18n::t;

use crate::config::{build_authed_client, build_client};
use crate::handlers::user as handlers;
use crate::output::{print_table, OutputFormat};

#[derive(Subcommand)]
pub enum UserAction {
    /// View user profile by ID
    Get {
        /// User ID
        user_id: String,
    },

    /// View user profile by username
    Name {
        /// Username
        username: String,
    },

    /// View current user's profile
    Me,

    /// Search users
    Search {
        /// Search keyword
        keyword: String,
    },

    /// View user's topics
    Topics {
        /// User ID
        user_id: String,
        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,
    },

    /// View user's posts
    Posts {
        /// User ID
        user_id: String,
        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,
    },
}

pub async fn handle(action: UserAction, format: OutputFormat, _verbose: bool) -> Result<()> {
    match action {
        UserAction::Get { user_id } => get_user_by_id(&user_id, format).await,
        UserAction::Name { username } => get_user_by_name(&username, format).await,
        UserAction::Me => get_me(format).await,
        UserAction::Search { keyword } => search_users(&keyword, format).await,
        UserAction::Topics { user_id, page } => user_topics(&user_id, page, format).await,
        UserAction::Posts { user_id, page } => user_posts(&user_id, page, format).await,
    }
}

async fn get_user_by_id(user_id: &str, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let user = handlers::get_user(&client, user_id).await?;

    print_table(vec![user], format);
    Ok(())
}

async fn get_user_by_name(username: &str, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let user = handlers::get_user_by_name(&client, username).await?;

    print_table(vec![user], format);
    Ok(())
}

async fn get_me(format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let user = handlers::get_me(&client).await?;

    print_table(vec![user], format);
    Ok(())
}

async fn search_users(keyword: &str, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let results = handlers::search_users(&client, keyword).await?;

    if results.is_empty() {
        if matches!(format, OutputFormat::Plain) {
            println!("{}", t!("no_users_found"));
        }
        return Ok(());
    }

    print_table(results, format);
    Ok(())
}

async fn user_topics(user_id: &str, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let result = handlers::user_topics(&client, user_id, page).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{}\n",
            t!(
                "topics_by_user",
                uid = user_id,
                page = page,
                total = result.total_pages
            )
        );
    }

    print_table(result.topics, format);
    Ok(())
}

async fn user_posts(user_id: &str, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let result = handlers::user_posts(&client, user_id, page).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "{}\n",
            t!(
                "posts_by_user",
                uid = user_id,
                page = page,
                total = result.total_pages
            )
        );
    }

    print_table(result.posts, format);
    Ok(())
}
