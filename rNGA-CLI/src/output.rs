//! Output formatting.

use chrono::{Local, TimeZone};
use clap::ValueEnum;
use colored::Colorize;
use comfy_table::{Table, ContentArrangement, presets::UTF8_FULL_CONDENSED};
use rnga::models::*;
use serde::Serialize;

/// Output format options.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Pretty table format
    Table,
    /// JSON format
    Json,
    /// Plain text format
    #[default]
    Plain,
}

/// Trait for plain text output.
pub trait PlainPrint {
    /// Print as plain text with formatting.
    fn plain_print(&self);
}

/// Trait for table row generation.
pub trait TableRow {
    /// Get table headers.
    fn headers() -> Vec<&'static str>;
    /// Get row data as strings.
    fn row(&self) -> Vec<String>;
}

/// Print items in plain text format.
pub fn print_plain<T: PlainPrint>(items: &[T]) {
    if items.is_empty() {
        println!("No results");
        return;
    }
    for item in items {
        item.plain_print();
    }
}

/// Format a Unix timestamp for display.
pub fn format_time(timestamp: i64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }

    let dt = Local.timestamp_opt(timestamp, 0).single();
    match dt {
        Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
        None => "-".to_string(),
    }
}

/// Format a relative time for display.
pub fn format_relative_time(timestamp: i64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }

    let now = Local::now().timestamp();
    let diff = now - timestamp;

    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m {}s ago", diff / 60, diff % 60)
    } else if diff < 86400 {
        format!("{}h {}m {}s ago", diff / 3600, (diff % 3600) / 60, diff % 60)
    } else if diff < 2592000 {
        format!("{}d {}h {}m {}s ago", diff / 86400, (diff % 86400) / 3600, (diff % 3600) / 60, diff % 60)
    } else {
        format_time(timestamp)
    }
}

/// Print a table of items with proper formatting for each output mode.
pub fn print_table<T: TableRow + Serialize + PlainPrint>(items: Vec<T>, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&items).unwrap_or_default());
        }
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No results");
                return;
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(T::headers());
            for item in &items {
                table.add_row(item.row());
            }
            println!("{table}");
        }
        OutputFormat::Plain => {
            print_plain(&items);
        }
    }
}

// ============================================================================
// Display implementations for models
// ============================================================================

/// Row for forum list display.
#[derive(Serialize)]
pub struct ForumRow {
    pub id: String,
    pub name: String,
    pub info: String,
}

impl From<&Forum> for ForumRow {
    fn from(f: &Forum) -> Self {
        Self {
            id: f.id.as_ref().map(|id| id.id().to_string()).unwrap_or_default(),
            name: f.name.clone(),
            info: f.info.clone(),
        }
    }
}

impl TableRow for ForumRow {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Name", "Info"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.id.clone(), self.name.clone(), self.info.clone()]
    }
}

impl PlainPrint for ForumRow {
    fn plain_print(&self) {
        println!("[{}] {}", self.id.cyan(), self.name.bold());
        if !self.info.is_empty() {
            println!("   {}", self.info.dimmed());
        }
    }
}

/// Row for category display.
#[derive(Serialize)]
pub struct CategoryRow {
    pub name: String,
    pub forum_count: usize,
}

impl From<&Category> for CategoryRow {
    fn from(c: &Category) -> Self {
        Self {
            name: c.name.clone(),
            forum_count: c.forums.len(),
        }
    }
}

impl TableRow for CategoryRow {
    fn headers() -> Vec<&'static str> {
        vec!["Category", "Forums"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.name.clone(), self.forum_count.to_string()]
    }
}

impl PlainPrint for CategoryRow {
    fn plain_print(&self) {
        println!("{} ({})", self.name.bold(), format!("{} forums", self.forum_count).dimmed());
    }
}

/// Row for topic list display.
#[derive(Serialize)]
pub struct TopicRow {
    pub id: String,
    pub subject: String,
    pub author: String,
    pub author_id: String,
    pub replies: i32,
    pub last_post: String,
    pub post_date: i64,
    pub last_post_date: i64,
}

impl From<&Topic> for TopicRow {
    fn from(t: &Topic) -> Self {
        Self {
            id: t.id.to_string(),
            subject: t.subject.content.clone(),
            author: t.author.name.display().to_string(),
            author_id: t.author.id.to_string(),
            replies: t.replies,
            last_post: format_relative_time(t.last_post_date),
            post_date: t.post_date,
            last_post_date: t.last_post_date,
        }
    }
}

impl TableRow for TopicRow {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Subject", "Author", "Replies", "Last Post"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.subject.clone(),
            self.author.clone(),
            self.replies.to_string(),
            self.last_post.clone(),
        ]
    }
}

impl PlainPrint for TopicRow {
    fn plain_print(&self) {
        println!(
            "{} {}",
            format!("[Topic {}]", self.id).cyan(),
            self.subject.bold()
        );
        println!(
            "   By {} {} | {} | {} replies",
            self.author.green(),
            format!("[UID: {}]", self.author_id).dimmed(),
            self.last_post.dimmed(),
            self.replies
        );
    }
}

/// Row for post display.
#[derive(Serialize)]
pub struct PostRow {
    pub floor: i32,
    pub post_id: String,
    pub topic_id: String,
    pub author: String,
    pub author_id: String,
    pub content: String,
    pub score: i32,
    pub time: String,
    pub post_date: i64,
}

impl From<&Post> for PostRow {
    fn from(p: &Post) -> Self {
        Self {
            floor: p.floor,
            post_id: p.id.to_string(),
            topic_id: p.topic_id.to_string(),
            author: p.author.name.display().to_string(),
            author_id: p.author.id.to_string(),
            content: p.content.to_plain_text(),
            score: p.score,
            time: format_relative_time(p.post_date),
            post_date: p.post_date,
        }
    }
}

impl TableRow for PostRow {
    fn headers() -> Vec<&'static str> {
        vec!["#", "Author", "Content", "Score", "Time"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.floor.to_string(),
            self.author.clone(),
            self.content.clone(),
            self.score.to_string(),
            self.time.clone(),
        ]
    }
}

impl PlainPrint for PostRow {
    fn plain_print(&self) {
        println!(
            "{} {} {} {}{}",
            format!("#{}", self.floor).yellow(),
            self.author.green(),
            format!("[UID: {}]", self.author_id).dimmed(),
            self.time.dimmed(),
            if self.score != 0 {
                format!(" (score: {})", self.score).dimmed().to_string()
            } else {
                String::new()
            }
        );
        for line in self.content.lines() {
            if !line.trim().is_empty() {
                println!("     {}", line);
            }
        }
        println!();
    }
}

/// Row for user display.
#[derive(Serialize)]
pub struct UserRow {
    pub id: String,
    pub name: String,
    pub reputation: i32,
    pub posts: i32,
    pub reg_date: String,
}

impl From<&User> for UserRow {
    fn from(u: &User) -> Self {
        Self {
            id: u.id.to_string(),
            name: u.name.display().to_string(),
            reputation: u.reputation,
            posts: u.posts,
            reg_date: format_time(u.reg_date),
        }
    }
}

impl TableRow for UserRow {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Name", "Reputation", "Posts", "Registered"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.reputation.to_string(),
            self.posts.to_string(),
            self.reg_date.clone(),
        ]
    }
}

impl PlainPrint for UserRow {
    fn plain_print(&self) {
        println!(
            "{} {}",
            format!("[UID: {}]", self.id).cyan(),
            self.name.bold()
        );
        println!(
            "   Rep: {} | Posts: {} | Registered: {}",
            self.reputation,
            self.posts,
            self.reg_date.dimmed()
        );
    }
}

/// Row for notification display.
#[derive(Serialize)]
pub struct NotificationRow {
    pub id: String,
    pub kind: String,
    pub content: String,
    pub from: String,
    pub from_uid: String,
    pub time: String,
    pub timestamp: i64,
    pub topic_id: String,
    pub post_id: String,
}

impl From<&Notification> for NotificationRow {
    fn from(n: &Notification) -> Self {
        Self {
            id: n.id.clone(),
            kind: format!("{:?}", n.kind),
            content: n.content.clone(),
            from: n.from_username.clone().unwrap_or_default(),
            from_uid: n.from_user_id.as_ref().map(|u| u.to_string()).unwrap_or_default(),
            time: format_relative_time(n.time),
            timestamp: n.time,
            topic_id: n.topic_id.as_ref().map(|t| t.to_string()).unwrap_or_default(),
            post_id: n.post_id.as_ref().map(|p| p.to_string()).unwrap_or_default(),
        }
    }
}

impl TableRow for NotificationRow {
    fn headers() -> Vec<&'static str> {
        vec!["Type", "Content", "From", "Time"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.kind.clone(),
            self.content.clone(),
            self.from.clone(),
            self.time.clone(),
        ]
    }
}

impl PlainPrint for NotificationRow {
    fn plain_print(&self) {
        let from_display = if !self.from.is_empty() {
            format!(" from {}", self.from.green())
        } else {
            String::new()
        };
        println!(
            "[{}]{} {}",
            self.kind.cyan(),
            from_display,
            self.time.dimmed()
        );
        if !self.content.is_empty() {
            for line in self.content.lines() {
                if !line.trim().is_empty() {
                    println!("   {}", line);
                }
            }
        }
    }
}

/// Row for notification counts.
#[derive(Serialize)]
pub struct NotificationCountRow {
    pub kind: String,
    pub count: i32,
}

impl TableRow for NotificationCountRow {
    fn headers() -> Vec<&'static str> {
        vec!["Type", "Count"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.kind.clone(), self.count.to_string()]
    }
}

impl PlainPrint for NotificationCountRow {
    fn plain_print(&self) {
        let count_display = if self.count > 0 {
            self.count.to_string().yellow().to_string()
        } else {
            self.count.to_string().dimmed().to_string()
        };
        println!("{}: {}", self.kind, count_display);
    }
}

/// Row for message conversation display.
#[derive(Serialize)]
pub struct MessageRow {
    pub id: String,
    pub other_user: String,
    pub other_uid: String,
    pub subject: String,
    pub last_time: String,
    pub last_timestamp: i64,
    pub unread: String,
    pub is_unread: bool,
}

impl From<&ShortMessage> for MessageRow {
    fn from(m: &ShortMessage) -> Self {
        Self {
            id: m.id.clone(),
            other_user: m.other_username.clone(),
            other_uid: m.other_user_id.to_string(),
            subject: m.subject.clone(),
            last_time: format_relative_time(m.last_time),
            last_timestamp: m.last_time,
            unread: if m.is_unread { "●".to_string() } else { "".to_string() },
            is_unread: m.is_unread,
        }
    }
}

impl TableRow for MessageRow {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "With", "Subject", "Last", "Unread"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.other_user.clone(),
            self.subject.clone(),
            self.last_time.clone(),
            self.unread.clone(),
        ]
    }
}

impl PlainPrint for MessageRow {
    fn plain_print(&self) {
        let unread_marker = if self.is_unread { "● ".red().to_string() } else { String::new() };
        println!(
            "{}[{}] {} {}",
            unread_marker,
            self.id.cyan(),
            self.other_user.green(),
            self.last_time.dimmed()
        );
        println!("   {}", self.subject.bold());
    }
}

/// Row for message post display.
#[derive(Serialize)]
pub struct MessagePostRow {
    pub id: String,
    pub from: String,
    pub from_uid: String,
    pub is_mine: bool,
    pub content: String,
    pub time: String,
    pub timestamp: i64,
}

impl From<&ShortMessagePost> for MessagePostRow {
    fn from(p: &ShortMessagePost) -> Self {
        Self {
            id: p.id.clone(),
            from: if p.is_mine {
                "You".to_string()
            } else {
                p.from_username.clone()
            },
            from_uid: p.from_user_id.to_string(),
            is_mine: p.is_mine,
            content: p.content.to_plain_text(),
            time: format_relative_time(p.time),
            timestamp: p.time,
        }
    }
}

impl TableRow for MessagePostRow {
    fn headers() -> Vec<&'static str> {
        vec!["From", "Content", "Time"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.from.clone(), self.content.clone(), self.time.clone()]
    }
}

impl PlainPrint for MessagePostRow {
    fn plain_print(&self) {
        let from_display = if self.is_mine {
            "You".green().to_string()
        } else {
            self.from.clone()
        };
        println!("{} {}", from_display, self.time.dimmed());
        for line in self.content.lines() {
            if !line.trim().is_empty() {
                println!("   {}", line);
            }
        }
        println!();
    }
}

/// Row for favorite folder display.
#[derive(Serialize)]
pub struct FolderRow {
    pub id: String,
    pub name: String,
    pub count: i32,
}

impl From<&FavoriteFolder> for FolderRow {
    fn from(f: &FavoriteFolder) -> Self {
        Self {
            id: f.id.clone(),
            name: f.name.clone(),
            count: f.count,
        }
    }
}

impl TableRow for FolderRow {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Name", "Topics"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.id.clone(), self.name.clone(), self.count.to_string()]
    }
}

impl PlainPrint for FolderRow {
    fn plain_print(&self) {
        println!(
            "[{}] {} {}",
            self.id.cyan(),
            self.name.bold(),
            format!("({} topics)", self.count).dimmed()
        );
    }
}

/// Row for light post display.
#[derive(Serialize)]
pub struct LightPostRow {
    pub author: String,
    pub author_id: String,
    pub content: String,
    pub score: i32,
    pub time: String,
    pub post_date: i64,
}

impl From<&LightPost> for LightPostRow {
    fn from(p: &LightPost) -> Self {
        Self {
            author: p.author.name.display().to_string(),
            author_id: p.author.id.to_string(),
            content: p.content.to_plain_text(),
            score: p.score,
            time: format_relative_time(p.post_date),
            post_date: p.post_date,
        }
    }
}

impl TableRow for LightPostRow {
    fn headers() -> Vec<&'static str> {
        vec!["Author", "Content", "Score", "Time"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.author.clone(),
            self.content.clone(),
            self.score.to_string(),
            self.time.clone(),
        ]
    }
}

impl PlainPrint for LightPostRow {
    fn plain_print(&self) {
        println!(
            "{} {} {}{}",
            self.author.green(),
            format!("[UID: {}]", self.author_id).dimmed(),
            self.time.dimmed(),
            if self.score != 0 {
                format!(" (+{})", self.score).yellow().to_string()
            } else {
                String::new()
            }
        );
        for line in self.content.lines() {
            if !line.trim().is_empty() {
                println!("   {}", line);
            }
        }
        println!();
    }
}

/// Row for user post display.
#[derive(Serialize)]
pub struct UserPostRow {
    pub post_id: String,
    pub topic_id: String,
    pub topic_subject: String,
    pub content_preview: String,
}

impl TableRow for UserPostRow {
    fn headers() -> Vec<&'static str> {
        vec!["Post ID", "Topic ID", "Subject", "Preview"]
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.post_id.clone(),
            self.topic_id.clone(),
            self.topic_subject.clone(),
            self.content_preview.clone(),
        ]
    }
}

impl PlainPrint for UserPostRow {
    fn plain_print(&self) {
        println!(
            "{} in topic {}",
            format!("[{}]", self.post_id).yellow(),
            self.topic_id
        );
        println!("   {}", self.topic_subject.dimmed());
        println!("   {}", self.content_preview);
        println!();
    }
}

/// Row for user search result display.
#[derive(Serialize)]
pub struct UserSearchRow {
    pub id: String,
    pub name: String,
}

impl TableRow for UserSearchRow {
    fn headers() -> Vec<&'static str> {
        vec!["ID", "Name"]
    }
    fn row(&self) -> Vec<String> {
        vec![self.id.clone(), self.name.clone()]
    }
}

impl PlainPrint for UserSearchRow {
    fn plain_print(&self) {
        println!("{}: {}", self.id, self.name.green());
    }
}
