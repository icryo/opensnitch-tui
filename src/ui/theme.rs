//! Color theme definitions

use ratatui::style::{Color, Modifier, Style};

/// Application color theme
#[derive(Debug, Clone)]
pub struct Theme {
    // Base colors
    pub bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub fg_bright: Color,

    // Accent colors
    pub accent: Color,
    pub accent_dim: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Action colors
    pub allow: Color,
    pub deny: Color,
    pub reject: Color,

    // UI elements
    pub border: Color,
    pub border_focused: Color,
    pub selection: Color,
    pub highlight: Color,

    // Tab colors
    pub tab_active: Color,
    pub tab_inactive: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Base colors
            bg: Color::Reset,
            fg: Color::White,
            fg_dim: Color::DarkGray,
            fg_bright: Color::White,

            // Accent colors
            accent: Color::Cyan,
            accent_dim: Color::DarkGray,

            // Status colors
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,

            // Action colors
            allow: Color::Green,
            deny: Color::Red,
            reject: Color::Magenta,

            // UI elements
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            selection: Color::Blue,
            highlight: Color::Yellow,

            // Tab colors
            tab_active: Color::Cyan,
            tab_inactive: Color::DarkGray,
        }
    }
}

impl Theme {
    /// Dark theme variant
    pub fn dark() -> Self {
        Self::default()
    }

    /// Light theme variant
    pub fn light() -> Self {
        Self {
            bg: Color::White,
            fg: Color::Black,
            fg_dim: Color::DarkGray,
            fg_bright: Color::Black,
            accent: Color::Blue,
            accent_dim: Color::Gray,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
            allow: Color::Green,
            deny: Color::Red,
            reject: Color::Magenta,
            border: Color::Gray,
            border_focused: Color::Blue,
            selection: Color::LightBlue,
            highlight: Color::Yellow,
            tab_active: Color::Blue,
            tab_inactive: Color::Gray,
        }
    }

    // Style helpers
    pub fn normal(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn dim(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    pub fn bright(&self) -> Style {
        Style::default().fg(self.fg_bright)
    }

    pub fn accent(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn info(&self) -> Style {
        Style::default().fg(self.info)
    }

    pub fn selected(&self) -> Style {
        Style::default().bg(self.selection).fg(self.fg_bright)
    }

    pub fn highlight(&self) -> Style {
        Style::default().fg(self.highlight).add_modifier(Modifier::BOLD)
    }

    pub fn border(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.border_focused)
    }

    pub fn tab_active(&self) -> Style {
        Style::default().fg(self.tab_active).add_modifier(Modifier::BOLD)
    }

    pub fn tab_inactive(&self) -> Style {
        Style::default().fg(self.tab_inactive)
    }

    pub fn action_style(&self, action: &str) -> Style {
        match action.to_lowercase().as_str() {
            "allow" | "accept" => Style::default().fg(self.allow),
            "deny" | "drop" => Style::default().fg(self.deny),
            "reject" => Style::default().fg(self.reject),
            _ => self.normal(),
        }
    }
}
