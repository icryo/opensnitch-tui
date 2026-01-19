//! Screen layout management

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Standard application layout areas
pub struct AppLayout {
    pub tabs: Rect,
    pub content: Rect,
    pub status: Rect,
}

impl AppLayout {
    /// Create layout from terminal area
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Tabs
                Constraint::Min(10),   // Content
                Constraint::Length(1), // Status bar
            ])
            .split(area);

        Self {
            tabs: chunks[0],
            content: chunks[1],
            status: chunks[2],
        }
    }
}

/// Layout with filter bar
pub struct FilterLayout {
    pub filter: Rect,
    pub content: Rect,
}

impl FilterLayout {
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Filter bar
                Constraint::Min(5),    // Content
            ])
            .split(area);

        Self {
            filter: chunks[0],
            content: chunks[1],
        }
    }
}

/// Two-panel layout (tree + table)
pub struct SplitLayout {
    pub left: Rect,
    pub right: Rect,
}

impl SplitLayout {
    pub fn new(area: Rect, left_percent: u16) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(left_percent),
                Constraint::Percentage(100 - left_percent),
            ])
            .split(area);

        Self {
            left: chunks[0],
            right: chunks[1],
        }
    }
}

/// Dialog/popup centered layout
pub struct DialogLayout {
    pub dialog: Rect,
}

impl DialogLayout {
    pub fn new(area: Rect, width_percent: u16, height_percent: u16) -> Self {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - height_percent) / 2),
                Constraint::Percentage(height_percent),
                Constraint::Percentage((100 - height_percent) / 2),
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - width_percent) / 2),
                Constraint::Percentage(width_percent),
                Constraint::Percentage((100 - width_percent) / 2),
            ])
            .split(vertical[1]);

        Self {
            dialog: horizontal[1],
        }
    }

    /// Create centered dialog with fixed dimensions
    pub fn centered(area: Rect, width: u16, height: u16) -> Self {
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;

        Self {
            dialog: Rect::new(x, y, width.min(area.width), height.min(area.height)),
        }
    }
}

/// Statistics dashboard layout
pub struct StatsLayout {
    pub summary: Rect,
    pub details: Vec<Rect>,
}

impl StatsLayout {
    pub fn new(area: Rect) -> Self {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Summary cards
                Constraint::Min(10),   // Detail tables
            ])
            .split(area);

        let detail_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(main_chunks[1]);

        Self {
            summary: main_chunks[0],
            details: detail_chunks.to_vec(),
        }
    }
}
