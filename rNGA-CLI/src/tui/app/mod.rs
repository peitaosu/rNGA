mod filter;
mod loaders;
mod state;

use std::time::Instant;

use crate::config::{auth_status, build_client, AuthStatus};
use crate::handlers::forum::ForumInfo;
use crate::handlers::topic::{CliTopicDetailsResult, PostInfo, TopicSummary};
use crate::tui::thread_view;
use crate::tui::theme::UiTheme;
use rust_i18n::t;
use rnga::NGAClient;
use tokio::sync::mpsc;

pub use filter::visible_forum_indices;
pub use state::*;

use filter::{post_matches, restore_forum_index, restore_post_index, restore_topic_index, topic_matches};

pub struct App {
    pub task_tx: mpsc::UnboundedSender<TaskResult>,
    pub client: NGAClient,
    pub auth: AuthStatus,
    pub focus: Pane,
    pub quit: bool,
    pub status: Option<String>,

    pub input_mode: InputMode,
    pub search_input: String,
    pub search_open: Option<Pane>,

    pub auto_refresh: bool,
    last_auto_refresh: Instant,
    refresh_cascade: RefreshCascade,

    pub forum_rows: Vec<ForumRow>,
    pub forum_index: usize,
    pub favorite_forums: Vec<ForumInfo>,
    pub forums_fetching: bool,
    forums_generation: u64,

    pub selected_forum: Option<SelectedForum>,
    pub topics: Vec<TopicSummary>,
    pub topic_index: usize,
    pub topic_page: u32,
    pub topic_total_pages: u32,
    pub topics_fetching: bool,
    topics_generation: u64,
    topics_preserve_selection: bool,

    pub thread: Option<CliTopicDetailsResult>,
    pub thread_layout: ThreadLayout,
    pub thread_post_index: usize,
    pub thread_scroll: u16,
    pub thread_fetching: bool,
    pub thread_body_width: u16,
    thread_generation: u64,
    pending_thread_focus: bool,
}

impl App {
    pub fn new(task_tx: mpsc::UnboundedSender<TaskResult>) -> Self {
        let client = build_client().expect("client");
        let auth = auth_status();
        Self {
            task_tx,
            client,
            auth,
            focus: Pane::Forums,
            quit: false,
            status: None,
            input_mode: InputMode::Normal,
            search_input: String::new(),
            search_open: None,
            auto_refresh: false,
            last_auto_refresh: Instant::now(),
            refresh_cascade: RefreshCascade::None,
            forum_rows: Vec::new(),
            forum_index: 0,
            favorite_forums: Vec::new(),
            forums_fetching: true,
            forums_generation: 0,
            selected_forum: None,
            topics: Vec::new(),
            topic_index: 0,
            topic_page: 1,
            topic_total_pages: 1,
            topics_fetching: false,
            topics_generation: 0,
            topics_preserve_selection: false,
            thread: None,
            thread_layout: ThreadLayout::default(),
            thread_post_index: 0,
            thread_scroll: 0,
            thread_fetching: false,
            thread_body_width: 0,
            thread_generation: 0,
            pending_thread_focus: false,
        }
    }

    pub fn on_task(&mut self, message: TaskResult) {
        match message {
            TaskResult::Forums(generation, result) => {
                if generation != self.forums_generation {
                    return;
                }
                self.forums_fetching = false;
                match result {
                    Ok(outcome) => {
                        let prev_id = self.selected_forum_id_from_index();
                        self.favorite_forums = outcome.favorites;
                        self.forum_rows = outcome.rows;
                        self.forum_index = restore_forum_index(&self.forum_rows, prev_id.as_deref());
                        self.status = outcome.favorites_error;
                        self.continue_cascade_after_forums();
                    }
                    Err(error) => {
                        self.refresh_cascade = RefreshCascade::None;
                        self.status = Some(error);
                    }
                }
            }
            TaskResult::Topics(generation, result) => {
                if generation != self.topics_generation {
                    return;
                }
                self.topics_fetching = false;
                match result {
                    Ok(list) => {
                        let prev_id = if self.topics_preserve_selection {
                            self.topics.get(self.topic_index).map(|topic| topic.id.clone())
                        } else {
                            None
                        };
                        if let Some(name) = list.forum_name {
                            if let Some(forum) = self.selected_forum.as_mut() {
                                forum.name = name;
                            }
                        }
                        self.topic_page = list.start_page;
                        self.topic_total_pages = list.total_pages.max(1);
                        self.topics = list.topics;
                        self.topic_index = if self.topics_preserve_selection {
                            restore_topic_index(&self.topics, prev_id.as_deref())
                        } else {
                            0
                        };
                        self.topics_preserve_selection = false;
                        self.status = None;
                        self.continue_cascade_after_topics();
                    }
                    Err(error) => {
                        self.refresh_cascade = RefreshCascade::None;
                        self.status = Some(error);
                    }
                }
            }
            TaskResult::Thread(generation, result) => {
                if generation != self.thread_generation {
                    return;
                }
                self.thread_fetching = false;
                self.refresh_cascade = RefreshCascade::None;
                match result {
                    Ok(details) => {
                        let same_page_refresh = self.thread.as_ref().is_some_and(|thread| {
                            thread.topic_id == details.topic_id && thread.page == details.page
                        });
                        let prev_post_id = if same_page_refresh {
                            self.current_post_id()
                        } else {
                            None
                        };
                        self.thread = Some(details);
                        if let Some(thread) = self.thread.as_ref() {
                            self.thread_post_index =
                                restore_post_index(&thread.posts, prev_post_id.as_deref());
                        }
                        self.rebuild_thread_layout();
                        self.scroll_to_post();
                        self.status = None;
                        if self.pending_thread_focus {
                            self.focus = Pane::Thread;
                            self.pending_thread_focus = false;
                        }
                    }
                    Err(error) => {
                        self.status = Some(error);
                        if self.thread.is_none() {
                            self.focus = Pane::Topics;
                            self.pending_thread_focus = false;
                        }
                    }
                }
            }
        }
    }

    pub fn toggle_auto_refresh(&mut self) {
        self.auto_refresh = !self.auto_refresh;
        self.last_auto_refresh = Instant::now();
    }

    pub fn auto_refresh_tick(&mut self) {
        if !self.auto_refresh || self.input_mode != InputMode::Normal || self.search_open.is_some() {
            return;
        }
        if self.last_auto_refresh.elapsed() < AUTO_REFRESH_INTERVAL {
            return;
        }
        self.last_auto_refresh = Instant::now();
        self.refresh_cascade = match self.focus {
            Pane::Forums => RefreshCascade::FromForums,
            Pane::Topics => RefreshCascade::FromTopics,
            Pane::Thread => RefreshCascade::None,
        };
        self.refresh_focused();
    }

    pub fn filter_active(&self, pane: Pane) -> bool {
        self.search_open == Some(pane)
            && matches!(self.input_mode, InputMode::Search(active) if active == pane)
    }

    fn can_navigate(&self) -> bool {
        self.input_mode == InputMode::Normal || self.filter_active(self.focus)
    }

    pub fn active_filter(&self, pane: Pane) -> &str {
        if self.filter_active(pane) {
            &self.search_input
        } else {
            ""
        }
    }

    pub fn start_search(&mut self) {
        self.search_open = Some(self.focus);
        self.search_input.clear();
        self.input_mode = InputMode::Search(self.focus);
    }

    pub fn end_search(&mut self) {
        let pane = match self.input_mode {
            InputMode::Search(pane) => pane,
            InputMode::Normal => {
                self.search_open = None;
                self.search_input.clear();
                return;
            }
        };
        let post_id = if pane == Pane::Thread {
            self.current_post_id()
        } else {
            None
        };
        self.input_mode = InputMode::Normal;
        self.search_input.clear();
        self.search_open = None;
        if let (Pane::Thread, Some(post_id)) = (pane, post_id) {
            if let Some(thread) = &self.thread {
                self.thread_post_index = restore_post_index(&thread.posts, Some(&post_id));
                self.rebuild_thread_layout();
                self.scroll_to_post();
            }
        }
    }

    pub fn push_search_char(&mut self, ch: char) {
        self.search_input.push(ch);
        self.snap_filter_selection();
    }

    pub fn pop_search_char(&mut self) {
        self.search_input.pop();
        self.snap_filter_selection();
    }

    fn snap_filter_selection(&mut self) {
        if let InputMode::Search(pane) = self.input_mode {
            self.snap_filter_selection_for(pane);
        }
    }

    fn snap_filter_selection_for(&mut self, pane: Pane) {
        match pane {
            Pane::Forums => {
                let visible = self.visible_forum_indices();
                if !visible.contains(&self.forum_index) {
                    self.forum_index = visible.first().copied().unwrap_or(0);
                }
            }
            Pane::Topics => {
                let visible = self.visible_topic_indices();
                if !visible.contains(&self.topic_index) {
                    self.topic_index = visible.first().copied().unwrap_or(0);
                }
            }
            Pane::Thread => {
                let visible = self
                    .thread
                    .as_ref()
                    .map(|thread| self.filtered_thread_posts(thread).len())
                    .unwrap_or(0);
                if visible == 0 {
                    self.thread_post_index = 0;
                    self.thread_scroll = 0;
                    self.rebuild_thread_layout();
                    return;
                }
                if self.thread_post_index >= visible {
                    self.thread_post_index = visible - 1;
                }
                self.rebuild_thread_layout();
                self.scroll_to_post();
            }
        }
    }

    pub fn search_visible(&self, pane: Pane) -> bool {
        self.search_open == Some(pane)
    }

    pub fn visible_forum_indices(&self) -> Vec<usize> {
        visible_forum_indices(&self.forum_rows, self.active_filter(Pane::Forums))
    }

    pub fn visible_topic_indices(&self) -> Vec<usize> {
        let Some(query) = super::search::FilterQuery::prepare(self.active_filter(Pane::Topics)) else {
            return (0..self.topics.len()).collect();
        };
        self.topics
            .iter()
            .enumerate()
            .filter(|(_, topic)| topic_matches(topic, &query))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn filtered_thread_posts<'a>(&'a self, thread: &'a CliTopicDetailsResult) -> Vec<&'a PostInfo> {
        let Some(query) = super::search::FilterQuery::prepare(self.active_filter(Pane::Thread)) else {
            return thread.posts.iter().collect();
        };
        thread
            .posts
            .iter()
            .filter(|post| post_matches(post, &query))
            .collect()
    }

    pub fn status_context(&self) -> Vec<String> {
        let mut parts = Vec::new();
        if self.filter_active(self.focus) {
            let (visible, total) = match self.focus {
                Pane::Forums => {
                    let visible = self
                        .visible_forum_indices()
                        .into_iter()
                        .filter(|index| {
                            !matches!(self.forum_rows.get(*index), Some(ForumRow::Header(_)))
                        })
                        .count();
                    let total = self
                        .forum_rows
                        .iter()
                        .filter(|row| !matches!(row, ForumRow::Header(_)))
                        .count();
                    (visible, total)
                }
                Pane::Topics => {
                    let visible = self.visible_topic_indices().len();
                    (visible, self.topics.len())
                }
                Pane::Thread => {
                    if let Some(thread) = &self.thread {
                        let visible = self.filtered_thread_posts(thread).len();
                        (visible, thread.posts.len())
                    } else {
                        (0, 0)
                    }
                }
            };
            if visible != total || !self.search_input.is_empty() {
                parts.push(
                    t!("tui_status_filter", visible = visible, total = total).into_owned(),
                );
            }
        }
        match self.focus {
            Pane::Forums => {
                if self.auth.authenticated {
                    parts.push(
                        t!("tui_status_favorites", count = self.favorite_forums.len()).into_owned(),
                    );
                }
                let selectable: Vec<usize> = self
                    .visible_forum_indices()
                    .into_iter()
                    .filter(|index| {
                        !matches!(self.forum_rows.get(*index), Some(ForumRow::Header(_)))
                    })
                    .collect();
                if !selectable.is_empty() {
                    let current = selectable
                        .iter()
                        .position(|index| *index == self.forum_index)
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    parts.push(
                        t!("tui_status_list_pos", current = current, total = selectable.len())
                            .into_owned(),
                    );
                }
                if self.forums_fetching {
                    parts.push(t!("tui_loading").into_owned());
                }
            }
            Pane::Topics => {
                if let Some(forum) = &self.selected_forum {
                    parts.push(forum.name.clone());
                }
                if self.selected_forum.is_some() {
                    parts.push(
                        t!(
                            "tui_status_page",
                            page = self.topic_page,
                            total = self.topic_total_pages
                        )
                        .into_owned(),
                    );
                }
                let visible = self.visible_topic_indices();
                if !visible.is_empty() {
                    let current = visible
                        .iter()
                        .position(|index| *index == self.topic_index)
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    parts.push(
                        t!("tui_status_list_pos", current = current, total = visible.len())
                            .into_owned(),
                    );
                }
                if self.topics_fetching || (self.pending_thread_focus && self.thread_fetching) {
                    parts.push(t!("tui_loading").into_owned());
                }
            }
            Pane::Thread => {
                if let Some(thread) = &self.thread {
                    parts.push(
                        t!(
                            "tui_status_page",
                            page = thread.page,
                            total = thread.total_pages
                        )
                        .into_owned(),
                    );
                    let posts = self.filtered_thread_posts(thread);
                    if let Some(post) = posts.get(self.thread_post_index) {
                        parts.push(t!("tui_status_floor", floor = post.floor).into_owned());
                    }
                    if !posts.is_empty() {
                        parts.push(
                            t!(
                                "tui_status_list_pos",
                                current = self.thread_post_index + 1,
                                total = posts.len()
                            )
                            .into_owned(),
                        );
                    }
                }
                if self.thread_fetching {
                    parts.push(t!("tui_loading").into_owned());
                }
            }
        }
        parts
    }

    pub fn next_pane(&mut self) {
        self.clear_pending_thread_focus_if_leaving_topics();
        self.focus = match self.focus {
            Pane::Forums => Pane::Topics,
            Pane::Topics => Pane::Thread,
            Pane::Thread => Pane::Forums,
        };
    }

    pub fn prev_pane(&mut self) {
        self.clear_pending_thread_focus_if_leaving_topics();
        self.focus = match self.focus {
            Pane::Forums => Pane::Thread,
            Pane::Topics => Pane::Forums,
            Pane::Thread => Pane::Topics,
        };
    }

    pub fn move_down(&mut self) {
        if !self.can_navigate() {
            return;
        }
        match self.focus {
            Pane::Forums => {
                let visible = self.visible_forum_indices();
                if let Some(pos) = visible.iter().position(|&index| index == self.forum_index) {
                    if pos + 1 < visible.len() {
                        self.forum_index = visible[pos + 1];
                    }
                }
            }
            Pane::Topics => {
                let visible = self.visible_topic_indices();
                if let Some(pos) = visible.iter().position(|&index| index == self.topic_index) {
                    if pos + 1 < visible.len() {
                        self.topic_index = visible[pos + 1];
                    }
                }
            }
            Pane::Thread => {
                if let Some(thread) = &self.thread {
                    let posts = self.filtered_thread_posts(thread);
                    if self.thread_post_index + 1 < posts.len() {
                        self.thread_post_index += 1;
                        self.scroll_to_post();
                    }
                }
            }
        }
    }

    pub fn move_up(&mut self) {
        if !self.can_navigate() {
            return;
        }
        match self.focus {
            Pane::Forums => {
                let visible = self.visible_forum_indices();
                if let Some(pos) = visible.iter().position(|&index| index == self.forum_index) {
                    if pos > 0 {
                        self.forum_index = visible[pos - 1];
                    }
                }
            }
            Pane::Topics => {
                let visible = self.visible_topic_indices();
                if let Some(pos) = visible.iter().position(|&index| index == self.topic_index) {
                    if pos > 0 {
                        self.topic_index = visible[pos - 1];
                    }
                }
            }
            Pane::Thread => {
                if let Some(thread) = &self.thread {
                    let posts = self.filtered_thread_posts(thread);
                    if self.thread_post_index > 0 && !posts.is_empty() {
                        self.thread_post_index -= 1;
                        self.scroll_to_post();
                    }
                }
            }
        }
    }

    pub fn move_first(&mut self) {
        if !self.can_navigate() {
            return;
        }
        match self.focus {
            Pane::Forums => {
                self.forum_index = self
                    .visible_forum_indices()
                    .into_iter()
                    .find(|&index| !matches!(self.forum_rows.get(index), Some(ForumRow::Header(_))))
                    .unwrap_or(0);
            }
            Pane::Topics => {
                self.topic_index = self
                    .visible_topic_indices()
                    .first()
                    .copied()
                    .unwrap_or(0);
            }
            Pane::Thread => {
                self.thread_post_index = 0;
                self.scroll_to_post();
            }
        }
    }

    pub fn move_last(&mut self) {
        if !self.can_navigate() {
            return;
        }
        match self.focus {
            Pane::Forums => {
                let visible = self.visible_forum_indices();
                if let Some(&last) = visible.last() {
                    self.forum_index = last;
                }
            }
            Pane::Topics => {
                if let Some(&last) = self.visible_topic_indices().last() {
                    self.topic_index = last;
                }
            }
            Pane::Thread => {
                if let Some(thread) = &self.thread {
                    let posts = self.filtered_thread_posts(thread);
                    if !posts.is_empty() {
                        self.thread_post_index = posts.len() - 1;
                        self.scroll_to_post();
                    }
                }
            }
        }
    }

    pub fn activate(&mut self) {
        match self.focus {
            Pane::Forums => {
                if let Some(row) = self.selectable_forum_row() {
                    self.select_forum_row(row);
                }
            }
            Pane::Topics => {
                if let Some(topic_id) = self.selected_topic_id() {
                    self.request_topic(&topic_id);
                } else {
                    self.clear_thread();
                }
            }
            Pane::Thread => {}
        }
    }

    pub fn refresh(&mut self) {
        self.refresh_cascade = RefreshCascade::None;
        self.refresh_focused();
    }

    fn refresh_focused(&mut self) {
        match self.focus {
            Pane::Forums => self.load_forums(),
            Pane::Topics => self.refresh_topics(),
            Pane::Thread => self.refresh_thread(),
        }
    }

    fn refresh_topics(&mut self) {
        let Some(forum) = self.selected_forum.clone() else {
            return;
        };
        self.load_topics(&forum.id, forum.is_stid, self.topic_page);
    }

    fn refresh_thread(&mut self) {
        if let Some((topic_id, page)) = self
            .thread
            .as_ref()
            .map(|thread| (thread.topic_id.clone(), thread.page))
        {
            self.load_thread(&topic_id, page);
        }
    }

    fn continue_cascade_after_forums(&mut self) {
        if self.refresh_cascade != RefreshCascade::FromForums {
            return;
        }
        if self.selected_forum.is_some() {
            self.refresh_cascade = if self.thread.is_some() {
                RefreshCascade::FromTopics
            } else {
                RefreshCascade::None
            };
            self.refresh_topics();
        } else {
            self.refresh_cascade = RefreshCascade::None;
        }
    }

    fn continue_cascade_after_topics(&mut self) {
        if !matches!(
            self.refresh_cascade,
            RefreshCascade::FromForums | RefreshCascade::FromTopics
        ) {
            return;
        }
        self.refresh_cascade = RefreshCascade::None;
        self.refresh_thread();
    }

    fn selected_forum_id_from_index(&self) -> Option<String> {
        self.forum_rows.get(self.forum_index).and_then(|row| match row {
            ForumRow::Favorite(info) | ForumRow::Forum(info) => {
                info.stid.clone().or(info.fid.clone())
            }
            ForumRow::Header(_) => None,
        })
    }

    pub fn next_page(&mut self) {
        match self.focus {
            Pane::Topics => {
                if self.selected_forum.is_none() || self.topics_fetching {
                    return;
                }
                if self.topic_page < self.topic_total_pages {
                    if let Some(forum) = self.selected_forum.clone() {
                        let page = self.topic_page + 1;
                        self.load_topics(&forum.id, forum.is_stid, page);
                    }
                }
            }
            Pane::Thread => {
                if self.thread_fetching {
                    return;
                }
                let Some((topic_id, page)) = self.thread.as_ref().map(|thread| {
                    (thread.topic_id.clone(), thread.page)
                }) else {
                    return;
                };
                if page < self.thread.as_ref().map_or(0, |thread| thread.total_pages) {
                    self.load_thread(&topic_id, page + 1);
                }
            }
            Pane::Forums => {}
        }
    }

    pub fn prev_page(&mut self) {
        match self.focus {
            Pane::Topics => {
                if self.selected_forum.is_none() || self.topics_fetching {
                    return;
                }
                if self.topic_page > 1 {
                    if let Some(forum) = self.selected_forum.clone() {
                        let page = self.topic_page - 1;
                        self.load_topics(&forum.id, forum.is_stid, page);
                    }
                }
            }
            Pane::Thread => {
                if self.thread_fetching {
                    return;
                }
                let Some((topic_id, page)) = self.thread.as_ref().map(|thread| {
                    (thread.topic_id.clone(), thread.page)
                }) else {
                    return;
                };
                if page > 1 {
                    self.load_thread(&topic_id, page - 1);
                }
            }
            Pane::Forums => {}
        }
    }

    pub fn scroll_thread(&mut self, delta: i16) {
        let max = self.thread_layout.line_count.saturating_sub(1) as u16;
        if delta.is_negative() {
            self.thread_scroll = self.thread_scroll.saturating_sub(delta.unsigned_abs());
        } else {
            self.thread_scroll = (self.thread_scroll + delta as u16).min(max);
        }
    }

    pub fn open_forum(&mut self, forum_id: &str, is_stid: bool) {
        let changing = self
            .selected_forum
            .as_ref()
            .is_none_or(|forum| forum.id != forum_id);
        self.selected_forum = Some(SelectedForum {
            id: forum_id.to_string(),
            is_stid,
            name: forum_id.to_string(),
        });
        self.focus = Pane::Topics;
        if changing {
            self.topics.clear();
            self.topic_page = 1;
            self.topic_index = 0;
            self.clear_thread();
        }
        self.load_topics(forum_id, is_stid, 1);
    }

    pub fn open_topic(&mut self, topic_id: &str) {
        self.request_topic(topic_id);
    }

    pub fn request_topic(&mut self, topic_id: &str) {
        self.prepare_thread_navigation(topic_id);
        self.pending_thread_focus = true;
        self.load_thread(topic_id, 1);
    }

    fn clear_pending_thread_focus_if_leaving_topics(&mut self) {
        if self.focus == Pane::Topics {
            self.pending_thread_focus = false;
        }
    }

    fn selectable_forum_row(&self) -> Option<ForumRow> {
        if let Some(row) = self.forum_rows.get(self.forum_index) {
            if !matches!(row, ForumRow::Header(_)) {
                return Some(row.clone());
            }
        }
        self.visible_forum_indices().iter().find_map(|&index| {
            self.forum_rows.get(index).and_then(|row| match row {
                ForumRow::Header(_) => None,
                _ => Some(row.clone()),
            })
        })
    }

    fn selected_topic_id(&self) -> Option<String> {
        self.topics
            .get(self.topic_index)
            .map(|topic| topic.id.clone())
    }

    fn prepare_thread_navigation(&mut self, topic_id: &str) {
        if self
            .thread
            .as_ref()
            .is_none_or(|thread| thread.topic_id != topic_id)
        {
            self.clear_thread();
        }
    }

    fn clear_thread(&mut self) {
        self.thread = None;
        self.thread_layout = ThreadLayout::default();
        self.thread_post_index = 0;
        self.thread_scroll = 0;
        self.pending_thread_focus = false;
    }

    fn select_forum_row(&mut self, row: ForumRow) {
        let (id, is_stid, name) = match row {
            ForumRow::Favorite(info) | ForumRow::Forum(info) => {
                let is_stid = info.stid.is_some();
                let id = info
                    .stid
                    .or(info.fid)
                    .unwrap_or_default();
                (id, is_stid, info.name)
            }
            ForumRow::Header(_) => return,
        };
        let changing = self
            .selected_forum
            .as_ref()
            .is_none_or(|forum| forum.id != id);
        self.selected_forum = Some(SelectedForum {
            id: id.clone(),
            is_stid,
            name,
        });
        self.focus = Pane::Topics;
        if changing {
            self.topics.clear();
            self.topic_page = 1;
            self.topic_index = 0;
            self.clear_thread();
        }
        self.load_topics(&id, is_stid, 1);
    }

    fn current_post_id(&self) -> Option<String> {
        self.thread.as_ref().and_then(|thread| {
            if self.filter_active(Pane::Thread) {
                self.filtered_thread_posts(thread)
                    .get(self.thread_post_index)
                    .map(|post| post.post_id.clone())
            } else {
                thread
                    .posts
                    .get(self.thread_post_index)
                    .map(|post| post.post_id.clone())
            }
        })
    }

    fn scroll_to_post(&mut self) {
        let posts = self.displayed_thread_posts();
        if posts.is_empty() {
            return;
        }
        if let Some(start) = self.thread_layout.post_starts.get(self.thread_post_index) {
            self.thread_scroll = *start as u16;
        }
    }

    pub fn displayed_thread_posts(&self) -> Vec<PostInfo> {
        let Some(thread) = &self.thread else {
            return Vec::new();
        };
        if self.filter_active(Pane::Thread) {
            self.filtered_thread_posts(thread)
                .into_iter()
                .cloned()
                .collect()
        } else {
            thread.posts.clone()
        }
    }

    pub fn rebuild_thread_layout(&mut self) {
        let width = self.thread_body_width.max(40) as usize;
        let content_width = width.saturating_sub(2);
        let posts = self.displayed_thread_posts();
        let blocks = thread_view::build_thread_blocks(
            &posts,
            self.thread_post_index,
            content_width,
            UiTheme::detect(),
        );
        self.thread_layout = thread_view::layout_from_blocks(&blocks);
    }

    pub fn update_thread_body_width(&mut self, width: u16) {
        let width = width.max(1);
        if width == self.thread_body_width {
            return;
        }
        self.thread_body_width = width;
        self.rebuild_thread_layout();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::topic::{CliTopicDetailsResult, TopicSummary};
    use tokio::sync::mpsc;

    fn sample_app(focus: Pane) -> App {
        let (task_tx, _task_rx) = mpsc::unbounded_channel();
        let mut app = App::new(task_tx);
        app.focus = focus;
        app.selected_forum = Some(SelectedForum {
            id: "7".into(),
            is_stid: false,
            name: "Test".into(),
        });
        app.topic_page = 2;
        app.topic_total_pages = 5;
        app.topics = vec![TopicSummary {
            id: "100".into(),
            subject: "One".into(),
            tags: vec![],
            author: "a".into(),
            author_id: "1".into(),
            replies: 0,
            post_date: 0,
            last_post_date: 0,
        }];
        app.topic_index = 0;
        app.thread = Some(CliTopicDetailsResult {
            topic_id: "100".into(),
            forum_name: "Test".into(),
            subject: "Topic".into(),
            tags: vec![],
            author: "a".into(),
            author_id: "1".into(),
            replies: 0,
            post_date: 0,
            page: 2,
            total_pages: 4,
            posts: vec![],
            meta: Default::default(),
        });
        app
    }

    #[tokio::test]
    async fn next_page_advances_topics_within_bounds() {
        let mut app = sample_app(Pane::Topics);
        app.topics_fetching = false;
        app.next_page();
        assert_eq!(app.topic_page, 3);
        assert_eq!(app.topic_index, 0);
        assert!(!app.topics_preserve_selection);
        assert!(app.topics_fetching);
    }

    #[test]
    fn next_page_does_not_exceed_topic_total_pages() {
        let mut app = sample_app(Pane::Topics);
        app.topic_page = 5;
        app.topics_fetching = false;
        app.next_page();
        assert_eq!(app.topic_page, 5);
        assert!(!app.topics_fetching);
    }

    #[test]
    fn prev_page_does_not_go_below_one() {
        let mut app = sample_app(Pane::Topics);
        app.topic_page = 1;
        app.prev_page();
        assert_eq!(app.topic_page, 1);
        assert!(!app.topics_fetching);
    }

    #[tokio::test]
    async fn thread_next_page_resets_post_selection() {
        let mut app = sample_app(Pane::Thread);
        app.thread_post_index = 3;
        app.thread_scroll = 9;
        app.thread_fetching = false;
        app.next_page();
        assert_eq!(app.thread_post_index, 0);
        assert_eq!(app.thread_scroll, 0);
        assert!(app.thread_fetching);
    }

    #[tokio::test]
    async fn refresh_same_topic_page_preserves_selection_flag() {
        let mut app = sample_app(Pane::Topics);
        app.load_topics("7", false, 2);
        assert!(app.topics_preserve_selection);
        assert_eq!(app.topic_page, 2);
    }

    #[test]
    fn activate_topics_with_empty_list_keeps_focus_on_topics() {
        let mut app = sample_app(Pane::Topics);
        app.topics.clear();
        app.activate();
        assert_eq!(app.focus, Pane::Topics);
        assert!(app.thread.is_none());
    }

    #[tokio::test]
    async fn request_topic_keeps_pending_focus_after_clear() {
        let mut app = sample_app(Pane::Topics);
        app.thread = Some(CliTopicDetailsResult {
            topic_id: "100".into(),
            forum_name: "Test".into(),
            subject: "Old".into(),
            tags: vec![],
            author: "a".into(),
            author_id: "1".into(),
            replies: 0,
            post_date: 0,
            page: 1,
            total_pages: 1,
            posts: vec![],
            meta: Default::default(),
        });
        app.request_topic("200");
        assert!(app.pending_thread_focus);
        assert!(app.thread.is_none());
    }

    #[test]
    fn thread_fetch_failure_keeps_focus_on_topics() {
        let mut app = sample_app(Pane::Topics);
        app.thread = None;
        app.pending_thread_focus = true;
        app.thread_fetching = true;
        app.on_task(TaskResult::Thread(
            app.thread_generation,
            Err("fetching topic details".into()),
        ));
        assert_eq!(app.focus, Pane::Topics);
        assert!(!app.pending_thread_focus);
        assert!(app.thread.is_none());
    }

    #[test]
    fn thread_fetch_success_moves_focus_when_pending() {
        let mut app = sample_app(Pane::Topics);
        app.pending_thread_focus = true;
        app.on_task(TaskResult::Thread(
            app.thread_generation,
            Ok(CliTopicDetailsResult {
                topic_id: "100".into(),
                forum_name: "Test".into(),
                subject: "Topic".into(),
                tags: vec![],
                author: "a".into(),
                author_id: "1".into(),
                replies: 0,
                post_date: 0,
                page: 1,
                total_pages: 1,
                posts: vec![],
                meta: Default::default(),
            }),
        ));
        assert_eq!(app.focus, Pane::Thread);
        assert!(!app.pending_thread_focus);
        assert!(app.thread.is_some());
    }

    #[tokio::test]
    async fn activate_forums_on_header_selects_first_forum() {
        let (task_tx, _task_rx) = mpsc::unbounded_channel();
        let mut app = App::new(task_tx);
        app.forum_rows = vec![
            ForumRow::Header("Section".into()),
            ForumRow::Forum(crate::handlers::forum::ForumInfo {
                fid: Some("7".into()),
                stid: None,
                name: "Test Forum".into(),
                info: String::new(),
            }),
        ];
        app.forum_index = 0;
        app.activate();
        assert_eq!(app.focus, Pane::Topics);
        assert_eq!(
            app.selected_forum.as_ref().map(|forum| forum.id.as_str()),
            Some("7")
        );
    }
}
