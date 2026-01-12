//! Forum API.

use std::sync::Arc;

use crate::{
    client::NGAClientInner,
    error::Result,
    models::{Category, FavoriteForumOp, Forum, ForumIdKind, SubforumFilterOp},
    parser::XmlDocument,
    client::FORUM_ICON_PATH,
};

/// API for forum operations.
pub struct ForumApi {
    client: Arc<NGAClientInner>,
}

impl ForumApi {
    pub(crate) fn new(client: Arc<NGAClientInner>) -> Self {
        Self { client }
    }

    /// List all forum categories.
    pub async fn list(&self) -> Result<Vec<Category>> {
        let xml = self.client.post(
            "app_api.php",
            &[("__lib", "home"), ("__act", "category")],
            &[],
        ).await?;
        
        let doc = XmlDocument::parse(&xml)?;
        let mut categories = Vec::new();
        
        for cat_node in doc.select("/root/data/item")? {
            if let Some(category) = parse_category(&cat_node)? {
                categories.push(category);
            }
        }
        
        Ok(categories)
    }

    /// Search forums by keyword.
    pub async fn search(&self, keyword: &str) -> Result<Vec<Forum>> {
        let xml = self.client.post(
            "forum.php",
            &[("key", keyword)],
            &[],
        ).await?;
        
        let doc = XmlDocument::parse(&xml)?;
        let mut forums = Vec::new();
        
        for node in doc.select("/root/item")? {
            if let Some(forum) = parse_forum(&node)? {
                forums.push(forum);
            }
        }
        
        Ok(forums)
    }

    /// Get favorite forums.
    pub async fn favorites(&self) -> Result<Vec<Forum>> {
        let xml = self.client.post_authed(
            "nuke.php",
            &[("__lib", "forum_favor2"), ("__act", "forum_favor")],
            &[("action", "get")],
        ).await?;
        
        let doc = XmlDocument::parse(&xml)?;
        let mut forums = Vec::new();
        
        for node in doc.select("/root/data/item/item")? {
            if let Some(forum) = parse_forum(&node)? {
                forums.push(forum);
            }
        }
        
        Ok(forums)
    }

    /// Modify favorite forums.
    pub async fn modify_favorite(&self, forum_id: ForumIdKind, op: FavoriteForumOp) -> Result<()> {
        let id_str = forum_id.id().to_owned();
        
        self.client.post_authed(
            "nuke.php",
            &[("__lib", "forum_favor2"), ("__act", "forum_favor")],
            &[("action", op.param()), ("fid", &id_str)],
        ).await?;
        
        Ok(())
    }

    /// Set subforum filter.
    pub async fn set_subforum_filter(
        &self,
        forum_id: &str,
        subforum_filter_id: &str,
        op: SubforumFilterOp,
    ) -> Result<()> {
        self.client.post_authed(
            "nuke.php",
            &[
                ("__lib", "user_option"),
                ("__act", "set"),
                (op.param(), subforum_filter_id),
            ],
            &[
                ("fid", forum_id),
                ("type", "1"),
                ("info", "add_to_block_tids"),
            ],
        ).await?;
        
        Ok(())
    }
}

/// Parse category from XML node.
fn parse_category(node: &crate::parser::XmlNode<'_>) -> Result<Option<Category>> {
    let attrs = node.attrs();
    
    let id = match attrs.get("_id") {
        Some(id) => id.clone(),
        None => return Ok(None),
    };
    
    let name = match attrs.get("name") {
        Some(name) => name.clone(),
        None => return Ok(None),
    };
    
    let mut forums = Vec::new();
    for group in node.children_named("groups") {
        for item in group.children_named("item") {
            for forums_node in item.children_named("forums") {
                for forum_node in forums_node.children_named("item") {
                    if let Some(forum) = parse_forum(&forum_node)? {
                        forums.push(forum);
                    }
                }
            }
        }
    }
    
    Ok(Some(Category { id, name, forums }))
}

/// Parse forum from XML node.
fn parse_forum(node: &crate::parser::XmlNode<'_>) -> Result<Option<Forum>> {
    let attrs = node.attrs();
    
    let icon_id = attrs.get("id")
        .or_else(|| attrs.get("fid"))
        .cloned()
        .unwrap_or_default();
    let icon_url = format!("{}{}.png", FORUM_ICON_PATH, icon_id);
    
    let id = if let Some(stid) = attrs.get("stid").filter(|s| !s.is_empty() && *s != "0") {
        Some(ForumIdKind::stid(stid.clone()))
    } else if let Some(fid) = attrs.get("fid").filter(|s| !s.is_empty() && *s != "0") {
        Some(ForumIdKind::fid(fid.clone()))
    } else {
        None
    };
    
    let name = match attrs.get("name") {
        Some(name) => name.clone(),
        None => return Ok(None),
    };
    
    Ok(Some(Forum {
        id,
        name,
        info: attrs.get("info").cloned().unwrap_or_default(),
        icon_url,
        topped_topic_id: attrs.get("topped_topic").cloned().unwrap_or_default(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forum_id_kind_param() {
        let fid = ForumIdKind::fid("123");
        assert_eq!(fid.param_name(), "fid");
        
        let stid = ForumIdKind::stid("456");
        assert_eq!(stid.param_name(), "stid");
    }
}
