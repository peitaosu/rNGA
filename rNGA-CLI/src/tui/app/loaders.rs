use crate::handlers::forum;
use crate::handlers::options::{ListTopicsOptions, ReadTopicOptions};
use crate::handlers::topic;
use rust_i18n::t;
use rnga::NGAClient;

use super::state::{ForumLoadOutcome, ForumRow, TaskResult};
use super::App;

pub(crate) async fn load_forum_rows(
    client: &NGAClient,
    authenticated: bool,
) -> Result<ForumLoadOutcome, String> {
    let categories = forum::list_categories(client)
        .await
        .map_err(|error| error.to_string())?;
    let (favorites, favorites_error) = if authenticated {
        match forum::list_favorites(client).await {
            Ok(result) => (result.forums, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        }
    } else {
        (Vec::new(), None)
    };

    let mut rows = Vec::new();
    if !favorites.is_empty() {
        rows.push(ForumRow::Header(t!("tui_favorites").into_owned()));
        for fav in &favorites {
            rows.push(ForumRow::Favorite(fav.clone()));
        }
    }
    for category in &categories.categories {
        rows.push(ForumRow::Header(category.name.clone()));
        for forum in &category.forums {
            rows.push(ForumRow::Forum(forum.clone()));
        }
    }
    Ok(ForumLoadOutcome {
        rows,
        favorites,
        favorites_error,
    })
}

impl App {
    pub fn load_forums(&mut self) {
        self.forums_generation += 1;
        self.forums_fetching = true;
        let generation = self.forums_generation;
        let client = self.client.clone();
        let authenticated = self.auth.authenticated;
        let tx = self.task_tx.clone();
        tokio::spawn(async move {
            let result = load_forum_rows(&client, authenticated).await;
            let _ = tx.send(TaskResult::Forums(generation, result));
        });
    }

    pub(crate) fn load_topics(&mut self, forum_id: &str, is_stid: bool, page: u32) {
        self.topics_preserve_selection = page == self.topic_page;
        self.topic_page = page;
        if !self.topics_preserve_selection {
            self.topic_index = 0;
        }
        self.topics_generation += 1;
        self.topics_fetching = true;
        let generation = self.topics_generation;
        let client = self.client.clone();
        let forum_id = forum_id.to_string();
        let tx = self.task_tx.clone();
        tokio::spawn(async move {
            let result = topic::list_topics(
                &client,
                &forum_id,
                ListTopicsOptions {
                    is_stid,
                    start_page: page,
                    num_pages: 1,
                    order: "lastpost".into(),
                    concurrency: 1,
                },
            )
            .await
            .map_err(|error| error.to_string());
            let _ = tx.send(TaskResult::Topics(generation, result));
        });
    }

    pub(crate) fn load_thread(&mut self, topic_id: &str, page: u32) {
        let page_changed = self
            .thread
            .as_ref()
            .is_none_or(|thread| thread.topic_id != topic_id || thread.page != page);
        if page_changed {
            self.thread_post_index = 0;
            self.thread_scroll = 0;
        }
        self.thread_generation += 1;
        self.thread_fetching = true;
        let generation = self.thread_generation;
        let client = self.client.clone();
        let topic_id = topic_id.to_string();
        let tx = self.task_tx.clone();
        tokio::spawn(async move {
            let result = topic::read_topic(
                &client,
                &topic_id,
                ReadTopicOptions {
                    page,
                    ..Default::default()
                },
            )
            .await
            .map_err(|error| error.to_string());
            let _ = tx.send(TaskResult::Thread(generation, result));
        });
    }
}
