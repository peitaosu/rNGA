//! BBCode parser for post content.

use crate::models::{PostContent, Span};

/// Parse BBCode content into structured spans.
pub fn parse_content(text: &str) -> PostContent {
    let text = unescape_html(text);
    let text = text.replace('\n', "<br/>");
    
    let (spans, error) = match Parser::new(&text).parse() {
        Ok(spans) => (spans, None),
        Err(e) => {
            let fallback = vec![Span::plain(text.replace("<br/>", "\n"))];
            (fallback, Some(e))
        }
    };
    
    PostContent {
        spans,
        raw: text,
        parse_error: error,
    }
}

/// Parse a subject/title line into tags and content.
pub fn parse_subject(text: &str) -> (Vec<String>, String) {
    let text = unescape_html(text);
    
    let mut tags = Vec::new();
    let mut remaining = text.as_str();
    
    while let Some(rest) = remaining.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let tag = rest[..end].trim();
            if !tag.is_empty() {
                tags.push(tag.to_owned());
            }
            remaining = rest[end + 1..].trim_start();
        } else {
            break;
        }
    }
    
    let content = if remaining.is_empty() && !tags.is_empty() {
        let last = tags.pop().unwrap();
        format!("[{}]", last)
    } else {
        remaining.to_owned()
    };
    
    (tags, content)
}

/// Unescape HTML entities.
fn unescape_html(text: &str) -> String {
    let first = html_escape::decode_html_entities(text);
    html_escape::decode_html_entities(&first).into_owned()
}

/// BBCode parser state machine.
struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }
    
    fn parse(&mut self) -> Result<Vec<Span>, String> {
        self.parse_spans(None)
    }
    
    fn parse_spans(&mut self, close_tag: Option<&str>) -> Result<Vec<Span>, String> {
        let mut spans = Vec::new();
        let mut plain_start = self.pos;
        
        while self.pos < self.input.len() {
            if let Some(tag) = close_tag {
                if self.looking_at_close_tag(tag) {
                    if plain_start < self.pos {
                        spans.push(Span::plain(&self.input[plain_start..self.pos]));
                    }
                    self.skip_close_tag(tag);
                    return Ok(spans);
                }
            }
            
            let start_pos = self.pos;
            
            if let Some(span) = self.try_parse_br() {
                if plain_start < start_pos {
                    spans.push(Span::plain(&self.input[plain_start..start_pos]));
                }
                spans.push(span);
                plain_start = self.pos;
                continue;
            }
            
            if let Some(span) = self.try_parse_divider() {
                if plain_start < start_pos {
                    spans.push(Span::plain(&self.input[plain_start..start_pos]));
                }
                spans.push(span);
                plain_start = self.pos;
                continue;
            }
            
            if self.current_char() == Some('[') {
                if let Some(span) = self.try_parse_sticker() {
                    if plain_start < start_pos {
                        spans.push(Span::plain(&self.input[plain_start..start_pos]));
                    }
                    spans.push(span);
                    plain_start = self.pos;
                    continue;
                }
                
                if let Some(span) = self.try_parse_at_mention() {
                    if plain_start < start_pos {
                        spans.push(Span::plain(&self.input[plain_start..start_pos]));
                    }
                    spans.push(span);
                    plain_start = self.pos;
                    continue;
                }
                
                if let Some(span) = self.try_parse_tag()? {
                    if plain_start < start_pos {
                        spans.push(Span::plain(&self.input[plain_start..start_pos]));
                    }
                    spans.push(span);
                    plain_start = self.pos;
                    continue;
                }
            }
            
            self.advance();
        }
        
        if plain_start < self.pos {
            spans.push(Span::plain(&self.input[plain_start..self.pos]));
        }
        
        Ok(spans)
    }
    
    fn current_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }
    
    fn advance(&mut self) {
        if let Some(c) = self.current_char() {
            self.pos += c.len_utf8();
        }
    }
    
    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }
    
    fn try_parse_br(&mut self) -> Option<Span> {
        let patterns = ["<br/>", "[stripbr]"];
        for pattern in patterns {
            if self.remaining().starts_with(pattern) {
                self.pos += pattern.len();
                return Some(Span::line_break());
            }
        }
        None
    }
    
    fn try_parse_divider(&mut self) -> Option<Span> {
        let remaining = self.remaining();
        
        let eq_count = remaining.chars().take_while(|&c| c == '=').count();
        if eq_count < 6 {
            return None;
        }
        
        if eq_count == remaining.len() || !remaining[eq_count..].chars().next().map_or(false, |c| c != '=' && c != '<' && c != '[') {
            self.pos += eq_count;
            return Some(Span::divider(Vec::new()));
        }
        
        let after_first = &remaining[eq_count..];
        if let Some(end_pos) = after_first.find("===") {
            let content = &after_first[..end_pos];
            let end_eq_count = after_first[end_pos..].chars().take_while(|&c| c == '=').count();
            
            let inner_spans = Parser::new(content).parse().unwrap_or_default();
            
            self.pos += eq_count + end_pos + end_eq_count;
            return Some(Span::divider(inner_spans));
        }
        
        self.pos += eq_count;
        Some(Span::divider(Vec::new()))
    }
    
    fn try_parse_sticker(&mut self) -> Option<Span> {
        let remaining = self.remaining();
        
        if !remaining.starts_with("[s:") {
            return None;
        }
        
        let end = remaining[3..].find(']')?;
        let name = &remaining[3..3 + end];
        self.pos += 4 + end;
        
        Some(Span::sticker(name))
    }
    
    fn try_parse_at_mention(&mut self) -> Option<Span> {
        let remaining = self.remaining();
        
        if !remaining.starts_with("[@") {
            return None;
        }
        
        let end = remaining[2..].find(']')?;
        let username = remaining[2..2 + end].trim();
        self.pos += 3 + end;
        
        Some(Span::tagged("at", vec![username.to_owned()], Vec::new(), Vec::new()))
    }
    
    fn looking_at_close_tag(&self, tag: &str) -> bool {
        let remaining = self.remaining();
        if !remaining.starts_with("[/") {
            return false;
        }
        
        let after_slash = &remaining[2..];
        if !after_slash.to_lowercase().starts_with(&tag.to_lowercase()) {
            return false;
        }
        
        let after_tag = &after_slash[tag.len()..];
        after_tag.starts_with(']')
    }
    
    fn skip_close_tag(&mut self, tag: &str) {
        self.pos += 3 + tag.len();
    }
    
    fn try_parse_tag(&mut self) -> Result<Option<Span>, String> {
        let remaining = self.remaining();
        
        if !remaining.starts_with('[') {
            return Ok(None);
        }
        
        if remaining.starts_with("[/") {
            return Ok(None);
        }
        
        let end = match remaining[1..].find(']') {
            Some(e) => e + 1,
            None => return Ok(None),
        };
        
        let tag_content = &remaining[1..end];
        
        let (tag_name, attrs, complex_attrs) = self.parse_tag_parts(tag_content);
        
        if tag_name.is_empty() {
            return Ok(None);
        }
        
        self.pos += end + 1;
        
        let inner_spans = self.parse_spans(Some(&tag_name))?;
        
        Ok(Some(Span::tagged(&tag_name, attrs, complex_attrs, inner_spans)))
    }
    
    fn parse_tag_parts(&self, content: &str) -> (String, Vec<String>, Vec<String>) {
        let content = content.trim();
        
        if let Some(space_pos) = content.find(' ') {
            let tag_name = content[..space_pos].to_lowercase();
            let rest = &content[space_pos + 1..];
            let complex_attrs: Vec<String> = rest.split(' ')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect();
            return (tag_name, Vec::new(), complex_attrs);
        }
        
        if let Some(eq_pos) = content.find('=') {
            let tag_name = content[..eq_pos].to_lowercase();
            let attrs_str = &content[eq_pos + 1..];
            let attrs: Vec<String> = attrs_str.split(',')
                .map(|s| s.trim().to_owned())
                .collect();
            return (tag_name, attrs, Vec::new());
        }
        
        (content.to_lowercase(), Vec::new(), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let content = parse_content("Hello world");
        assert_eq!(content.spans.len(), 1);
        assert!(matches!(&content.spans[0].kind, SpanKind::Plain { text } if text == "Hello world"));
    }

    #[test]
    fn test_line_break() {
        let content = parse_content("Hello<br/>World");
        assert_eq!(content.spans.len(), 3);
        assert!(matches!(&content.spans[1].kind, SpanKind::LineBreak));
    }

    #[test]
    fn test_simple_tag() {
        let content = parse_content("[b]bold text[/b]");
        assert_eq!(content.spans.len(), 1);
        if let SpanKind::Tagged { tag, spans, .. } = &content.spans[0].kind {
            assert_eq!(tag, "b");
            assert_eq!(spans.len(), 1);
        } else {
            panic!("Expected tagged span");
        }
    }

    #[test]
    fn test_tag_with_attribute() {
        let content = parse_content("[url=http://example.com]link[/url]");
        assert_eq!(content.spans.len(), 1);
        if let SpanKind::Tagged { tag, attributes, .. } = &content.spans[0].kind {
            assert_eq!(tag, "url");
            assert_eq!(attributes, &["http://example.com"]);
        } else {
            panic!("Expected tagged span");
        }
    }

    #[test]
    fn test_sticker() {
        let content = parse_content("[s:ac:doge]");
        assert_eq!(content.spans.len(), 1);
        if let SpanKind::Sticker { name } = &content.spans[0].kind {
            assert_eq!(name, "ac:doge");
        } else {
            panic!("Expected sticker span");
        }
    }

    #[test]
    fn test_at_mention() {
        let content = parse_content("[@username]");
        assert_eq!(content.spans.len(), 1);
        if let SpanKind::Tagged { tag, attributes, .. } = &content.spans[0].kind {
            assert_eq!(tag, "at");
            assert_eq!(attributes, &["username"]);
        } else {
            panic!("Expected at mention");
        }
    }

    #[test]
    fn test_divider() {
        let content = parse_content("======");
        assert_eq!(content.spans.len(), 1);
        assert!(matches!(&content.spans[0].kind, SpanKind::Divider { .. }));
    }

    #[test]
    fn test_nested_tags() {
        let content = parse_content("[quote][b]bold[/b][/quote]");
        assert_eq!(content.spans.len(), 1);
        if let SpanKind::Tagged { tag, spans, .. } = &content.spans[0].kind {
            assert_eq!(tag, "quote");
            assert_eq!(spans.len(), 1);
            if let SpanKind::Tagged { tag: inner_tag, .. } = &spans[0].kind {
                assert_eq!(inner_tag, "b");
            }
        } else {
            panic!("Expected nested tags");
        }
    }

    #[test]
    fn test_subject_parsing() {
        let (tags, content) = parse_subject("[News][Important] Hello World");
        assert_eq!(tags, vec!["News", "Important"]);
        assert_eq!(content, "Hello World");
    }

    #[test]
    fn test_subject_only_tag() {
        let (tags, content) = parse_subject("[SingleTag]");
        assert!(tags.is_empty());
        assert_eq!(content, "[SingleTag]");
    }

    #[test]
    fn test_unescape_double() {
        let text = "&amp;#128514;";
        let unescaped = unescape_html(text);
        assert_eq!(unescaped, "😂");
    }
}
