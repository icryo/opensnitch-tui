//! User action handling

use crate::grpc::notifications::NotificationAction;
use crate::models::{Rule, RuleAction, RuleDuration};

/// User-initiated actions
#[derive(Debug, Clone)]
pub enum UserAction {
    // Navigation
    NextTab,
    PrevTab,
    GoToTab(usize),
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    GoToTop,
    GoToBottom,

    // Selection
    Select,
    SelectMultiple,
    ClearSelection,

    // Filtering
    OpenFilter,
    ClearFilter,
    ApplyFilter(String),

    // Rule actions
    NewRule,
    EditRule(String),
    DeleteRule(String),
    EnableRule(String),
    DisableRule(String),
    DuplicateRule(String),

    // Firewall actions
    ToggleFirewall,
    EnableFirewall,
    DisableFirewall,
    NewFwRule,
    EditFwRule(String),
    DeleteFwRule(String),

    // Prompt actions
    AllowConnection,
    DenyConnection,
    RejectConnection,
    SetDuration(RuleDuration),
    ToggleAdvanced,

    // Node actions
    SwitchNode(String),
    RefreshNode,

    // General
    Refresh,
    Help,
    Quit,
    Cancel,
    Confirm,
}

/// Result of processing a user action
#[derive(Debug)]
pub enum ActionResult {
    Continue,
    Quit,
    ShowDialog(DialogType),
    CloseDialog,
    SendNotification(String, NotificationAction),
    UpdateFilter(String),
    SwitchTab(usize),
}

/// Dialog types
#[derive(Debug, Clone)]
pub enum DialogType {
    Prompt,
    RuleEditor(Option<Rule>),
    FwRuleEditor,
    Preferences,
    Help,
    Confirm(String),
}
