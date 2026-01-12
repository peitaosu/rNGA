//! Parsers for NGA responses.

pub mod bbcode;
pub mod xml;

pub use bbcode::{parse_content, parse_subject};
pub use xml::{XmlDocument, XmlNode, extract_kv, parse_timestamp};
