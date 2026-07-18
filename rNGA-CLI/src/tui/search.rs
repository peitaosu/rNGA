use std::borrow::Cow;

use ib_pinyin::{matcher::PinyinMatcher, pinyin::PinyinNotation};

pub struct FilterQuery<'q> {
    raw: &'q str,
    lower: Cow<'q, str>,
    pinyin: PinyinMatcher<'q>,
}

impl<'q> FilterQuery<'q> {
    pub fn prepare(query: &'q str) -> Option<Self> {
        let raw = query.trim();
        if raw.is_empty() {
            return None;
        }
        Some(Self {
            raw,
            lower: Cow::Owned(raw.to_lowercase()),
            pinyin: PinyinMatcher::builder(raw)
                .pinyin_notations(PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter)
                .build(),
        })
    }

    pub fn matches(&self, haystack: &str) -> bool {
        if haystack.contains(self.raw) {
            return true;
        }
        if haystack.to_lowercase().contains(self.lower.as_ref()) {
            return true;
        }
        self.pinyin.is_match(haystack)
    }

    pub fn matches_any<'a, I>(&self, parts: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        parts.into_iter().any(|part| self.matches(part))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(query: &str, haystack: &str) -> bool {
        FilterQuery::prepare(query)
            .map(|filter| filter.matches(haystack))
            .unwrap_or(true)
    }

    #[test]
    fn matches_chinese_substring() {
        assert!(matches("杂谈", "网事杂谈"));
    }

    #[test]
    fn matches_pinyin_initials() {
        assert!(matches("wszt", "网事杂谈"));
    }

    #[test]
    fn matches_pinyin_full() {
        assert!(matches("wangshi", "网事杂谈"));
    }

    #[test]
    fn empty_query_matches_all() {
        assert!(matches("", "anything"));
    }
}
