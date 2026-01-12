//! Forum commands.

use anyhow::Result;
use clap::Subcommand;
use rnga::models::*;

use crate::config::{build_authed_client, build_client};
use crate::output::{print_table, CategoryRow, ForumRow, OutputFormat};

#[derive(Subcommand)]
pub enum ForumAction {
    /// List all forum categories
    #[command(alias = "ls")]
    List,

    /// Search forums by name
    Search {
        /// Search keyword
        keyword: String,
    },

    /// List favorite forums
    Favorites,

    /// Add forum to favorites
    FavAdd {
        /// Forum ID
        id: String,
        /// Treat ID as stid instead of fid
        #[arg(short, long)]
        stid: bool,
    },

    /// Remove forum from favorites
    FavRemove {
        /// Forum ID
        id: String,
        /// Treat ID as stid instead of fid
        #[arg(short, long)]
        stid: bool,
    },
}

pub async fn handle(action: ForumAction, format: OutputFormat, verbose: bool) -> Result<()> {
    match action {
        ForumAction::List => list_categories(format, verbose).await,
        ForumAction::Search { keyword } => search_forums(&keyword, format).await,
        ForumAction::Favorites => list_favorites(format).await,
        ForumAction::FavAdd { id, stid } => add_favorite(&id, stid).await,
        ForumAction::FavRemove { id, stid } => remove_favorite(&id, stid).await,
    }
}

async fn list_categories(format: OutputFormat, verbose: bool) -> Result<()> {
    let client = build_client()?;
    let categories = client.forums().list().await?;

    if verbose {
        for category in &categories {
            if matches!(format, OutputFormat::Plain) {
                println!("\n{}", category.name);
                println!("{}", "=".repeat(category.name.len()));
            }
            let rows: Vec<ForumRow> = category.forums.iter().map(ForumRow::from).collect();
            print_table(rows, format);
        }
    } else {
        let rows: Vec<CategoryRow> = categories.iter().map(CategoryRow::from).collect();
        print_table(rows, format);
    }

    Ok(())
}

async fn search_forums(keyword: &str, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let forums = client.forums().search(keyword).await?;

    let rows: Vec<ForumRow> = forums.iter().map(ForumRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn list_favorites(format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let forums = client.forums().favorites().await?;

    let rows: Vec<ForumRow> = forums.iter().map(ForumRow::from).collect();
    print_table(rows, format);

    Ok(())
}

async fn add_favorite(id: &str, is_stid: bool) -> Result<()> {
    let client = build_authed_client()?;

    let forum_id = if is_stid {
        ForumIdKind::stid(id)
    } else {
        ForumIdKind::fid(id)
    };

    client
        .forums()
        .modify_favorite(forum_id, FavoriteForumOp::Add)
        .await?;

    println!("Added forum {} to favorites", id);
    Ok(())
}

async fn remove_favorite(id: &str, is_stid: bool) -> Result<()> {
    let client = build_authed_client()?;

    let forum_id = if is_stid {
        ForumIdKind::stid(id)
    } else {
        ForumIdKind::fid(id)
    };

    client
        .forums()
        .modify_favorite(forum_id, FavoriteForumOp::Remove)
        .await?;

    println!("Removed forum {} from favorites", id);
    Ok(())
}

