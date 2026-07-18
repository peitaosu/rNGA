//! Forum API.

use std::sync::Arc;
use std::time::Duration;

use crate::{
    cache::{get_json, set_json},
    client::NGAClientInner,
    error::{Error, Result},
    models::{Category, FavoriteForumOp, Forum, ForumIdKind, SubforumFilterOp},
    parser::XmlDocument,
};

const FORUM_LIST_CACHE_KEY: &str = "forums:list";
const FORUM_LIST_CACHE_TTL: Duration = Duration::from_secs(300);

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
        if let Some(cache) = &self.client.cache {
            if let Some(categories) = get_json(cache.as_ref(), FORUM_LIST_CACHE_KEY).await {
                return Ok(categories);
            }
        }

        let json = self
            .client
            .post_json(
                "app_api.php",
                &[("__lib", "home"), ("__act", "category"), ("_v", "2")],
                &[],
            )
            .await?;

        let categories = parse_categories_json(&json)?;

        if let Some(cache) = &self.client.cache {
            let _ = set_json(
                cache.as_ref(),
                FORUM_LIST_CACHE_KEY,
                &categories,
                Some(FORUM_LIST_CACHE_TTL),
            )
            .await;
        }

        Ok(categories)
    }

    /// Search forums by keyword.
    pub async fn search(&self, keyword: &str) -> Result<Vec<Forum>> {
        let xml = self
            .client
            .post("forum.php", &[("key", keyword)], &[])
            .await?;

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
        let json = self
            .client
            .post_json_authed(
                "app_api.php",
                &[("__lib", "favorforum"), ("__act", "sync")],
                &[],
            )
            .await?;

        parse_favorites_json(&json)
    }

    /// Modify favorite forums.
    pub async fn modify_favorite(&self, forum_id: ForumIdKind, op: FavoriteForumOp) -> Result<()> {
        let id_str = forum_id.id().to_owned();
        let act = match op {
            FavoriteForumOp::Add => "add",
            FavoriteForumOp::Remove => "del",
        };

        self.client
            .post_json_authed(
                "app_api.php",
                &[("__lib", "favorforum"), ("__act", act)],
                &[("fid", &id_str)],
            )
            .await?;

        Ok(())
    }

    /// Set subforum filter.
    pub async fn set_subforum_filter(
        &self,
        forum_id: &str,
        subforum_filter_id: &str,
        op: SubforumFilterOp,
    ) -> Result<()> {
        self.client
            .post_authed(
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
            )
            .await?;

        Ok(())
    }
}

/// Parse forum from XML node.
fn parse_forum(node: &crate::parser::XmlNode<'_>) -> Result<Option<Forum>> {
    let attrs = node.attrs();

    let icon_id = attrs
        .get("id")
        .or_else(|| attrs.get("fid"))
        .cloned()
        .unwrap_or_default();
    let icon_url = Forum::icon_url_for(&icon_id);

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

fn parse_favorites_json(value: &serde_json::Value) -> Result<Vec<Forum>> {
    let mut forums = Vec::new();
    collect_favorite_forums(value, &mut forums)?;
    Ok(forums)
}

fn collect_favorite_forums(value: &serde_json::Value, forums: &mut Vec<Forum>) -> Result<()> {
    if let Some(items) = value.get("result").and_then(|value| value.as_array()) {
        for item in items {
            collect_favorite_forum_entry(item, forums)?;
        }
        return Ok(());
    }

    if let Some(data) = value.get("data") {
        collect_favorite_forums(data, forums)?;
    }

    if let Some(items) = value.get("item").and_then(|value| value.as_array()) {
        for item in items {
            collect_favorite_forum_entry(item, forums)?;
        }
    } else if value.get("item").is_some() {
        collect_favorite_forum_entry(value.get("item").unwrap(), forums)?;
    }

    Ok(())
}

fn collect_favorite_forum_entry(value: &serde_json::Value, forums: &mut Vec<Forum>) -> Result<()> {
    if let Some(groups) = value.get("groups").and_then(|value| value.as_array()) {
        for group in groups {
            if let Some(items) = group.get("forums").and_then(|value| value.as_array()) {
                for item in items {
                    if let Some(forum) = parse_forum_json(item)? {
                        forums.push(forum);
                    }
                }
            }
        }
        return Ok(());
    }

    if let Some(forum) = parse_forum_json(value)? {
        forums.push(forum);
    }

    Ok(())
}

fn parse_categories_json(value: &serde_json::Value) -> Result<Vec<Category>> {
    let items = value
        .get("result")
        .and_then(|value| value.as_array())
        .ok_or_else(|| Error::parse("missing category result array"))?;

    let mut categories = Vec::new();
    for item in items {
        if let Some(category) = parse_category_json(item)? {
            categories.push(category);
        }
    }

    Ok(categories)
}

fn parse_category_json(value: &serde_json::Value) -> Result<Option<Category>> {
    let id = value
        .get("_id")
        .and_then(json_string)
        .or_else(|| value.get("id").and_then(json_string))
        .filter(|id| !id.is_empty());

    let id = match id {
        Some(id) => id,
        None => return Ok(None),
    };

    let name = match value.get("name").and_then(|value| value.as_str()) {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => return Ok(None),
    };

    let mut forums = Vec::new();
    if let Some(groups) = value.get("groups").and_then(|value| value.as_array()) {
        for group in groups {
            if let Some(forum_items) = group.get("forums").and_then(|value| value.as_array()) {
                for forum_value in forum_items {
                    if let Some(forum) = parse_forum_json(forum_value)? {
                        forums.push(forum);
                    }
                }
            }
        }
    }

    Ok(Some(Category { id, name, forums }))
}

fn parse_forum_json(value: &serde_json::Value) -> Result<Option<Forum>> {
    let name = match value.get("name").and_then(|value| value.as_str()) {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => return Ok(None),
    };

    let id = if let Some(stid) = value
        .get("stid")
        .and_then(json_string)
        .filter(|stid| !stid.is_empty() && stid != "0")
    {
        Some(ForumIdKind::stid(stid))
    } else if let Some(fid) = value
        .get("fid")
        .and_then(json_string)
        .filter(|fid| !fid.is_empty() && fid != "0")
    {
        Some(ForumIdKind::fid(fid))
    } else {
        None
    };

    let icon_id = value
        .get("id")
        .and_then(json_string)
        .unwrap_or_default();
    let icon_url = Forum::icon_url_for(&icon_id);
    let info = value
        .get("info")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();
    let topped_topic_id = value
        .get("topped_topic")
        .and_then(json_string)
        .unwrap_or_default();

    Ok(Some(Forum {
        id,
        name,
        info,
        icon_url,
        topped_topic_id,
    }))
}

fn json_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_categories_json() {
        let json = serde_json::json!({
            "result": [{
                "_id": "other",
                "name": "Test Category",
                "groups": [{
                    "forums": [{
                        "fid": "706",
                        "name": "大时代",
                        "info": "股市讨论",
                        "id": "706"
                    }, {
                        "fid": -7,
                        "stid": "39827852",
                        "name": "考研讨论",
                        "id": "39827852"
                    }]
                }]
            }]
        });

        let categories = parse_categories_json(&json).unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].id, "other");
        assert_eq!(categories[0].name, "Test Category");
        assert_eq!(categories[0].forums.len(), 2);
        assert_eq!(categories[0].forums[0].name, "大时代");
        assert!(categories[0].forums[1].id.as_ref().unwrap().is_stid());
    }

    #[test]
    fn test_parse_favorites_json_flat_result() {
        let json = serde_json::json!({
            "result": [{
                "fid": "7",
                "name": "网事杂谈",
                "info": "谈笑风生",
                "id": "7"
            }, {
                "stid": "39827852",
                "name": "考研讨论",
                "id": "39827852"
            }]
        });

        let forums = parse_favorites_json(&json).unwrap();
        assert_eq!(forums.len(), 2);
        assert_eq!(forums[0].name, "网事杂谈");
        assert!(forums[1].id.as_ref().unwrap().is_stid());
    }

    #[test]
    fn test_parse_favorites_json_nested_item() {
        let json = serde_json::json!({
            "data": {
                "item": [{
                    "fid": "706",
                    "name": "大时代",
                    "info": "股市讨论",
                    "id": "706"
                }]
            }
        });

        let forums = parse_favorites_json(&json).unwrap();
        assert_eq!(forums.len(), 1);
        assert_eq!(forums[0].name, "大时代");
    }
}
