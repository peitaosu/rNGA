use std::collections::HashMap;

use crate::error::Result;
use crate::models::{User, UserId, UserName};

pub fn parse_user_from_attrs(
    attrs: &HashMap<String, String>,
    user_id: UserId,
) -> Result<User> {
    let name = attrs
        .get("username")
        .map(|s| UserName::parse(s))
        .unwrap_or_default();

    Ok(User {
        id: user_id,
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
    })
}

pub fn parse_user_from_node(
    attrs: &HashMap<String, String>,
) -> Result<Option<User>> {
    let id = match attrs.get("uid") {
        Some(uid) => UserId::new(uid),
        None => return Ok(None),
    };

    Ok(Some(parse_user_from_attrs(attrs, id)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_from_attrs() {
        let mut attrs = HashMap::new();
        attrs.insert("username".into(), "tester".into());
        attrs.insert("fame".into(), "100".into());
        attrs.insert("postnum".into(), "42".into());

        let user = parse_user_from_attrs(&attrs, UserId::new("123")).unwrap();
        assert_eq!(user.id.as_str(), "123");
        assert_eq!(user.name.display(), "tester");
        assert_eq!(user.reputation, 100);
        assert_eq!(user.posts, 42);
    }

    #[test]
    fn test_parse_user_from_node_missing_uid() {
        let attrs = HashMap::new();
        assert!(parse_user_from_node(&attrs).unwrap().is_none());
    }
}
