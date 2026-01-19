//! Tree view widget

/// Tree node for hierarchical display
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub label: String,
    pub expanded: bool,
    pub children: Vec<TreeNode>,
    pub data: Option<String>,
}

impl TreeNode {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            expanded: false,
            children: Vec::new(),
            data: None,
        }
    }

    pub fn with_children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    pub fn with_data(mut self, data: &str) -> Self {
        self.data = Some(data.to_string());
        self
    }

    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// Tree state
pub struct TreeState {
    pub selected: usize,
    pub offset: usize,
}

impl TreeState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            offset: 0,
        }
    }
}

impl Default for TreeState {
    fn default() -> Self {
        Self::new()
    }
}
