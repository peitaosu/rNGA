use crate::{
    error::{Error, Result},
    models::{
        Attachment, AttachmentKind, Forum, ForumIdKind, Post, Subject, Topic, TopicType, User,
        UserName,
    },
    parser::{parse_content, parse_subject, parse_user_from_node, XmlDocument, XmlNode},
};

use super::{TopicDetailsResult, TopicListResult};

pub const SUBFORUM_SELECTED_ATTRIBUTES: [u64; 6] = [7, 558, 542, 2606, 2590, 4654];

#[derive(Debug, Clone, Default)]
pub struct Subforum {
    pub forum: Forum,
    pub filter_id: String,
    pub filterable: bool,
    pub selected: bool,
}

pub(crate) fn parse_topic_list_response(xml: &str, page: u32) -> Result<TopicListResult> {
    let doc = XmlDocument::parse(xml)?;
    let mut topics = Vec::new();

    for node in doc.select("/root/__T/item")? {
        if let Some(topic) = parse_topic(&node)? {
            topics.push(topic);
        }
    }

    let total_pages = parse_pages(&doc, "/root/__ROWS", "/root/__T__ROWS_PAGE", 35)?;

    let mut subforums = Vec::new();
    for node in doc.select("/root/__F/sub_forums/*")? {
        if let Some(subforum) = parse_subforum(&node) {
            subforums.push(subforum);
        }
    }

    let forum = doc
        .select_one("/root/__F")?
        .as_ref()
        .and_then(|node| parse_forum_from_node(node));

    Ok(TopicListResult {
        topics,
        forum,
        subforums,
        total_pages,
        page,
    })
}

pub(crate) fn parse_topic_details_response(xml: &str, page: u32) -> Result<TopicDetailsResult> {
    let doc = XmlDocument::parse(xml)?;

    let mut users = std::collections::HashMap::new();
    for node in doc.select("/root/__U/item")? {
        if let Some(user) = parse_user(&node)? {
            users.insert(user.id.0.clone(), user);
        }
    }

    let topic = doc
        .select_one("/root/__T")?
        .and_then(|n| parse_topic(&n).ok().flatten())
        .ok_or_else(|| Error::missing("topic"))?;

    let mut posts = Vec::new();
    for node in doc.select("/root/__R/item")? {
        if let Some(post) = parse_post(&node, &users)? {
            posts.push(post);
        }
    }

    let forum_name = doc
        .string_opt("/root/__F/name")
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

fn parse_topic(node: &XmlNode<'_>) -> Result<Option<Topic>> {
    let attrs = node.attrs();

    let id = match attrs
        .get("quote_from")
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
        name: attrs
            .get("author")
            .map(|s| UserName::parse(s))
            .unwrap_or_default(),
        ..Default::default()
    };

    let typ: u64 = attrs.get("type").and_then(|s| s.parse().ok()).unwrap_or(0);

    let is_locked = typ & 0x10 != 0;
    let is_bold = typ & 0x20 != 0;
    let is_assembly = typ & 0x4000 != 0;
    let is_topped = typ & 0x400 != 0;

    let forum_id = attrs
        .get("fid")
        .filter(|s| !s.is_empty() && *s != "0")
        .map(|s| ForumIdKind::fid(s.clone()));

    let topic = Topic {
        id: id.into(),
        forum_id,
        subject,
        author,
        post_date: attrs
            .get("postdate")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        last_post_date: attrs
            .get("lastpost")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        replies: attrs
            .get("replies")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        last_poster: attrs.get("lastposter").cloned().unwrap_or_default(),
        is_locked,
        is_bold,
        is_assembly,
        is_topped,
        topic_type: attrs
            .get("type")
            .and_then(|s| s.parse::<i32>().ok())
            .map(TopicType::from)
            .unwrap_or_default(),
        parent_id: None,
        recommend: attrs
            .get("recommend")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    };

    Ok(Some(topic))
}

fn parse_user(node: &XmlNode<'_>) -> Result<Option<User>> {
    parse_user_from_node(&node.attrs())
}

fn parse_post(
    node: &XmlNode<'_>,
    users: &std::collections::HashMap<String, User>,
) -> Result<Option<Post>> {
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

    let floor = attrs
        .get("lou")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);

    let post = Post {
        id: PostId::new(id),
        topic_id: attrs.get("tid").cloned().unwrap_or_default().into(),
        floor,
        author,
        content,
        post_date: attrs
            .get("postdatetimestamp")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        edit_date: attrs.get("alterinfo").and_then(|s| s.parse().ok()),
        is_edited: attrs.get("alterinfo").is_some(),
        attachments: parse_attachments(node),
        vote: VoteState {
            up: attrs.get("score").and_then(|s| s.parse().ok()).unwrap_or(0),
            down: 0,
            user_vote: None,
        },
        score: attrs.get("score").and_then(|s| s.parse().ok()).unwrap_or(0),
        is_hidden: attrs
            .get("score")
            .and_then(|s| s.parse::<i32>().ok())
            .map(|s| s < -50)
            .unwrap_or(false),
        from_device: attrs.get("from_client").cloned(),
        signature: attrs.get("signature").cloned(),
        hot_replies: Vec::new(),
        comments: Vec::new(),
        comment_count: attrs
            .get("comment_count")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    };

    Ok(Some(post))
}

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

    let dimensions = explicit_type.split(':').nth(1).and_then(|dim| {
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
        size: attrs.get("size").and_then(|s| s.parse().ok()).unwrap_or(0),
        kind,
        thumb_url,
        dimensions,
    })
}

pub(crate) fn parse_subforum(node: &XmlNode<'_>) -> Option<Subforum> {
    let is_fid = node.name() == "item";
    let children = node.children();

    let id = children.get(0).map(|n| n.text())?;
    let name = children.get(1).map(|n| n.text()).unwrap_or_default();
    let info = children.get(2).map(|n| n.text()).unwrap_or_default();
    let filter_id = children.get(3).map(|n| n.text()).unwrap_or_default();
    let attributes: u64 = children
        .get(4)
        .and_then(|n| n.text().parse().ok())
        .unwrap_or(0);

    let forum_id = if is_fid {
        ForumIdKind::fid(&id)
    } else {
        ForumIdKind::stid(&id)
    };

    let icon_url = Forum::icon_url_for(&id);

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
        selected: SUBFORUM_SELECTED_ATTRIBUTES.contains(&attributes),
    })
}

pub(crate) fn parse_forum_from_node(node: &XmlNode<'_>) -> Option<Forum> {
    let attrs = node.attrs();
    let id = attrs.get("fid").or_else(|| attrs.get("stid"))?;
    let is_stid = attrs.get("stid").is_some();

    let forum_id = if is_stid {
        ForumIdKind::stid(id)
    } else {
        ForumIdKind::fid(id)
    };

    let icon_url = Forum::icon_url_for(&id);

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
    Ok(crate::parser::compute_total_pages(total_rows, per_page_actual))
}
