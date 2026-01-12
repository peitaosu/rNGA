//! Topic API.

use std::sync::Arc;

use crate::{
    client::NGAClientInner,
    error::{Error, Result},
    models::{
        Attachment, AttachmentKind, FavoriteFolder, FavoriteTopicOp, Forum, ForumIdKind, Post,
        SearchTimeRange, Subject, Topic, TopicId, TopicOrder, TopicType, User, UserName,
    },
    parser::{parse_subject, XmlDocument, XmlNode},
};

/// API for topic operations.
pub struct TopicApi {
    client: Arc<NGAClientInner>,
}

impl TopicApi {
    pub(crate) fn new(client: Arc<NGAClientInner>) -> Self {
        Self { client }
    }

    /// Get topics from a forum.
    pub fn list(&self, forum_id: ForumIdKind) -> TopicListBuilder {
        TopicListBuilder {
            client: self.client.clone(),
            forum_id,
            page: 1,
            order: TopicOrder::default(),
            recommended_only: false,
        }
    }

    /// Get topic details and posts.
    pub fn details(&self, topic_id: impl Into<TopicId>) -> TopicDetailsBuilder {
        TopicDetailsBuilder {
            client: self.client.clone(),
            topic_id: topic_id.into(),
            page: 1,
            fav: None,
            post_id: None,
            author_id: None,
            anonymous_only: false,
        }
    }

    /// Search topics in a forum.
    pub fn search(&self, forum_id: ForumIdKind, keyword: &str) -> TopicSearchBuilder {
        TopicSearchBuilder {
            client: self.client.clone(),
            forum_id,
            keyword: keyword.to_owned(),
            page: 1,
            search_content: false,
            recommended_only: false,
            time_range: SearchTimeRange::default(),
        }
    }

    /// Get favorite topics.
    pub fn favorites(&self) -> FavoriteTopicsBuilder {
        FavoriteTopicsBuilder {
            client: self.client.clone(),
            folder_id: None,
            page: 1,
        }
    }

    /// Get favorite folders.
    pub async fn favorite_folders(&self) -> Result<Vec<FavoriteFolder>> {
        let xml = self.client.post_authed(
            "nuke.php",
            &[
                ("__lib", "topic_favor_v2"),
                ("__act", "list_folder"),
                ("page", "1"),
            ],
            &[],
        ).await?;

        let doc = XmlDocument::parse(&xml)?;
        let mut folders = Vec::new();

        for node in doc.select("/root/data/item/item")? {
            let attrs = node.attrs();
            if let Some(id) = attrs.get("id") {
                folders.push(FavoriteFolder {
                    id: id.clone(),
                    name: attrs.get("name").cloned().unwrap_or_default(),
                    count: attrs.get("length")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0),
                });
            }
        }

        Ok(folders)
    }

    /// Add or remove topic from favorites.
    pub async fn modify_favorite(
        &self,
        topic_id: impl AsRef<str>,
        folder_id: impl AsRef<str>,
        op: FavoriteTopicOp,
    ) -> Result<()> {
        let (act, tid_key) = match op {
            FavoriteTopicOp::Add => ("add", "tid"),
            FavoriteTopicOp::Remove => ("del", "tidarray"),
        };

        self.client.post_authed(
            "nuke.php",
            &[("__lib", "topic_favor_v2"), ("__act", act)],
            &[(tid_key, topic_id.as_ref()), ("folder", folder_id.as_ref())],
        ).await?;

        Ok(())
    }

    /// Get topics posted by a specific user.
    pub async fn by_user(&self, user_id: impl AsRef<str>, page: u32) -> Result<TopicListResult> {
        let page_str = page.to_string();
        let xml = self.client.post(
            "thread.php",
            &[("authorid", user_id.as_ref()), ("page", &page_str)],
            &[],
        ).await?;

        parse_topic_list_response(&xml)
    }
}

/// Builder for topic list requests.
pub struct TopicListBuilder {
    client: Arc<NGAClientInner>,
    forum_id: ForumIdKind,
    page: u32,
    order: TopicOrder,
    recommended_only: bool,
}

impl TopicListBuilder {
    /// Set the page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Set the sort order.
    pub fn order(mut self, order: TopicOrder) -> Self {
        self.order = order;
        self
    }

    /// Only show recommended topics.
    pub fn recommended_only(mut self, recommended: bool) -> Self {
        self.recommended_only = recommended;
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<TopicListResult> {
        let page_str = self.page.to_string();
        let recommend_str = if self.recommended_only { "1" } else { "" };

        let xml = self.client.post(
            "thread.php",
            &[
                (self.forum_id.param_name(), self.forum_id.id()),
                ("page", &page_str),
                ("order_by", self.order.param()),
                ("recommend", recommend_str),
            ],
            &[],
        ).await?;

        let mut result = parse_topic_list_response(&xml)?;

        let doc = XmlDocument::parse(&xml)?;
        for node in doc.select("/root/__F/sub_forums/*")? {
            if let Some(subforum) = parse_subforum(&node) {
                result.subforums.push(subforum);
            }
        }

        if let Ok(Some(forum_node)) = doc.select_one("/root/__F") {
            result.forum = parse_forum_from_node(&forum_node);
        }

        Ok(result)
    }
}

/// Result of a topic list request.
#[derive(Debug, Clone, Default)]
pub struct TopicListResult {
    /// Topics in the list.
    pub topics: Vec<Topic>,
    /// Forum information.
    pub forum: Option<Forum>,
    /// Subforums.
    pub subforums: Vec<Subforum>,
    /// Total number of pages.
    pub total_pages: u32,
    /// Current page.
    pub page: u32,
}

/// A subforum entry.
#[derive(Debug, Clone, Default)]
pub struct Subforum {
    /// Forum information.
    pub forum: Forum,
    /// Filter ID for blocking.
    pub filter_id: String,
    /// Whether this subforum can be filtered.
    pub filterable: bool,
    /// Whether this subforum is currently selected/active.
    pub selected: bool,
}

/// Builder for topic details requests.
pub struct TopicDetailsBuilder {
    client: Arc<NGAClientInner>,
    topic_id: TopicId,
    page: u32,
    fav: Option<String>,
    post_id: Option<String>,
    author_id: Option<String>,
    anonymous_only: bool,
}

impl TopicDetailsBuilder {
    /// Set the page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Set the fav parameter.
    pub fn fav(mut self, fav: impl Into<String>) -> Self {
        self.fav = Some(fav.into());
        self
    }

    /// Jump to a specific post.
    pub fn post(mut self, post_id: impl Into<String>) -> Self {
        self.post_id = Some(post_id.into());
        self
    }

    /// Filter to posts by a specific author.
    pub fn author(mut self, author_id: impl Into<String>) -> Self {
        self.author_id = Some(author_id.into());
        self
    }

    /// Only show anonymous posts.
    pub fn anonymous_only(mut self, only: bool) -> Self {
        self.anonymous_only = only;
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<TopicDetailsResult> {
        let page_str = self.page.to_string();
        let opt = if self.anonymous_only { "512" } else { "" };

        let xml = self.client.post(
            "read.php",
            &[
                ("tid", self.topic_id.as_str()),
                ("page", &page_str),
                ("fav", self.fav.as_deref().unwrap_or("")),
                ("pid", self.post_id.as_deref().unwrap_or("")),
                ("authorid", self.author_id.as_deref().unwrap_or("")),
                ("opt", opt),
            ],
            &[],
        ).await?;

        parse_topic_details_response(&xml, self.page)
    }
}

/// Result of a topic details request.
#[derive(Debug, Clone, Default)]
pub struct TopicDetailsResult {
    /// The topic.
    pub topic: Topic,
    /// Posts/replies.
    pub posts: Vec<Post>,
    /// Forum name.
    pub forum_name: String,
    /// Total number of pages.
    pub total_pages: u32,
    /// Current page.
    pub page: u32,
}

/// Builder for topic search requests.
pub struct TopicSearchBuilder {
    client: Arc<NGAClientInner>,
    forum_id: ForumIdKind,
    keyword: String,
    page: u32,
    search_content: bool,
    recommended_only: bool,
    time_range: SearchTimeRange,
}

impl TopicSearchBuilder {
    /// Set the page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Search in post content, not just titles.
    pub fn search_content(mut self, search: bool) -> Self {
        self.search_content = search;
        self
    }

    /// Only show recommended topics.
    pub fn recommended_only(mut self, recommended: bool) -> Self {
        self.recommended_only = recommended;
        self
    }

    /// Set time range filter.
    pub fn time_range(mut self, range: SearchTimeRange) -> Self {
        self.time_range = range;
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<TopicListResult> {
        let page_str = self.page.to_string();
        let content_str = if self.search_content { "1" } else { "" };
        let recommend_str = if self.recommended_only { "1" } else { "" };

        let xml = self.client.post(
            "thread.php",
            &[
                (self.forum_id.param_name(), self.forum_id.id()),
                ("key", &self.keyword),
                ("page", &page_str),
                ("content", content_str),
                ("recommend", recommend_str),
            ],
            &[],
        ).await?;

        parse_topic_list_response(&xml)
    }
}

/// Builder for favorite topics list.
pub struct FavoriteTopicsBuilder {
    client: Arc<NGAClientInner>,
    folder_id: Option<String>,
    page: u32,
}

impl FavoriteTopicsBuilder {
    /// Set the folder ID.
    pub fn folder(mut self, folder_id: impl Into<String>) -> Self {
        self.folder_id = Some(folder_id.into());
        self
    }

    /// Set the page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<TopicListResult> {
        let page_str = self.page.to_string();
        let folder = self.folder_id.as_deref().unwrap_or("");

        let xml = self.client.post_authed(
            "thread.php",
            &[("favor", folder), ("page", &page_str)],
            &[],
        ).await?;

        parse_topic_list_response(&xml)
    }
}

// ============================================================================
// Parsing helpers
// ============================================================================

fn parse_topic_list_response(xml: &str) -> Result<TopicListResult> {
    let doc = XmlDocument::parse(xml)?;
    let mut topics = Vec::new();

    for node in doc.select("/root/__T/item")? {
        if let Some(topic) = parse_topic(&node)? {
            topics.push(topic);
        }
    }

    let total_pages = parse_pages(&doc, "/root/__ROWS", "/root/__T__ROWS_PAGE", 35)?;

    Ok(TopicListResult {
        topics,
        forum: None,
        subforums: Vec::new(),
        total_pages,
        page: 1,
    })
}

fn parse_topic_details_response(xml: &str, page: u32) -> Result<TopicDetailsResult> {
    let doc = XmlDocument::parse(xml)?;

    let mut users = std::collections::HashMap::new();
    for node in doc.select("/root/__U/item")? {
        if let Some(user) = parse_user(&node)? {
            users.insert(user.id.0.clone(), user);
        }
    }

    let topic = doc.select_one("/root/__T")?
        .and_then(|n| parse_topic(&n).ok().flatten())
        .ok_or_else(|| Error::missing("topic"))?;

    let mut posts = Vec::new();
    for node in doc.select("/root/__R/item")? {
        if let Some(post) = parse_post(&node, &users)? {
            posts.push(post);
        }
    }

    let forum_name = doc.string_opt("/root/__F/name")
        .or_else(|| doc.string_opt("/root/__F"))
        .unwrap_or_default();

    let total_pages = parse_pages(&doc, "/root/__ROWS", "/root/__R__ROWS_PAGE", 20)?;

    Ok(TopicDetailsResult {
        topic,
        posts,
        forum_name,
        total_pages,
        page,
    })
}

fn parse_topic(node: &crate::parser::XmlNode<'_>) -> Result<Option<Topic>> {
    let attrs = node.attrs();

    let id = match attrs.get("quote_from")
        .filter(|s| !s.is_empty() && *s != "0")
        .or_else(|| attrs.get("tid"))
        .cloned()
    {
        Some(id) => id,
        None => return Ok(None),
    };

    let subject_raw = attrs.get("subject").cloned().unwrap_or_default();
    let (tags, content) = parse_subject(&subject_raw);
    let subject = Subject::new(tags, content);

    let author = User {
        id: attrs.get("authorid").cloned().unwrap_or_default().into(),
        name: attrs.get("author")
            .map(|s| UserName::parse(s))
            .unwrap_or_default(),
        ..Default::default()
    };

    let typ: u64 = attrs.get("type")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let is_locked = typ & 0x10 != 0;
    let is_bold = typ & 0x20 != 0;
    let is_assembly = typ & 0x4000 != 0;
    let is_topped = typ & 0x400 != 0;

    let forum_id = attrs.get("fid")
        .filter(|s| !s.is_empty() && *s != "0")
        .map(|s| ForumIdKind::fid(s.clone()));

    let topic = Topic {
        id: id.into(),
        forum_id,
        subject,
        author,
        post_date: attrs.get("postdate").and_then(|s| s.parse().ok()).unwrap_or(0),
        last_post_date: attrs.get("lastpost").and_then(|s| s.parse().ok()).unwrap_or(0),
        replies: attrs.get("replies").and_then(|s| s.parse().ok()).unwrap_or(0),
        last_poster: attrs.get("lastposter").cloned().unwrap_or_default(),
        is_locked,
        is_bold,
        is_assembly,
        is_topped,
        topic_type: attrs.get("type")
            .and_then(|s| s.parse::<i32>().ok())
            .map(TopicType::from)
            .unwrap_or_default(),
        parent_id: None,
        recommend: attrs.get("recommend").and_then(|s| s.parse().ok()).unwrap_or(0),
    };

    Ok(Some(topic))
}

fn parse_user(node: &crate::parser::XmlNode<'_>) -> Result<Option<User>> {
    let attrs = node.attrs();

    let id = match attrs.get("uid") {
        Some(uid) => uid.clone(),
        None => return Ok(None),
    };

    let name = attrs.get("username")
        .map(|s| UserName::parse(s))
        .unwrap_or_default();

    let user = User {
        id: id.into(),
        name,
        avatar_url: attrs.get("avatar").cloned(),
        reputation: attrs.get("fame").and_then(|s| s.parse().ok()).unwrap_or(0),
        posts: attrs.get("postnum").and_then(|s| s.parse().ok()).unwrap_or(0),
        reg_date: attrs.get("regdate").and_then(|s| s.parse().ok()).unwrap_or(0),
        signature: attrs.get("signature").cloned(),
        is_admin: attrs.get("admincheck").map(|s| s != "0").unwrap_or(false),
        is_mod: attrs.get("groupid").map(|s| s == "5" || s == "6").unwrap_or(false),
        is_muted: attrs.get("mute").and_then(|s| s.parse::<i64>().ok()).map(|t| t > 0).unwrap_or(false),
        honor: attrs.get("honor").cloned(),
    };

    Ok(Some(user))
}

fn parse_post(
    node: &crate::parser::XmlNode<'_>,
    users: &std::collections::HashMap<String, User>,
) -> Result<Option<Post>> {
    use crate::parser::parse_content;
    use crate::models::{PostId, VoteState};

    let attrs = node.attrs();

    let id = match attrs.get("pid") {
        Some(pid) => pid.clone(),
        None => return Ok(None),
    };

    let author_id = attrs.get("authorid").cloned().unwrap_or_default();
    let author = users.get(&author_id).cloned().unwrap_or_else(|| User {
        id: author_id.into(),
        ..Default::default()
    });

    let content_raw = attrs.get("content").cloned().unwrap_or_default();
    let content = parse_content(&content_raw);

    let floor = attrs.get("lou").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);

    let post = Post {
        id: PostId::new(id),
        topic_id: attrs.get("tid").cloned().unwrap_or_default().into(),
        floor,
        author,
        content,
        post_date: attrs.get("postdatetimestamp").and_then(|s| s.parse().ok()).unwrap_or(0),
        edit_date: attrs.get("alterinfo").and_then(|s| s.parse().ok()),
        is_edited: attrs.get("alterinfo").is_some(),
        attachments: parse_attachments(node),
        vote: VoteState {
            up: attrs.get("score").and_then(|s| s.parse().ok()).unwrap_or(0),
            down: 0,
            user_vote: None,
        },
        score: attrs.get("score").and_then(|s| s.parse().ok()).unwrap_or(0),
        is_hidden: attrs.get("score")
            .and_then(|s| s.parse::<i32>().ok())
            .map(|s| s < -50)
            .unwrap_or(false),
        from_device: attrs.get("from_client").cloned(),
        signature: attrs.get("signature").cloned(),
        hot_replies: Vec::new(),
        comments: Vec::new(),
        comment_count: attrs.get("comment_count").and_then(|s| s.parse().ok()).unwrap_or(0),
    };

    Ok(Some(post))
}

/// Parse attachments from a post node.
/// Attachments are in `<attachs><item>...</item></attachs>` child elements.
fn parse_attachments(node: &XmlNode<'_>) -> Vec<Attachment> {
    let mut attachments = Vec::new();

    let attachs_node = match node.child_named("attachs") {
        Some(n) => n,
        None => return attachments,
    };

    for item in attachs_node.children_named("item") {
        if let Some(attachment) = parse_attachment(&item) {
            attachments.push(attachment);
        }
    }

    attachments
}

/// Parse a single attachment from an XML node.
fn parse_attachment(node: &XmlNode<'_>) -> Option<Attachment> {
    let attrs = node.attrs();

    let url = attrs.get("attachurl").or_else(|| attrs.get("url"))?;
    if url.is_empty() {
        return None;
    }

    let explicit_type = attrs.get("type").cloned().unwrap_or_default();
    let name = attrs.get("name").cloned().unwrap_or_default();
    let ext = attrs.get("ext").cloned().unwrap_or_else(|| {
        name.rsplit('.')
            .next()
            .or_else(|| url.rsplit('.').next().and_then(|s| s.split('?').next()))
            .unwrap_or("")
            .to_owned()
    });

    let kind = if explicit_type.contains("img") || explicit_type.contains("image") {
        AttachmentKind::Image
    } else {
        AttachmentKind::from_ext(&ext)
    };

    let dimensions = explicit_type
        .split(':')
        .nth(1)
        .and_then(|dim| {
            let parts: Vec<&str> = dim.split('x').collect();
            if parts.len() == 2 {
                let w = parts[0].parse().ok()?;
                let h = parts[1].parse().ok()?;
                Some((w, h))
            } else {
                None
            }
        });

    let thumb_url = attrs.get("thumb").cloned().filter(|s| !s.is_empty());

    Some(Attachment {
        url: url.clone(),
        name: if name.is_empty() {
            url.rsplit('/').next().unwrap_or("attachment").to_owned()
        } else {
            name
        },
        size: attrs
            .get("size")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        kind,
        thumb_url,
        dimensions,
    })
}

fn parse_subforum(node: &crate::parser::XmlNode<'_>) -> Option<Subforum> {
    use crate::client::FORUM_ICON_PATH;

    let is_fid = node.name() == "item";
    let children = node.children();

    let id = children.get(0).map(|n| n.text())?;
    let name = children.get(1).map(|n| n.text()).unwrap_or_default();
    let info = children.get(2).map(|n| n.text()).unwrap_or_default();
    let filter_id = children.get(3).map(|n| n.text()).unwrap_or_default();
    let attributes: u64 = children.get(4)
        .and_then(|n| n.text().parse().ok())
        .unwrap_or(0);

    let forum_id = if is_fid {
        ForumIdKind::fid(&id)
    } else {
        ForumIdKind::stid(&id)
    };

    let icon_url = format!("{}{}.png", FORUM_ICON_PATH, id);

    Some(Subforum {
        forum: Forum {
            id: Some(forum_id),
            name,
            info,
            icon_url,
            topped_topic_id: String::new(),
        },
        filter_id,
        filterable: attributes > 40,
        selected: [7, 558, 542, 2606, 2590, 4654].contains(&attributes),
    })
}

fn parse_forum_from_node(node: &crate::parser::XmlNode<'_>) -> Option<Forum> {
    use crate::client::FORUM_ICON_PATH;

    let attrs = node.attrs();
    let id = attrs.get("fid").or_else(|| attrs.get("stid"))?;
    let is_stid = attrs.get("stid").is_some();

    let forum_id = if is_stid {
        ForumIdKind::stid(id)
    } else {
        ForumIdKind::fid(id)
    };

    let icon_url = format!("{}{}.png", FORUM_ICON_PATH, id);

    Some(Forum {
        id: Some(forum_id),
        name: attrs.get("name").cloned().unwrap_or_default(),
        info: attrs.get("info").cloned().unwrap_or_default(),
        icon_url,
        topped_topic_id: attrs.get("topped_topic").cloned().unwrap_or_default(),
    })
}

fn parse_pages(doc: &XmlDocument, rows_path: &str, page_path: &str, per_page: u32) -> Result<u32> {
    let total_rows = doc.int_or(rows_path, 0) as u32;
    let per_page_actual = doc.int_or(page_path, per_page as i64) as u32;

    if total_rows == 0 {
        return Ok(1);
    }

    let pages = (total_rows + per_page_actual - 1) / per_page_actual;
    Ok(pages.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_order_param() {
        assert_eq!(TopicOrder::LastPost.param(), "");
        assert_eq!(TopicOrder::PostDate.param(), "postdate");
        assert_eq!(TopicOrder::Recommend.param(), "recommend");
    }
}
