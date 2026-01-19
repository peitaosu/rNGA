//! Forum handlers.

use anyhow::Result;
use colored::Colorize;
use rnga::models::*;
use rnga::NGAClient;
use rust_i18n::t;
use serde::Serialize;

use crate::output::{PlainPrint, TableRow};

/// Forum information.
#[derive(Debug, Clone, Serialize)]
pub struct ForumInfo {
    pub id: String,
    pub name: String,
    pub info: String,
}

impl From<&Forum> for ForumInfo {
    fn from(f: &Forum) -> Self {
        Self {
            id: f
                .id
                .as_ref()
                .map(|id| id.id().to_string())
                .unwrap_or_default(),
            name: f.name.clone(),
            info: f.info.clone(),
        }
    }
}

impl TableRow for ForumInfo {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Name", "Info"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.id.clone(), self.name.clone(), self.info.clone()]
    }
}

impl PlainPrint for ForumInfo {
    fn plain_print(&self) {
        println!("[{}] {}", self.id.cyan(), self.name.bold());
        if !self.info.is_empty() {
            println!("   {}", self.info.dimmed());
        }
    }
}

/// Category with its forums.
#[derive(Debug, Clone, Serialize)]
pub struct CategoryInfo {
    pub name: String,
    pub forum_count: usize,
    pub forums: Vec<ForumInfo>,
}

impl From<&Category> for CategoryInfo {
    fn from(c: &Category) -> Self {
        Self {
            name: c.name.clone(),
            forum_count: c.forums.len(),
            forums: c.forums.iter().map(ForumInfo::from).collect(),
        }
    }
}

impl TableRow for CategoryInfo {
    fn headers() -> Vec<&'static str> {
        vec!["Category", "Forums"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.name.clone(), self.forum_count.to_string()]
    }
}

impl PlainPrint for CategoryInfo {
    fn plain_print(&self) {
        println!(
            "{} {}",
            self.name.bold(),
            t!("forums_count", count = self.forum_count)
                .to_string()
                .dimmed()
        );
    }
}

/// Result of favorite modification.
#[derive(Debug, Clone, Serialize)]
pub struct FavoriteModifyResult {
    pub id: String,
    pub action: String,
}

/// List all forum categories.
pub async fn list_categories(client: &NGAClient) -> Result<Vec<CategoryInfo>> {
    let categories = client.forums().list().await?;
    Ok(categories.iter().map(CategoryInfo::from).collect())
}

/// Search forums by keyword.
pub async fn search_forums(client: &NGAClient, keyword: &str) -> Result<Vec<ForumInfo>> {
    let forums = client.forums().search(keyword).await?;
    Ok(forums.iter().map(ForumInfo::from).collect())
}

/// List favorite forums.
pub async fn list_favorites(client: &NGAClient) -> Result<Vec<ForumInfo>> {
    let forums = client.forums().favorites().await?;
    Ok(forums.iter().map(ForumInfo::from).collect())
}

/// Add forum to favorites.
pub async fn add_favorite(
    client: &NGAClient,
    id: &str,
    is_stid: bool,
) -> Result<FavoriteModifyResult> {
    let forum_id = if is_stid {
        ForumIdKind::stid(id)
    } else {
        ForumIdKind::fid(id)
    };

    client
        .forums()
        .modify_favorite(forum_id, FavoriteForumOp::Add)
        .await?;

    Ok(FavoriteModifyResult {
        id: id.to_string(),
        action: "added".to_string(),
    })
}

/// Remove forum from favorites.
pub async fn remove_favorite(
    client: &NGAClient,
    id: &str,
    is_stid: bool,
) -> Result<FavoriteModifyResult> {
    let forum_id = if is_stid {
        ForumIdKind::stid(id)
    } else {
        ForumIdKind::fid(id)
    };

    client
        .forums()
        .modify_favorite(forum_id, FavoriteForumOp::Remove)
        .await?;

    Ok(FavoriteModifyResult {
        id: id.to_string(),
        action: "removed".to_string(),
    })
}
