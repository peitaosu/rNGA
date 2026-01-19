//! XML parsing for NGA responses.

use crate::error::{Error, Result};
use std::collections::HashMap;
use sxd_document::parser;
use sxd_xpath::{nodeset::Node, Context, Factory, Value};

/// Wrapper around an XML document providing convenient access methods.
pub struct XmlDocument {
    package: sxd_document::Package,
}

impl XmlDocument {
    /// Parse XML text into a document.
    pub fn parse(xml: &str) -> Result<Self> {
        check_nga_error(xml)?;

        let package = parser::parse(xml).map_err(|e| Error::Xml(e.to_string()))?;
        Ok(Self { package })
    }

    /// Get the root document.
    fn doc(&self) -> sxd_document::dom::Document<'_> {
        self.package.as_document()
    }

    /// Evaluate an XPath expression.
    pub fn xpath(&self, expr: &str) -> Result<XPathResult<'_>> {
        let factory = Factory::new();
        let xpath = factory
            .build(expr)
            .map_err(|e| Error::XPath(e.to_string()))?
            .ok_or_else(|| Error::XPath("Empty XPath".into()))?;

        let context = Context::new();
        let value = xpath
            .evaluate(&context, self.doc().root())
            .map_err(|e| Error::XPath(e.to_string()))?;

        Ok(XPathResult { value })
    }

    /// Select all matching nodes.
    pub fn select(&self, expr: &str) -> Result<Vec<XmlNode<'_>>> {
        let result = self.xpath(expr)?;
        match result.value {
            Value::Nodeset(ns) => Ok(ns.document_order().into_iter().map(XmlNode).collect()),
            _ => Ok(vec![]),
        }
    }

    /// Select the first matching node.
    pub fn select_one(&self, expr: &str) -> Result<Option<XmlNode<'_>>> {
        let nodes = self.select(expr)?;
        Ok(nodes.into_iter().next())
    }

    /// Get a string value from an XPath.
    pub fn string(&self, expr: &str) -> Result<String> {
        let result = self.xpath(expr)?;
        Ok(result.as_string())
    }

    /// Get an optional string from an XPath.
    pub fn string_opt(&self, expr: &str) -> Option<String> {
        self.string(expr).ok().filter(|s| !s.is_empty())
    }

    /// Get an integer from an XPath.
    pub fn int(&self, expr: &str) -> Result<i64> {
        let s = self.string(expr)?;
        parse_int(&s)
    }

    /// Get an integer with default.
    pub fn int_or(&self, expr: &str, default: i64) -> i64 {
        self.int(expr).unwrap_or(default)
    }
}

/// XPath evaluation result.
pub struct XPathResult<'a> {
    value: Value<'a>,
}

impl<'a> XPathResult<'a> {
    /// Get as string.
    pub fn as_string(&self) -> String {
        match &self.value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Nodeset(ns) => ns
                .document_order()
                .into_iter()
                .next()
                .map(|n| XmlNode(n).text())
                .unwrap_or_default(),
        }
    }
}

/// Wrapper around an XML node with convenient attribute access.
#[derive(Clone, Copy)]
pub struct XmlNode<'a>(Node<'a>);

impl<'a> XmlNode<'a> {
    /// Get an attribute value.
    pub fn attr(&self, name: &str) -> Option<String> {
        self.as_element()
            .and_then(|e| e.attribute_value(name))
            .map(|s| s.to_owned())
    }

    /// Get a required attribute value.
    pub fn require_attr(&self, name: &str) -> Result<String> {
        self.attr(name).ok_or_else(|| Error::missing(name))
    }

    /// Get an attribute as integer.
    pub fn attr_int(&self, name: &str) -> Option<i64> {
        self.attr(name).and_then(|s| parse_int(&s).ok())
    }

    /// Get an attribute as integer with default.
    pub fn attr_int_or(&self, name: &str, default: i64) -> i64 {
        self.attr_int(name).unwrap_or(default)
    }

    /// Get an attribute as boolean.
    pub fn attr_bool(&self, name: &str) -> bool {
        self.attr(name)
            .map(|s| s == "1" || s.to_lowercase() == "true")
            .unwrap_or(false)
    }

    /// Get all values as a map.
    pub fn attrs(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();

        if let Some(e) = self.as_element() {
            for attr in e.attributes() {
                map.insert(attr.name().local_part().to_owned(), attr.value().to_owned());
            }

            for child in e.children() {
                if let sxd_document::dom::ChildOfElement::Element(child_el) = child {
                    let name = child_el.name().local_part().to_owned();
                    let text = XmlNode(Node::Element(child_el)).text();
                    map.insert(name, text);
                }
            }
        }

        map
    }

    /// Get text content.
    pub fn text(&self) -> String {
        match self.0 {
            Node::Text(t) => t.text().to_owned(),
            Node::Element(e) => e
                .children()
                .iter()
                .filter_map(|c| {
                    if let sxd_document::dom::ChildOfElement::Text(t) = c {
                        Some(t.text())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(""),
            _ => String::new(),
        }
    }

    /// Get element name.
    pub fn name(&self) -> &str {
        self.as_element()
            .map(|e| e.name().local_part())
            .unwrap_or("")
    }

    /// Get as element.
    fn as_element(&self) -> Option<sxd_document::dom::Element<'a>> {
        match self.0 {
            Node::Element(e) => Some(e),
            _ => None,
        }
    }

    /// Select child nodes.
    pub fn children(&self) -> Vec<XmlNode<'a>> {
        self.as_element()
            .map(|e| {
                e.children()
                    .iter()
                    .filter_map(|c| match c {
                        sxd_document::dom::ChildOfElement::Element(el) => {
                            Some(XmlNode(Node::Element(*el)))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Select children with specific name.
    pub fn children_named(&self, name: &str) -> Vec<XmlNode<'a>> {
        self.children()
            .into_iter()
            .filter(|n| n.name() == name)
            .collect()
    }

    /// Get first child with name.
    pub fn child_named(&self, name: &str) -> Option<XmlNode<'a>> {
        self.children_named(name).into_iter().next()
    }
}

/// Parse integer.
fn parse_int(s: &str) -> Result<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(0);
    }

    if s.contains('e') || s.contains('E') {
        let f: f64 = s
            .parse()
            .map_err(|_| Error::parse(format!("Invalid number: {}", s)))?;
        return Ok(f as i64);
    }

    if s.contains('.') {
        let f: f64 = s
            .parse()
            .map_err(|_| Error::parse(format!("Invalid number: {}", s)))?;
        return Ok(f as i64);
    }

    s.parse()
        .map_err(|_| Error::parse(format!("Invalid integer: {}", s)))
}

/// Check if XML response contains an NGA error.
fn check_nga_error(xml: &str) -> Result<()> {
    if !xml.contains("__error") && !xml.contains("error code=") {
        return Ok(());
    }

    if let Some(code_start) = xml.find("code=\"") {
        let code_start = code_start + 6;
        if let Some(code_end) = xml[code_start..].find('"') {
            let code = &xml[code_start..code_start + code_end];

            let message = if let Some(msg_start) = xml.find("message=\"") {
                let msg_start = msg_start + 9;
                xml[msg_start..]
                    .find('"')
                    .map(|end| &xml[msg_start..msg_start + end])
                    .unwrap_or("")
            } else {
                "Unknown error"
            };

            let message = html_escape::decode_html_entities(message);

            return Err(Error::nga(code, message.as_ref()));
        }
    }

    if xml.contains("__error") {
        return Err(Error::nga("-1", "Error response received"));
    }

    Ok(())
}

/// Extract key-value pairs from tab-separated format.
pub fn extract_kv(text: &str) -> HashMap<String, String> {
    let parts: Vec<&str> = text.split('\t').collect();
    let mut map = HashMap::new();

    for chunk in parts.chunks(2) {
        if chunk.len() == 2 {
            let key = chunk[0].trim();
            let value = chunk[1].trim();
            if !key.is_empty() {
                map.insert(key.to_owned(), value.to_owned());
            }
        }
    }

    map
}

/// Parse NGA timestamp.
pub fn parse_timestamp(s: &str) -> Option<i64> {
    parse_int(s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int() {
        assert_eq!(parse_int("123").unwrap(), 123);
        assert_eq!(parse_int("-456").unwrap(), -456);
        assert_eq!(parse_int("1.5e9").unwrap(), 1500000000);
        assert_eq!(parse_int("123.45").unwrap(), 123);
        assert_eq!(parse_int("").unwrap(), 0);
    }

    #[test]
    fn test_extract_kv() {
        let text = "key1\tval1\tkey2\tval2\tkey3\tval3";
        let kv = extract_kv(text);
        assert_eq!(kv.get("key1"), Some(&"val1".to_owned()));
        assert_eq!(kv.get("key2"), Some(&"val2".to_owned()));
        assert_eq!(kv.get("key3"), Some(&"val3".to_owned()));
    }

    #[test]
    fn test_xml_parse() {
        let xml = r#"<?xml version="1.0"?><root><item id="1" name="test"/></root>"#;
        let doc = XmlDocument::parse(xml).unwrap();

        let items = doc.select("//item").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].attr("id"), Some("1".to_owned()));
        assert_eq!(items[0].attr("name"), Some("test".to_owned()));
    }

    #[test]
    fn test_nga_error_detection() {
        let error_xml = r#"<error code="1" message="Not logged in"/>"#;
        let result = check_nga_error(error_xml);
        assert!(result.is_err());

        let ok_xml = r#"<data><item id="1"/></data>"#;
        assert!(check_nga_error(ok_xml).is_ok());
    }
}
