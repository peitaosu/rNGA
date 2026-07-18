//! User API.

use std::sync::Arc;
use std::time::Duration;

use crate::{
    cache::{get_json, set_json},
    client::NGAClientInner,
    error::{Error, Result},
    models::{User, UserId},
    parser::{parse_user_from_attrs, XmlDocument},
};

const USER_CACHE_TTL: Duration = Duration::from_secs(300);

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
        let cache_key = format!("user:id:{}", user_id.as_str());

        if let Some(cache) = &self.client.cache {
            if let Some(user) = get_json(cache.as_ref(), &cache_key).await {
                return Ok(user);
            }
        }

        let user = self.fetch_user_by_id(&user_id).await?;

        if let Some(cache) = &self.client.cache {
            let _ = set_json(
                cache.as_ref(),
                &cache_key,
                &user,
                Some(USER_CACHE_TTL),
            )
            .await;
        }

        Ok(user)
    }

    async fn fetch_user_by_id(&self, user_id: &UserId) -> Result<User> {
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

        parse_user_response(&xml, user_id)
    }

    /// Get user by username.
    pub async fn get_by_name(&self, username: &str) -> Result<User> {
        let cache_key = format!("user:name:{username}");

        if let Some(cache) = &self.client.cache {
            if let Some(user) = get_json(cache.as_ref(), &cache_key).await {
                return Ok(user);
            }
        }

        let xml = self
            .client
            .post(
                "nuke.php",
                &[("__lib", "ucp"), ("__act", "get"), ("username", username)],
                &[],
            )
            .await?;

        let uid = {
            let doc = XmlDocument::parse(&xml)?;
            doc.string_opt("/root/data/item/uid")
                .ok_or_else(|| Error::missing("uid"))?
        };

        let user = parse_user_response(&xml, &UserId::new(uid))?;

        if let Some(cache) = &self.client.cache {
            let _ = set_json(
                cache.as_ref(),
                &cache_key,
                &user,
                Some(USER_CACHE_TTL),
            )
            .await;
            let id_key = format!("user:id:{}", user.id.as_str());
            let _ = set_json(cache.as_ref(), &id_key, &user, Some(USER_CACHE_TTL)).await;
        }

        Ok(user)
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

    parse_user_from_attrs(&attrs, user_id.clone())
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
