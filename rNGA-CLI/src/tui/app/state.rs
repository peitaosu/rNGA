use std::time::Duration;

use crate::handlers::forum::ForumInfo;
use crate::handlers::topic::{CliTopicDetailsResult, CliTopicListResult};

pub const FORUMS_WIDTH: u16 = 26;
pub const TOPICS_WIDTH: u16 = 40;
pub const AUTO_REFRESH_INTERVAL: Duration = Duration::from_secs(5);
pub const SEARCH_FIELD_HEIGHT: u16 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search(Pane),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefreshCascade {
    None,
    FromForums,
    FromTopics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Forums,
    Topics,
    Thread,
}

#[derive(Debug, Clone)]
pub enum ForumRow {
    Header(String),
    Favorite(ForumInfo),
    Forum(ForumInfo),
}

#[derive(Debug, Clone)]
pub struct SelectedForum {
    pub id: String,
    pub is_stid: bool,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct ThreadLayout {
    pub post_starts: Vec<usize>,
    pub line_count: usize,
}

pub enum TaskResult {
    Forums(u64, Result<ForumLoadOutcome, String>),
    Topics(u64, Result<CliTopicListResult, String>),
    Thread(u64, Result<CliTopicDetailsResult, String>),
}

pub(crate) struct ForumLoadOutcome {
    pub(crate) rows: Vec<ForumRow>,
    pub(crate) favorites: Vec<ForumInfo>,
    pub(crate) favorites_error: Option<String>,
}
