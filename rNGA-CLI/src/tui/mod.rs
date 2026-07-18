mod app;
mod search;
mod ui;

use std::io::stdout;
use std::time::Duration;

use anyhow::Result;
use app::{App, InputMode};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

pub async fn run(
    start_forum: Option<String>,
    start_stid: bool,
    start_topic: Option<String>,
) -> Result<()> {
    let (task_tx, mut task_rx) = mpsc::unbounded_channel();
    let mut app = App::new(task_tx);
    app.load_forums();
    if let Some(forum_id) = start_forum {
        app.open_forum(&forum_id, start_stid);
    }
    if let Some(topic_id) = start_topic {
        app.open_topic(&topic_id);
    }

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        while let Ok(message) = task_rx.try_recv() {
            app.on_task(message);
        }

        if app.quit {
            break Ok(());
        }

        app.auto_refresh_tick();

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(&mut app, key) {
                    break Ok(());
                }
            }
        }
    };

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    result
}

fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    if app.input_mode != InputMode::Normal {
        return handle_search_key(app, key);
    }

    match key.code {
        KeyCode::Char('q') if key.modifiers.is_empty() => {
            app.quit = true;
            return true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.quit = true;
            return true;
        }
        KeyCode::Char('/') => app.start_search(),
        KeyCode::Char('a') if key.modifiers.is_empty() => app.toggle_auto_refresh(),
        KeyCode::Left => app.prev_pane(),
        KeyCode::Right => app.next_pane(),
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('g') => app.move_first(),
        KeyCode::Char('G') => app.move_last(),
        KeyCode::Enter => app.activate(),
        KeyCode::Char('r') => app.refresh(),
        KeyCode::Char('n') | KeyCode::Char(']') => app.next_page(),
        KeyCode::Char('p') | KeyCode::Char('[') => app.prev_page(),
        KeyCode::PageDown => app.scroll_thread(3),
        KeyCode::PageUp => app.scroll_thread(-3),
        KeyCode::Char('J') => app.scroll_thread(1),
        KeyCode::Char('K') => app.scroll_thread(-1),
        _ => {}
    }
    false
}

fn handle_search_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => app.end_search(),
        KeyCode::Backspace => app.pop_search_char(),
        KeyCode::Down => app.move_down(),
        KeyCode::Up => app.move_up(),
        KeyCode::Char('n') | KeyCode::Char(']') if key.modifiers.is_empty() => app.next_page(),
        KeyCode::Char('p') | KeyCode::Char('[') if key.modifiers.is_empty() => app.prev_page(),
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => app.push_search_char(ch),
        _ => {}
    }
    false
}
