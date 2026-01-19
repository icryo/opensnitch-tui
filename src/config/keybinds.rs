//! Keyboard shortcut definitions

use crossterm::event::{KeyCode, KeyModifiers};

/// Keyboard shortcut configuration
#[derive(Debug, Clone)]
pub struct KeyBindings {
    // Global
    pub quit: KeyBind,
    pub help: KeyBind,
    pub refresh: KeyBind,

    // Tab navigation
    pub next_tab: KeyBind,
    pub prev_tab: KeyBind,

    // List navigation (arrow keys primary, vi alternative)
    pub up: KeyBind,
    pub down: KeyBind,
    pub page_up: KeyBind,
    pub page_down: KeyBind,
    pub top: KeyBind,
    pub bottom: KeyBind,

    // Actions
    pub select: KeyBind,
    pub delete: KeyBind,
    pub edit: KeyBind,
    pub new_item: KeyBind,
    pub filter: KeyBind,
    pub clear_filter: KeyBind,
    pub copy: KeyBind,

    // Prompt dialog
    pub allow: KeyBind,
    pub deny: KeyBind,
    pub reject: KeyBind,
    pub toggle_advanced: KeyBind,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            // Global
            quit: KeyBind::new(KeyCode::Char('q'), KeyModifiers::NONE),
            help: KeyBind::new(KeyCode::Char('?'), KeyModifiers::NONE),
            refresh: KeyBind::new(KeyCode::Char('r'), KeyModifiers::NONE),

            // Tab navigation
            next_tab: KeyBind::new(KeyCode::Tab, KeyModifiers::NONE),
            prev_tab: KeyBind::new(KeyCode::BackTab, KeyModifiers::SHIFT),

            // List navigation (arrow keys primary)
            up: KeyBind::new(KeyCode::Up, KeyModifiers::NONE),
            down: KeyBind::new(KeyCode::Down, KeyModifiers::NONE),
            page_up: KeyBind::new(KeyCode::PageUp, KeyModifiers::NONE),
            page_down: KeyBind::new(KeyCode::PageDown, KeyModifiers::NONE),
            top: KeyBind::new(KeyCode::Home, KeyModifiers::NONE),
            bottom: KeyBind::new(KeyCode::End, KeyModifiers::NONE),

            // Actions
            select: KeyBind::new(KeyCode::Enter, KeyModifiers::NONE),
            delete: KeyBind::new(KeyCode::Delete, KeyModifiers::NONE),
            edit: KeyBind::new(KeyCode::Char('e'), KeyModifiers::NONE),
            new_item: KeyBind::new(KeyCode::Char('n'), KeyModifiers::NONE),
            filter: KeyBind::new(KeyCode::Char('/'), KeyModifiers::NONE),
            clear_filter: KeyBind::new(KeyCode::Esc, KeyModifiers::NONE),
            copy: KeyBind::new(KeyCode::Char('y'), KeyModifiers::NONE),

            // Prompt dialog
            allow: KeyBind::new(KeyCode::Char('a'), KeyModifiers::NONE),
            deny: KeyBind::new(KeyCode::Char('d'), KeyModifiers::NONE),
            reject: KeyBind::new(KeyCode::Char('r'), KeyModifiers::NONE),
            toggle_advanced: KeyBind::new(KeyCode::Tab, KeyModifiers::NONE),
        }
    }
}

/// A single key binding
#[derive(Debug, Clone)]
pub struct KeyBind {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBind {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn matches(&self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.code == code && self.modifiers == modifiers
    }
}
