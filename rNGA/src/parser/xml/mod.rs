//! XML parsing for NGA responses.
//!
//! Public API is backend-agnostic. The default backend is `sxd`; evaluated
//! replacements include `simdxml` and `xmloxide`.

mod sxd;

pub use sxd::{compute_total_pages, extract_kv, parse_timestamp, XmlDocument, XmlNode};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_total_pages() {
        assert_eq!(compute_total_pages(0, 20), 1);
        assert_eq!(compute_total_pages(20, 20), 1);
        assert_eq!(compute_total_pages(21, 20), 2);
        assert_eq!(compute_total_pages(35, 35), 1);
        assert_eq!(compute_total_pages(36, 35), 2);
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
        let result = XmlDocument::parse(error_xml);
        assert!(result.is_err());

        let ok_xml = r#"<data><item id="1"/></data>"#;
        assert!(XmlDocument::parse(ok_xml).is_ok());
    }
}
