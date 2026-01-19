//! Parsers for NGA responses.

pub mod bbcode;
pub mod xml;

pub use bbcode::{parse_content, parse_subject};
pub use xml::{extract_kv, parse_timestamp, XmlDocument, XmlNode};
