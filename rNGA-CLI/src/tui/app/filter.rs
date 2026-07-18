use crate::handlers::topic::{PostInfo, TopicSummary};
use crate::tui::search::FilterQuery;
use super::state::{ForumRow, ThreadLayout};

pub(crate) fn forum_row_matches(row: &ForumRow, query: &FilterQuery<'_>) -> bool {
    match row {
        ForumRow::Header(name) => query.matches(name),
        ForumRow::Favorite(info) | ForumRow::Forum(info) => {
            query.matches_any([&info.name.as_str(), info.info.as_str()])
        }
    }
}

pub(crate) fn topic_matches(topic: &TopicSummary, query: &FilterQuery<'_>) -> bool {
    let tags = topic.tags.join(" ");
    query.matches_any([
        topic.subject.as_str(),
        topic.author.as_str(),
        tags.as_str(),
        topic.id.as_str(),
    ])
}

pub(crate) fn post_matches(post: &PostInfo, query: &FilterQuery<'_>) -> bool {
    let floor = post.floor.to_string();
    query.matches_any([
        post.author.as_str(),
        post.author_id.as_str(),
        post.content.as_str(),
        post.content_raw.as_str(),
        floor.as_str(),
        post.post_id.as_str(),
    ])
}

pub fn visible_forum_indices(rows: &[ForumRow], filter: &str) -> Vec<usize> {
    let Some(query) = FilterQuery::prepare(filter) else {
        return rows
            .iter()
            .enumerate()
            .filter(|(_, row)| !matches!(row, ForumRow::Header(_)))
            .map(|(index, _)| index)
            .collect();
    };

    let mut visible = Vec::new();
    let mut pending_header: Option<usize> = None;
    for (index, row) in rows.iter().enumerate() {
        match row {
            ForumRow::Header(_) => pending_header = Some(index),
            ForumRow::Favorite(_) | ForumRow::Forum(_) if forum_row_matches(row, &query) => {
                if let Some(header_index) = pending_header.take() {
                    visible.push(header_index);
                }
                visible.push(index);
            }
            ForumRow::Favorite(_) | ForumRow::Forum(_) => pending_header = None,
        }
    }
    visible
}

pub(crate) fn first_visible_forum(rows: &[ForumRow], filter: &str) -> usize {
    visible_forum_indices(rows, filter)
        .into_iter()
        .find(|&index| !matches!(rows.get(index), Some(ForumRow::Header(_))))
        .unwrap_or(0)
}

pub(crate) fn restore_forum_index(rows: &[ForumRow], forum_id: Option<&str>) -> usize {
    let Some(forum_id) = forum_id else {
        return first_visible_forum(rows, "");
    };
    rows.iter().enumerate().find_map(|(index, row)| {
        let id = match row {
            ForumRow::Favorite(info) | ForumRow::Forum(info) => {
                info.stid.as_deref().or(info.fid.as_deref())
            }
            ForumRow::Header(_) => None,
        };
        id.filter(|value| *value == forum_id).map(|_| index)
    })
    .unwrap_or_else(|| first_visible_forum(rows, ""))
}

pub(crate) fn restore_topic_index(topics: &[TopicSummary], topic_id: Option<&str>) -> usize {
    let Some(topic_id) = topic_id else {
        return 0;
    };
    topics
        .iter()
        .position(|topic| topic.id == topic_id)
        .unwrap_or(0)
}

pub(crate) fn restore_post_index(posts: &[PostInfo], post_id: Option<&str>) -> usize {
    let Some(post_id) = post_id else {
        return 0;
    };
    posts
        .iter()
        .position(|post| post.post_id == post_id)
        .unwrap_or(0)
}

pub fn thread_layout_for(posts: &[PostInfo]) -> ThreadLayout {
    let mut post_starts = Vec::new();
    let mut line_count = 0usize;
    for post in posts {
        post_starts.push(line_count);
        line_count += 1;
        line_count += post.content.lines().count().max(1);
        line_count += 1;
    }
    ThreadLayout {
        post_starts,
        line_count,
    }
}
