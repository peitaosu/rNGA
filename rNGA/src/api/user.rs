//! User API.

use std::sync::Arc;

use crate::{
    client::NGAClientInner,
    error::{Error, Result},
    models::{User, UserId, UserName},
    parser::XmlDocument,
};

/// API for user operations.
pub struct UserApi {
    client: Arc<NGAClientInner>,
}

impl UserApi {
    pub(crate) fn new(client: Arc<NGAClientInner>) -> Self {
        Self { client }
    }

    /// Get user by ID.
    pub async fn get(&self, user_id: impl Into<UserId>) -> Result<User> {
        let user_id = user_id.into();

        let xml = self
            .client
            .post(
                "nuke.php",
                &[
                    ("__lib", "ucp"),
                    ("__act", "get"),
                    ("uid", user_id.as_str()),
                ],
                &[],
            )
            .await?;

        parse_user_response(&xml, &user_id)
    }

    /// Get user by username.
    pub async fn get_by_name(&self, username: &str) -> Result<User> {
        let xml = self
            .client
            .post(
                "nuke.php",
                &[("__lib", "ucp"), ("__act", "get"), ("username", username)],
                &[],
            )
            .await?;

        let doc = XmlDocument::parse(&xml)?;
        let uid = doc
            .string_opt("/root/data/item/uid")
            .ok_or_else(|| Error::missing("uid"))?;

        parse_user_response(&xml, &UserId::new(uid))
    }

    /// Get current authenticated user.
    pub async fn me(&self) -> Result<User> {
        let auth = self.client.require_auth()?;
        self.get(&auth.uid).await
    }

    /// Search users.
    pub async fn search(&self, keyword: &str) -> Result<Vec<UserSearchResult>> {
        let xml = self
            .client
            .post(
                "nuke.php",
                &[("__lib", "ucp"), ("__act", "search"), ("key", keyword)],
                &[],
            )
            .await?;

        parse_user_search(&xml)
    }
}

/// Result of a user search.
#[derive(Debug, Clone)]
pub struct UserSearchResult {
    /// User ID.
    pub id: UserId,
    /// Username.
    pub name: String,
    /// Avatar URL.
    pub avatar_url: Option<String>,
}

fn parse_user_response(xml: &str, user_id: &UserId) -> Result<User> {
    let doc = XmlDocument::parse(xml)?;

    let node = doc
        .select_one("/root/data/item")?
        .ok_or_else(|| Error::missing("user data"))?;

    let attrs = node.attrs();

    let name = attrs
        .get("username")
        .map(|s| UserName::parse(s))
        .unwrap_or_default();

    let user = User {
        id: user_id.clone(),
        name,
        avatar_url: attrs.get("avatar").cloned(),
        reputation: attrs.get("fame").and_then(|s| s.parse().ok()).unwrap_or(0),
        posts: attrs
            .get("postnum")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        reg_date: attrs
            .get("regdate")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        signature: attrs.get("signature").cloned(),
        is_admin: attrs.get("admincheck").map(|s| s != "0").unwrap_or(false),
        is_mod: attrs
            .get("groupid")
            .map(|s| s == "5" || s == "6")
            .unwrap_or(false),
        is_muted: attrs
            .get("mute")
            .and_then(|s| s.parse::<i64>().ok())
            .map(|t| t > 0)
            .unwrap_or(false),
        honor: attrs.get("honor").cloned(),
    };

    Ok(user)
}

fn parse_user_search(xml: &str) -> Result<Vec<UserSearchResult>> {
    let doc = XmlDocument::parse(xml)?;
    let mut results = Vec::new();

    for node in doc.select("/root/data/item")? {
        let attrs = node.attrs();

        if let Some(uid) = attrs.get("uid") {
            results.push(UserSearchResult {
                id: uid.clone().into(),
                name: attrs.get("username").cloned().unwrap_or_default(),
                avatar_url: attrs.get("avatar").cloned(),
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_creation() {
        let id = UserId::new("12345");
        assert_eq!(id.as_str(), "12345");
    }
}
