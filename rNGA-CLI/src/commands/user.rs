//! User commands.

use anyhow::Result;
use clap::Subcommand;

use crate::config::{build_authed_client, build_client};
use crate::output::{print_table, OutputFormat, TopicRow, UserRow, UserSearchRow, UserPostRow};

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

pub async fn handle(action: UserAction, format: OutputFormat, verbose: bool) -> Result<()> {
    match action {
        UserAction::Get { user_id } => get_user_by_id(&user_id, format, verbose).await,
        UserAction::Name { username } => get_user_by_name(&username, format, verbose).await,
        UserAction::Me => get_me(format, verbose).await,
        UserAction::Search { keyword } => search_users(&keyword, format).await,
        UserAction::Topics { user_id, page } => user_topics(&user_id, page, format).await,
        UserAction::Posts { user_id, page } => user_posts(&user_id, page, format).await,
    }
}

async fn get_user_by_id(user_id: &str, format: OutputFormat, verbose: bool) -> Result<()> {
    let client = build_client()?;
    let user = client.users().get(user_id).await?;

    print_user(&user, format, verbose);
    Ok(())
}

async fn get_user_by_name(username: &str, format: OutputFormat, verbose: bool) -> Result<()> {
    let client = build_client()?;
    let user = client.users().get_by_name(username).await?;

    print_user(&user, format, verbose);
    Ok(())
}

async fn get_me(format: OutputFormat, verbose: bool) -> Result<()> {
    let client = build_authed_client()?;
    let user = client.users().me().await?;

    print_user(&user, format, verbose);
    Ok(())
}

fn print_user(user: &rnga::User, format: OutputFormat, _verbose: bool) {
    let rows = vec![UserRow::from(user)];
    print_table(rows, format);
}

async fn search_users(keyword: &str, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let results = client.users().search(keyword).await?;

    if results.is_empty() {
        if matches!(format, OutputFormat::Plain) {
            println!("No users found");
        }
        return Ok(());
    }

    let rows: Vec<UserSearchRow> = results
        .iter()
        .map(|u| UserSearchRow {
            id: u.id.to_string(),
            name: u.name.clone(),
        })
        .collect();
    print_table(rows, format);

    Ok(())
}

async fn user_topics(user_id: &str, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let result = client.topics().by_user(user_id, page).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "Topics by user {} (page {}/{})\n",
            user_id, page, result.total_pages
        );
    }

    let rows: Vec<TopicRow> = result.topics.iter().map(TopicRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn user_posts(user_id: &str, page: u32, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let result = client.posts().by_user(user_id, page).await?;

    if matches!(format, OutputFormat::Plain) {
        println!(
            "Posts by user {} (page {}/{})\n",
            user_id, page, result.total_pages
        );
    }

    let rows: Vec<UserPostRow> = result
        .posts
        .iter()
        .map(|p| UserPostRow {
            post_id: p.post_id.to_string(),
            topic_id: p.topic_id.to_string(),
            topic_subject: p.topic_subject.clone(),
            content_preview: p.content_preview.clone(),
        })
        .collect();
    print_table(rows, format);

    Ok(())
}

