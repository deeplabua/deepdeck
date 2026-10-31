//! Styles from the terminal's 16-colour palette, so the user's theme wins.
//! With `NO_COLOR` only modifiers (bold, reverse) remain.

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub color: bool,
}

impl Theme {
    pub fn from_env() -> Self {
        Theme {
            color: std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()),
        }
    }

    fn fg(&self, c: Color) -> Style {
        if self.color {
            Style::new().fg(c)
        } else {
            Style::new()
        }
    }

    /// Yellow is reserved for "waiting for your decision".
    pub fn waiting(&self) -> Style {
        self.fg(Color::Yellow)
    }
    /// The agent's blocking message: black on yellow (reverse without colour).
    pub fn waiting_banner(&self) -> Style {
        if self.color {
            Style::new().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::new().add_modifier(Modifier::REVERSED)
        }
    }
    pub fn working(&self) -> Style {
        self.fg(Color::Magenta)
    }
    pub fn done(&self) -> Style {
        self.fg(Color::Green)
    }
    pub fn dim(&self) -> Style {
        self.fg(Color::DarkGray)
    }
    pub fn question(&self) -> Style {
        self.fg(Color::Cyan)
    }
    pub fn idea(&self) -> Style {
        self.fg(Color::LightRed)
    }
    pub fn name(&self) -> Style {
        Style::new().add_modifier(Modifier::BOLD)
    }
    pub fn accent(&self) -> Style {
        self.fg(Color::Magenta).add_modifier(Modifier::BOLD)
    }
    pub fn tab_active(&self) -> Style {
        Style::new().add_modifier(Modifier::BOLD | Modifier::REVERSED)
    }
}
