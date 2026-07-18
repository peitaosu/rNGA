//! Parsers for NGA responses.

pub mod bbcode;
pub mod user;
pub mod xml;

pub use bbcode::{parse_content, parse_subject};
pub use user::{parse_user_from_attrs, parse_user_from_node};
pub use xml::{compute_total_pages, extract_kv, parse_timestamp, XmlDocument, XmlNode};
