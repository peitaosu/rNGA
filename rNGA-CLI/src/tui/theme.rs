use crossterm::style::{available_color_count, Colored};
use ratatui::style::{Color, Modifier, Style};
use ratatui::style::palette::tailwind::{AMBER, RED, SKY, SLATE};

#[derive(Clone, Copy)]
pub struct UiTheme {
    text: Style,
    dim: Style,
    focus: Style,
    border: Style,
    selected: Style,
    section: Style,
    post_header: Style,
    post_header_selected: Style,
    error: Style,
    auto_refresh: Style,
    search_border: Style,
    search_border_editing: Style,
}

impl UiTheme {
    pub fn detect() -> Self {
        if supports_true_color() {
            Self::colorful()
        } else {
            Self::basic()
        }
    }

    pub fn text(self) -> Style {
        self.text
    }

    pub fn dim(self) -> Style {
        self.dim
    }

    pub fn focus(self) -> Style {
        self.focus
    }

    pub fn border(self) -> Style {
        self.border
    }

    pub fn selected(self) -> Style {
        self.selected
    }

    pub fn section(self) -> Style {
        self.section
    }

    pub fn post_header(self) -> Style {
        self.post_header
    }

    pub fn post_header_selected(self) -> Style {
        self.post_header_selected
    }

    pub fn error(self) -> Style {
        self.error
    }

    pub fn auto_refresh(self) -> Style {
        self.auto_refresh
    }

    pub fn search_border(self) -> Style {
        self.search_border
    }

    pub fn search_border_editing(self) -> Style {
        self.search_border_editing
    }
}

fn supports_true_color() -> bool {
    !Colored::ansi_color_disabled() && available_color_count() == u16::MAX
}

fn reset_style() -> Style {
    Style::reset().fg(Color::Reset)
}

impl UiTheme {
    fn basic() -> Self {
        Self {
            text: reset_style(),
            dim: reset_style().add_modifier(Modifier::DIM),
            focus: reset_style().add_modifier(Modifier::BOLD),
            border: reset_style(),
            selected: reset_style().add_modifier(Modifier::REVERSED | Modifier::BOLD),
            section: reset_style().add_modifier(Modifier::BOLD),
            post_header: reset_style(),
            post_header_selected: reset_style().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            error: reset_style().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            auto_refresh: reset_style().add_modifier(Modifier::REVERSED | Modifier::BOLD),
            search_border: reset_style(),
            search_border_editing: reset_style().add_modifier(Modifier::BOLD),
        }
    }

    fn colorful() -> Self {
        Self {
            text: Style::new().fg(SLATE.c300),
            dim: Style::new().fg(SLATE.c500),
            focus: Style::new().fg(SKY.c400).add_modifier(Modifier::BOLD),
            border: Style::new().fg(SLATE.c600),
            selected: Style::new()
                .fg(SLATE.c100)
                .bg(SLATE.c700)
                .add_modifier(Modifier::BOLD),
            section: Style::new().fg(AMBER.c400).add_modifier(Modifier::BOLD),
            post_header: Style::new().fg(SKY.c400),
            post_header_selected: Style::new()
                .fg(AMBER.c300)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            error: Style::new().fg(RED.c400).add_modifier(Modifier::BOLD),
            auto_refresh: Style::new().fg(SLATE.c950).bg(SKY.c400),
            search_border: Style::new().fg(SKY.c400),
            search_border_editing: Style::new().fg(AMBER.c400).add_modifier(Modifier::BOLD),
        }
    }
}
