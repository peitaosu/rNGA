use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use rust_i18n::t;

use crate::handlers::topic::{CliTopicDetailsResult, PostInfo, TopicSummary};
use crate::output::format_relative_time;

use super::app::{App, ForumRow, InputMode, Pane, SEARCH_FIELD_HEIGHT, FORUMS_WIDTH, TOPICS_WIDTH};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    let cols = Layout::horizontal([
        Constraint::Length(FORUMS_WIDTH),
        Constraint::Length(TOPICS_WIDTH),
        Constraint::Min(10),
    ])
    .split(rows[0]);

    draw_forums(frame, cols[0], app);
    draw_topics(frame, cols[1], app);
    draw_thread(frame, cols[2], app);
    draw_status(frame, rows[1], app);
}

fn pane_border(title: &str, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(format!(" {title} "))
}

fn draw_forums(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Pane::Forums;
    let title = t!("tui_pane_forums");
    let block = pane_border(&title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = split_pane(inner, app.search_visible(Pane::Forums));
    let list_area = rows.body;
    if let Some(search_area) = rows.search {
        draw_search_field(frame, search_area, app, Pane::Forums);
    }

    if app.forum_rows.is_empty() && app.forums_fetching {
        frame.render_widget(
            Paragraph::new(t!("tui_loading")).style(Style::default().fg(Color::DarkGray)),
            list_area,
        );
        return;
    }

    if app.forum_rows.is_empty() {
        frame.render_widget(
            Paragraph::new(t!("tui_no_forums")).style(Style::default().fg(Color::DarkGray)),
            list_area,
        );
        return;
    }

    let filtering = app.filter_active(Pane::Forums);
    let items: Vec<ListItem> = if filtering {
        app.visible_forum_indices()
            .iter()
            .map(|&index| forum_row_item(&app.forum_rows[index], index == app.forum_index))
            .collect()
    } else {
        app.forum_rows
            .iter()
            .enumerate()
            .map(|(index, row)| forum_row_item(row, index == app.forum_index))
            .collect()
    };

    let selected = if filtering {
        app.visible_forum_indices()
            .iter()
            .position(|&index| index == app.forum_index)
            .unwrap_or(0)
    } else {
        app.forum_index
    };
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_stateful_widget(list, list_area, &mut state);
}

fn forum_row_item(row: &ForumRow, selected: bool) -> ListItem<'static> {
    match row {
        ForumRow::Header(name) => ListItem::new(Line::from(Span::styled(
            name.clone(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))),
        ForumRow::Favorite(info) => forum_item(&info.name, true, selected),
        ForumRow::Forum(info) => forum_item(&info.name, false, selected),
    }
}

fn forum_item(name: &str, favorite: bool, selected: bool) -> ListItem<'static> {
    let prefix = if favorite { "★ " } else { "  " };
    let style = if selected {
        Style::default()
    } else {
        Style::default().fg(Color::Gray)
    };
    ListItem::new(Line::from(Span::styled(format!("{prefix}{name}"), style)))
}

fn draw_topics(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Pane::Topics;
    let default_title = t!("tui_pane_topics");
    let title = app
        .selected_forum
        .as_ref()
        .map(|forum| forum.name.as_str())
        .unwrap_or(&default_title);
    let block = pane_border(title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = split_pane(inner, app.search_visible(Pane::Topics));
    let list_area = rows.body;
    if let Some(search_area) = rows.search {
        draw_search_field(frame, search_area, app, Pane::Topics);
    }

    if app.selected_forum.is_none() {
        frame.render_widget(
            Paragraph::new(t!("tui_select_forum")).style(Style::default().fg(Color::DarkGray)),
            list_area,
        );
        return;
    }

    if app.topics.is_empty() && app.topics_fetching {
        frame.render_widget(
            Paragraph::new(t!("tui_loading")).style(Style::default().fg(Color::DarkGray)),
            list_area,
        );
        return;
    }

    let filtering = app.filter_active(Pane::Topics);
    let visible = if filtering {
        app.visible_topic_indices()
    } else {
        (0..app.topics.len()).collect()
    };

    if visible.is_empty() {
        let message = if filtering {
            t!("tui_no_match")
        } else {
            t!("tui_no_topics")
        };
        frame.render_widget(
            Paragraph::new(message).style(Style::default().fg(Color::DarkGray)),
            list_area,
        );
        return;
    }

    let list_width = list_area.width.saturating_sub(2) as usize;
    let items: Vec<ListItem> = visible
        .iter()
        .map(|&index| {
            let topic = &app.topics[index];
            topic_item(topic, index == app.topic_index, list_width)
        })
        .collect();

    let selected = visible
        .iter()
        .position(|&index| index == app.topic_index)
        .unwrap_or(0);
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_stateful_widget(list, list_area, &mut state);
}

fn topic_item(topic: &TopicSummary, selected: bool, width: usize) -> ListItem<'static> {
    let tags = if topic.tags.is_empty() {
        String::new()
    } else {
        format!("[{}] ", topic.tags.join("]["))
    };
    let subject = format!("{tags}{}", topic.subject);
    let meta = t!(
        "tui_topic_list_meta",
        author = topic.author,
        time = format_relative_time(topic.last_post_date),
        replies = topic.replies
    )
    .into_owned();
    let subject_style = if selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let meta_style = Style::default().fg(Color::DarkGray);
    let mut lines = styled_wrap(&subject, width, subject_style);
    lines.extend(styled_wrap(&meta, width, meta_style));
    ListItem::new(lines)
}

fn draw_thread(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Pane::Thread;
    let title = t!("tui_pane_thread");
    let block = pane_border(&title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = split_pane(inner, app.search_visible(Pane::Thread));
    let content = rows.body;
    if let Some(search_area) = rows.search {
        draw_search_field(frame, search_area, app, Pane::Thread);
    }

    if app.thread.is_none() && app.thread_fetching {
        frame.render_widget(
            Paragraph::new(t!("tui_loading")).style(Style::default().fg(Color::DarkGray)),
            content,
        );
        return;
    }

    let Some(thread) = &app.thread else {
        frame.render_widget(
            Paragraph::new(t!("tui_select_topic")).style(Style::default().fg(Color::DarkGray)),
            content,
        );
        return;
    };

    let header_h = 3u16.min(content.height);
    let body = Layout::vertical([
        Constraint::Length(header_h),
        Constraint::Min(1),
    ])
    .split(content);

    draw_thread_header(frame, body[0], thread);
    draw_thread_body(frame, body[1], app, thread);
}

fn draw_thread_header(frame: &mut Frame, area: Rect, thread: &CliTopicDetailsResult) {
    let tags = if thread.tags.is_empty() {
        String::new()
    } else {
        format!("[{}] ", thread.tags.join("]["))
    };
    let title = format!("{tags}{}", thread.subject);
    let meta = t!(
        "tui_thread_meta",
        forum = thread.forum_name,
        replies = thread.replies,
        page = thread.page,
        total = thread.total_pages
    );
    let paragraph = Paragraph::new(vec![
        Line::from(Span::styled(title, Style::default().add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(meta, Style::default().fg(Color::DarkGray))),
    ]);
    frame.render_widget(paragraph, area);
}

fn draw_thread_body(frame: &mut Frame, area: Rect, app: &App, thread: &CliTopicDetailsResult) {
    let posts = app.filtered_thread_posts(thread);
    if posts.is_empty() {
        let message = if app.filter_active(Pane::Thread) {
            t!("tui_no_match")
        } else {
            t!("tui_no_posts")
        };
        frame.render_widget(
            Paragraph::new(message).style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    }

    let width = area.width.saturating_sub(2) as usize;
    let mut rendered: Vec<Line> = Vec::new();
    for (post_index, post) in posts.iter().enumerate() {
        let selected = post_index == app.thread_post_index;
        rendered.push(post_header_line(post, selected));
        for line in post.content.lines() {
            if line.trim().is_empty() {
                rendered.push(Line::from(""));
            } else {
                for part in wrap_line(line, width, 4) {
                    rendered.push(part);
                }
            }
        }
        rendered.push(Line::from(""));
    }

    let paragraph = Paragraph::new(rendered)
        .wrap(Wrap { trim: false })
        .scroll((app.thread_scroll, 0));
    frame.render_widget(paragraph, area);
}

fn post_header_line(post: &PostInfo, selected: bool) -> Line<'static> {
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
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    Line::from(Span::styled(text, style))
}

fn styled_wrap(text: &str, width: usize, style: Style) -> Vec<Line<'static>> {
    wrap_line(text, width.max(1), 0)
        .into_iter()
        .map(|line| {
            let content: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            Line::from(Span::styled(content, style))
        })
        .collect()
}

fn wrap_line(text: &str, width: usize, indent: usize) -> Vec<Line<'static>> {
    if width <= indent {
        return vec![Line::from(text.to_string())];
    }
    let usable = width - indent;
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + w > usable && !current.is_empty() {
            lines.push(Line::from(format!("{:indent$}{current}", "", indent = indent)));
            current.clear();
            used = 0;
        }
        current.push(ch);
        used += w;
    }
    if !current.is_empty() {
        lines.push(Line::from(format!("{:indent$}{current}", "", indent = indent)));
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let auth = if app.auth.authenticated {
        app.auth
            .uid
            .as_deref()
            .map(|uid| uid.to_string())
            .unwrap_or_else(|| t!("tui_logged_in").into_owned())
    } else {
        t!("tui_guest").into_owned()
    };
    let pane = match app.focus {
        Pane::Forums => t!("tui_focus_forums"),
        Pane::Topics => t!("tui_focus_topics"),
        Pane::Thread => t!("tui_focus_thread"),
    };
    let context_style = Style::default().fg(Color::Gray);
    let hint_style = Style::default().fg(Color::DarkGray);
    let mut meta_spans = vec![Span::raw(format!(" {auth} │ {pane}"))];
    for part in app.status_context() {
        meta_spans.push(Span::raw(" │ "));
        meta_spans.push(Span::styled(part, context_style));
    }
    if let Some(error) = &app.status {
        meta_spans.push(Span::raw(" │ "));
        meta_spans.push(Span::styled(error.clone(), Style::default().fg(Color::Red)));
    }

    let mut hint_spans = vec![
        Span::styled(t!("tui_status_hints").into_owned(), hint_style),
        Span::styled(" | ", hint_style),
    ];
    let auto_style = if app.auto_refresh {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        hint_style
    };
    hint_spans.push(Span::styled(
        t!("tui_status_auto_refresh").into_owned(),
        auto_style,
    ));

    let meta_width = spans_width(&meta_spans);
    let hint_width = spans_width(&hint_spans);
    let pad = area.width as usize - meta_width - hint_width;
    let mut spans = meta_spans;
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans.extend(hint_spans);
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn spans_width(spans: &[Span<'_>]) -> usize {
    spans
        .iter()
        .map(|span| unicode_width::UnicodeWidthStr::width(span.content.as_ref()))
        .sum()
}

struct PaneAreas {
    body: Rect,
    search: Option<Rect>,
}

fn split_pane(area: Rect, show_search: bool) -> PaneAreas {
    if !show_search {
        return PaneAreas {
            body: area,
            search: None,
        };
    }
    let rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(SEARCH_FIELD_HEIGHT),
    ])
    .split(area);
    PaneAreas {
        body: rows[0],
        search: Some(rows[1]),
    }
}

fn draw_search_field(frame: &mut Frame, area: Rect, app: &App, pane: Pane) {
    let editing = matches!(app.input_mode, InputMode::Search(active) if active == pane);
    let border_style = if editing {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let filter_title = format!(" {} ", t!("tui_filter"));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(filter_title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let display = format!("{}{}", app.search_input, "▌");
    frame.render_widget(
        Paragraph::new(display).style(Style::default().fg(Color::White)),
        inner,
    );
}
