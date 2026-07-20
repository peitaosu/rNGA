use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use rust_i18n::t;

use crate::handlers::topic::{AttachmentInfo, PostInfo};
use crate::output::format_relative_time;

use super::app::ThreadLayout;
use super::theme::UiTheme;

pub enum ThreadBlockKind {
    PostHeader(Line<'static>),
    TextLine(Line<'static>),
    AttachmentLabel(String),
    Blank,
}

pub struct ThreadBlock {
    pub height: u16,
    pub kind: ThreadBlockKind,
}

pub fn post_header_line(post: &PostInfo, selected: bool, theme: UiTheme) -> Line<'static> {
    let score = if post.score != 0 {
        format!(" ▲{}", post.score)
    } else {
        String::new()
    };
    let comments = if post.comment_count > 0 {
        format!(" 💬{}", post.comment_count)
    } else {
        String::new()
    };
    let text = format!(
        "#{} {} · {} · {}{}{}",
        post.floor,
        post.author,
        format_relative_time(post.post_date),
        post.author_id,
        score,
        comments
    );
    let style = if selected {
        theme.post_header_selected()
    } else {
        theme.post_header()
    };
    Line::from(Span::styled(text, style))
}

pub fn build_thread_blocks(
    posts: &[PostInfo],
    selected_index: usize,
    width: usize,
    theme: UiTheme,
) -> Vec<ThreadBlock> {
    let mut blocks = Vec::new();
    for (post_index, post) in posts.iter().enumerate() {
        blocks.push(ThreadBlock {
            height: 1,
            kind: ThreadBlockKind::PostHeader(post_header_line(
                post,
                post_index == selected_index,
                theme,
            )),
        });
        for line in post.content.lines() {
            if line.trim().is_empty() {
                blocks.push(ThreadBlock {
                    height: 1,
                    kind: ThreadBlockKind::TextLine(Line::from("")),
                });
            } else {
                for part in wrap_line(line, width, 4) {
                    blocks.push(ThreadBlock {
                        height: 1,
                        kind: ThreadBlockKind::TextLine(part),
                    });
                }
            }
        }
        for attachment in &post.attachments {
            blocks.push(ThreadBlock {
                height: 1,
                kind: ThreadBlockKind::AttachmentLabel(attachment_label(attachment)),
            });
        }
        blocks.push(ThreadBlock {
            height: 1,
            kind: ThreadBlockKind::Blank,
        });
    }
    blocks
}

fn attachment_label(attachment: &AttachmentInfo) -> String {
    let size = format_attachment_size(attachment.size);
    if size.is_empty() {
        t!("tui_attachment_file", name = attachment.name).into_owned()
    } else {
        t!(
            "tui_attachment_file_size",
            name = attachment.name,
            size = size
        )
        .into_owned()
    }
}

fn format_attachment_size(size: i64) -> String {
    if size <= 0 {
        return String::new();
    }
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.0} KB", size as f64 / KB as f64)
    } else {
        format!("{size} B")
    }
}

pub fn layout_from_blocks(blocks: &[ThreadBlock]) -> ThreadLayout {
    let mut post_starts = Vec::new();
    let mut line_count = 0usize;
    for block in blocks {
        if matches!(block.kind, ThreadBlockKind::PostHeader(_)) {
            post_starts.push(line_count);
        }
        line_count += block.height as usize;
    }
    ThreadLayout {
        post_starts,
        line_count,
    }
}

pub fn draw_thread_blocks(
    frame: &mut Frame,
    area: Rect,
    scroll: u16,
    blocks: &[ThreadBlock],
    theme: UiTheme,
) {
    let mut y = area.y as i32 - scroll as i32;
    for block in blocks {
        let height = block.height as i32;
        let bottom = y + height;
        if bottom <= area.y as i32 {
            y += height;
            continue;
        }
        if y >= (area.y + area.height) as i32 {
            break;
        }

        match &block.kind {
            ThreadBlockKind::PostHeader(line) | ThreadBlockKind::TextLine(line) => {
                if y >= area.y as i32 && y < (area.y + area.height) as i32 {
                    let rect = Rect {
                        x: area.x,
                        y: y as u16,
                        width: area.width,
                        height: 1,
                    };
                    frame.render_widget(Paragraph::new(line.clone()), rect);
                }
            }
            ThreadBlockKind::AttachmentLabel(label) => {
                if y >= area.y as i32 && y < (area.y + area.height) as i32 {
                    frame.render_widget(
                        Paragraph::new(label.as_str()).style(theme.dim()),
                        Rect {
                            x: area.x,
                            y: y as u16,
                            width: area.width,
                            height: 1,
                        },
                    );
                }
            }
            ThreadBlockKind::Blank => {}
        }
        y += height;
    }
}

fn wrap_line(text: &str, width: usize, indent: usize) -> Vec<Line<'static>> {
    if width <= indent {
        return vec![Line::from(text.to_owned())];
    }
    let usable = width - indent;
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + char_width > usable && !current.is_empty() {
            lines.push(Line::from(format!("{:indent$}{current}", "", indent = indent)));
            current.clear();
            used = 0;
        }
        current.push(ch);
        used += char_width;
    }
    if !current.is_empty() {
        lines.push(Line::from(format!("{:indent$}{current}", "", indent = indent)));
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::topic::PostInfo;
    use rnga::models::AttachmentKind;

    #[test]
    fn layout_includes_attachment_labels() {
        let posts = vec![PostInfo {
            floor: 1,
            post_id: "1".into(),
            topic_id: "100".into(),
            author: "user".into(),
            author_id: "42".into(),
            content: "hello".into(),
            content_raw: String::new(),
            content_parse_error: None,
            score: 0,
            post_date: 0,
            comment_count: 0,
            attachments: vec![AttachmentInfo {
                url: "https://img.nga.178.com/attachments/mon/a.webp".into(),
                name: "a.webp".into(),
                size: 1024,
                kind: AttachmentKind::Image,
                thumb_url: None,
                dimensions: None,
            }],
        }];
        let blocks = build_thread_blocks(&posts, 0, 40, UiTheme::detect());
        assert!(blocks.iter().any(|block| {
            matches!(block.kind, ThreadBlockKind::AttachmentLabel(_))
        }));
        let layout = layout_from_blocks(&blocks);
        assert_eq!(layout.post_starts, vec![0]);
        assert!(layout.line_count >= 4);
    }
}
