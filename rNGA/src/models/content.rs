//! Post content and span models.

use serde::{Deserialize, Serialize};

/// Parsed post content with structured spans.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PostContent {
    /// Structured content spans.
    pub spans: Vec<Span>,
    /// Raw unparsed content.
    pub raw: String,
    /// Parse error if content couldn't be fully parsed.
    pub parse_error: Option<String>,
}

impl PostContent {
    /// Create empty content.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create content from plain text.
    pub fn plain(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            spans: vec![Span::plain(&text)],
            raw: text,
            parse_error: None,
        }
    }

    /// Check if content is empty.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() && self.raw.is_empty()
    }

    /// Extract plain text from all spans.
    pub fn to_plain_text(&self) -> String {
        self.spans.iter().map(|s| s.to_plain_text()).collect()
    }
}

/// A span of content with specific formatting or type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// The kind of span with its data.
    pub kind: SpanKind,
}

/// Different kinds of content spans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanKind {
    /// Plain text.
    Plain { text: String },
    /// Line break.
    LineBreak,
    /// Sticker/emoji.
    Sticker { name: String },
    /// Divider line with optional inner content.
    Divider { spans: Vec<Span> },
    /// Tagged content.
    Tagged {
        tag: String,
        attributes: Vec<String>,
        complex_attributes: Vec<String>,
        spans: Vec<Span>,
    },
}

impl Span {
    /// Create a plain text span.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            kind: SpanKind::Plain { text: text.into() },
        }
    }

    /// Create a line break span.
    pub fn line_break() -> Self {
        Self {
            kind: SpanKind::LineBreak,
        }
    }

    /// Create a sticker span.
    pub fn sticker(name: impl Into<String>) -> Self {
        Self {
            kind: SpanKind::Sticker { name: name.into() },
        }
    }

    /// Create a divider span.
    pub fn divider(spans: Vec<Span>) -> Self {
        Self {
            kind: SpanKind::Divider { spans },
        }
    }

    /// Create a tagged span.
    pub fn tagged(
        tag: impl Into<String>,
        attributes: Vec<String>,
        complex_attributes: Vec<String>,
        spans: Vec<Span>,
    ) -> Self {
        Self {
            kind: SpanKind::Tagged {
                tag: tag.into(),
                attributes,
                complex_attributes,
                spans,
            },
        }
    }

    /// Extract plain text from this span.
    pub fn to_plain_text(&self) -> String {
        match &self.kind {
            SpanKind::Plain { text } => text.clone(),
            SpanKind::LineBreak => "\n".to_owned(),
            SpanKind::Sticker { .. } => String::new(),
            SpanKind::Divider { spans } => spans.iter().map(|s| s.to_plain_text()).collect(),
            SpanKind::Tagged { spans, .. } => spans.iter().map(|s| s.to_plain_text()).collect(),
        }
    }

    /// Check if this is a plain text span.
    pub fn is_plain(&self) -> bool {
        matches!(self.kind, SpanKind::Plain { .. })
    }

    /// Check if this is a tagged span with the given tag.
    pub fn is_tag(&self, tag: &str) -> bool {
        matches!(&self.kind, SpanKind::Tagged { tag: t, .. } if t == tag)
    }

    /// Get the tag name if this is a tagged span.
    pub fn tag_name(&self) -> Option<&str> {
        match &self.kind {
            SpanKind::Tagged { tag, .. } => Some(tag),
            _ => None,
        }
    }

    /// Get the first attribute if this is a tagged span.
    pub fn first_attr(&self) -> Option<&str> {
        match &self.kind {
            SpanKind::Tagged { attributes, .. } => attributes.first().map(|s| s.as_str()),
            _ => None,
        }
    }
}

/// Subject with parsed tags and content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Subject {
    /// Tags extracted from subject.
    pub tags: Vec<String>,
    /// Main content after tags.
    pub content: String,
}

impl Subject {
    /// Create from tags and content.
    pub fn new(tags: Vec<String>, content: String) -> Self {
        Self { tags, content }
    }

    /// Create from plain text.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            tags: Vec::new(),
            content: text.into(),
        }
    }

    /// Full display text including tags.
    pub fn full_text(&self) -> String {
        if self.tags.is_empty() {
            self.content.clone()
        } else {
            let tags: String = self.tags.iter().map(|t| format!("[{}]", t)).collect();
            format!("{} {}", tags, self.content)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_content_plain_text() {
        let content = PostContent::plain("Hello world");
        assert_eq!(content.to_plain_text(), "Hello world");
    }

    #[test]
    fn test_span_tagged() {
        let span = Span::tagged("b", vec![], vec![], vec![Span::plain("bold")]);
        assert!(span.is_tag("b"));
        assert_eq!(span.to_plain_text(), "bold");
    }

    #[test]
    fn test_subject() {
        let subject = Subject::new(vec!["News".into()], "Hello".into());
        assert_eq!(subject.full_text(), "[News] Hello");
    }
}
