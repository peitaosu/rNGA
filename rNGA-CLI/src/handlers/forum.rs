//! Forum handlers.

use anyhow::{Context, Result};
use colored::Colorize;
use rnga::models::*;
use rnga::NGAClient;
use rust_i18n::t;
use serde::Serialize;

use crate::handlers::meta::ResponseMeta;
use crate::output::{PlainPrint, TableRow};

#[derive(Debug, Clone, Serialize)]
pub struct ForumInfo {
    pub fid: Option<String>,
    pub stid: Option<String>,
    pub name: String,
    pub info: String,
}

impl ForumInfo {
    fn id_label(&self) -> &str {
        self.stid
            .as_deref()
            .or(self.fid.as_deref())
            .unwrap_or("")
    }
}

impl From<&Forum> for ForumInfo {
    fn from(f: &Forum) -> Self {
        let (fid, stid) = match f.id.as_ref() {
            Some(ForumIdKind::Fid(id)) => (Some(id.clone()), None),
            Some(ForumIdKind::Stid(id)) => (None, Some(id.clone())),
            None => (None, None),
        };

        Self {
            fid,
            stid,
            name: f.name.clone(),
            info: f.info.clone(),
        }
    }
}

impl TableRow for ForumInfo {
    fn headers() -> Vec<&'static str> {
        vec!["FID", "STID", "Name", "Info"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.fid.clone().unwrap_or_default(),
            self.stid.clone().unwrap_or_default(),
            self.name.clone(),
            self.info.clone(),
        ]
    }
}

impl PlainPrint for ForumInfo {
    fn plain_print(&self) {
        println!("[{}] {}", self.id_label().cyan(), self.name.bold());
        if !self.info.is_empty() {
            println!("   {}", self.info.dimmed());
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryInfo {
    pub id: String,
    pub name: String,
    pub forum_count: usize,
    pub forums: Vec<ForumInfo>,
}

impl From<&Category> for CategoryInfo {
    fn from(c: &Category) -> Self {
        Self {
            id: c.id.clone(),
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

#[derive(Debug, Clone, Serialize)]
pub struct ForumListResult {
    pub categories: Vec<CategoryInfo>,
    pub meta: ResponseMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForumSearchResult {
    pub forums: Vec<ForumInfo>,
    pub meta: ResponseMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct FavoriteModifyResult {
    pub id: String,
    pub action: String,
}

pub async fn list_categories(client: &NGAClient) -> Result<ForumListResult> {
    let categories = client
        .forums()
        .list()
        .await
        .context("listing forum categories")?;
    Ok(ForumListResult {
        categories: categories.iter().map(CategoryInfo::from).collect(),
        meta: ResponseMeta::empty(),
    })
}

pub async fn search_forums(client: &NGAClient, keyword: &str) -> Result<ForumSearchResult> {
    let forums = client
        .forums()
        .search(keyword)
        .await
        .context("searching forums")?;
    Ok(ForumSearchResult {
        forums: forums.iter().map(ForumInfo::from).collect(),
        meta: ResponseMeta::empty(),
    })
}

pub async fn list_favorites(client: &NGAClient) -> Result<ForumSearchResult> {
    let forums = client
        .forums()
        .favorites()
        .await
        .context("listing favorite forums")?;
    Ok(ForumSearchResult {
        forums: forums.iter().map(ForumInfo::from).collect(),
        meta: ResponseMeta::empty(),
    })
}

pub async fn add_favorite(client: &NGAClient, forum_id: &str, is_stid: bool) -> Result<FavoriteModifyResult> {
    let id = ForumIdKind::from_stid_flag(forum_id, is_stid);
    client
        .forums()
        .modify_favorite(id, FavoriteForumOp::Add)
        .await
        .context("adding forum favorite")?;

    Ok(FavoriteModifyResult {
        id: forum_id.to_string(),
        action: "added".to_string(),
    })
}

pub async fn remove_favorite(client: &NGAClient, forum_id: &str, is_stid: bool) -> Result<FavoriteModifyResult> {
    let id = ForumIdKind::from_stid_flag(forum_id, is_stid);
    client
        .forums()
        .modify_favorite(id, FavoriteForumOp::Remove)
        .await
        .context("removing forum favorite")?;

    Ok(FavoriteModifyResult {
        id: forum_id.to_string(),
        action: "removed".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forum_info_from_stid() {
        let forum = Forum::minimal(ForumIdKind::stid("123"), "Test");
        let info = ForumInfo::from(&forum);
        assert_eq!(info.stid.as_deref(), Some("123"));
        assert!(info.fid.is_none());
    }
}
