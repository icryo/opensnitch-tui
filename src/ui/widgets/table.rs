//! Sortable and filterable table widget

use ratatui::widgets::TableState;

/// Extended table state with sorting and filtering
pub struct SortableTableState {
    pub state: TableState,
    pub sort_column: usize,
    pub sort_ascending: bool,
    pub filter: String,
    pub filtered_indices: Vec<usize>,
}

impl SortableTableState {
    pub fn new() -> Self {
        Self {
            state: TableState::default(),
            sort_column: 0,
            sort_ascending: true,
            filter: String::new(),
            filtered_indices: Vec::new(),
        }
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.state.select(index);
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn toggle_sort(&mut self, column: usize) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
    }

    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.filtered_indices.clear();
    }
}

impl Default for SortableTableState {
    fn default() -> Self {
        Self::new()
    }
}
