pub mod alerts;
pub mod connections;
pub mod firewall;
pub mod nodes;
pub mod rules;
pub mod statistics;

use std::sync::Arc;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

use crate::app::state::AppState;
use crate::ui::theme::Theme;

/// Trait for tab implementations
pub trait Tab {
    fn render(&self, frame: &mut Frame, area: Rect, state: &Arc<AppState>, theme: &Theme);

    fn handle_key(&mut self, key: KeyEvent, state: &Arc<AppState>) -> impl std::future::Future<Output = ()> + Send;
}
