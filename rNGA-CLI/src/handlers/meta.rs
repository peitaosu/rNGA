//! Shared response metadata for CLI and MCP agents.

use serde::{Deserialize, Serialize};

pub const MAX_LIST_PAGES: u32 = 5;
pub const MAX_READ_PAGES: u32 = 20;
pub const MAX_RANGE_PAGES: u32 = 20;
pub const MAX_RECENT_TOPICS: usize = 50;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageWarning {
    pub page: u32,
    pub error: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_pages: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetched_pages: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<PageWarning>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub truncated: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl ResponseMeta {
    pub fn list(page: u32, total_pages: u32, fetched_pages: u32, warnings: Vec<PageWarning>, truncated: bool) -> Self {
        Self {
            page: Some(page),
            total_pages: Some(total_pages),
            fetched_pages: Some(fetched_pages),
            warnings,
            truncated,
        }
    }

    pub fn page_only(page: u32, total_pages: u32) -> Self {
        Self {
            page: Some(page),
            total_pages: Some(total_pages),
            fetched_pages: Some(1),
            ..Default::default()
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }
}

pub fn content_fields(content: &rnga::PostContent) -> (String, String, Option<String>) {
    (
        content.to_plain_text(),
        content.raw.clone(),
        content.parse_error.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_meta_truncated_omitted_when_false() {
        let json = serde_json::to_string(&ResponseMeta::page_only(1, 10)).unwrap();
        assert!(!json.contains("truncated"));
    }

    #[test]
    fn test_limits_are_positive() {
        assert!(MAX_LIST_PAGES > 0);
        assert!(MAX_READ_PAGES > 0);
        assert!(MAX_RANGE_PAGES > 0);
        assert!(MAX_RECENT_TOPICS > 0);
    }

    #[test]
    fn test_content_fields_returns_plain_and_raw() {
        let content = rnga::PostContent::plain("hello world");
        let (plain, raw, parse_error) = content_fields(&content);
        assert_eq!(plain, "hello world");
        assert_eq!(raw, "hello world");
        assert!(parse_error.is_none());
    }
}
