//! Forum commands.

use anyhow::Result;
use clap::Subcommand;
use rust_i18n::t;

use crate::config::{build_authed_client, build_client};
use crate::handlers::forum as handlers;
use crate::output::{print_table, OutputFormat};

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
    let categories = handlers::list_categories(&client).await?;

    if verbose {
        for category in &categories {
            if matches!(format, OutputFormat::Plain) {
                println!("\n{}", category.name);
                println!("{}", "=".repeat(category.name.len()));
            }
            print_table(category.forums.clone(), format);
        }
    } else {
        print_table(categories, format);
    }

    Ok(())
}

async fn search_forums(keyword: &str, format: OutputFormat) -> Result<()> {
    let client = build_client()?;
    let forums = handlers::search_forums(&client, keyword).await?;

    print_table(forums, format);

    Ok(())
}

async fn list_favorites(format: OutputFormat) -> Result<()> {
    let client = build_authed_client()?;
    let forums = handlers::list_favorites(&client).await?;

    print_table(forums, format);

    Ok(())
}

async fn add_favorite(id: &str, is_stid: bool) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::add_favorite(&client, id, is_stid).await?;

    println!("{}", t!("added_forum_to_favorites", id = result.id));
    Ok(())
}

async fn remove_favorite(id: &str, is_stid: bool) -> Result<()> {
    let client = build_authed_client()?;
    let result = handlers::remove_favorite(&client, id, is_stid).await?;

    println!("{}", t!("removed_forum_from_favorites", id = result.id));
    Ok(())
}
