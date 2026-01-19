//! Input event handling

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Application input events
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
}

/// Event handler for terminal input
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Poll for the next event
    pub fn next(&self) -> Option<AppEvent> {
        if event::poll(self.tick_rate).ok()? {
            match event::read().ok()? {
                Event::Key(key) => Some(AppEvent::Key(key)),
                Event::Resize(w, h) => Some(AppEvent::Resize(w, h)),
                _ => None,
            }
        } else {
            Some(AppEvent::Tick)
        }
    }
}

/// Check if a key event matches a specific key
pub fn is_key(event: &KeyEvent, code: KeyCode) -> bool {
    event.code == code && event.modifiers.is_empty()
}

/// Check if a key event matches a key with modifiers
pub fn is_key_with_mod(event: &KeyEvent, code: KeyCode, modifiers: KeyModifiers) -> bool {
    event.code == code && event.modifiers == modifiers
}

/// Check if this is a quit key combination
pub fn is_quit(event: &KeyEvent) -> bool {
    matches!(
        (event.code, event.modifiers),
        (KeyCode::Char('q'), KeyModifiers::NONE)
            | (KeyCode::Char('c'), KeyModifiers::CONTROL)
    )
}

/// Check for navigation keys (returns delta)
pub fn navigation_delta(event: &KeyEvent) -> Option<i32> {
    match (event.code, event.modifiers) {
        // Arrow keys (primary)
        (KeyCode::Up, KeyModifiers::NONE) => Some(-1),
        (KeyCode::Down, KeyModifiers::NONE) => Some(1),
        (KeyCode::PageUp, KeyModifiers::NONE) => Some(-10),
        (KeyCode::PageDown, KeyModifiers::NONE) => Some(10),
        (KeyCode::Home, KeyModifiers::NONE) => Some(i32::MIN),
        (KeyCode::End, KeyModifiers::NONE) => Some(i32::MAX),

        // Vi-style (alternative)
        (KeyCode::Char('k'), KeyModifiers::NONE) => Some(-1),
        (KeyCode::Char('j'), KeyModifiers::NONE) => Some(1),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => Some(-10),
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => Some(10),
        (KeyCode::Char('g'), KeyModifiers::NONE) => Some(i32::MIN),
        (KeyCode::Char('G'), KeyModifiers::SHIFT) => Some(i32::MAX),

        _ => None,
    }
}

/// Check for tab navigation (returns delta)
pub fn tab_delta(event: &KeyEvent) -> Option<i32> {
    match (event.code, event.modifiers) {
        (KeyCode::Tab, KeyModifiers::NONE) => Some(1),
        (KeyCode::BackTab, KeyModifiers::SHIFT) => Some(-1),
        (KeyCode::Char('l'), KeyModifiers::NONE) => Some(1),
        (KeyCode::Char('h'), KeyModifiers::NONE) => Some(-1),
        _ => None,
    }
}

/// Check for tab number keys (1-6)
pub fn tab_number(event: &KeyEvent) -> Option<usize> {
    match event.code {
        KeyCode::Char('1') => Some(0),
        KeyCode::Char('2') => Some(1),
        KeyCode::Char('3') => Some(2),
        KeyCode::Char('4') => Some(3),
        KeyCode::Char('5') => Some(4),
        KeyCode::Char('6') => Some(5),
        _ => None,
    }
}
