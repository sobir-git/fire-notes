//! Focus management - determines which component receives keyboard input
//!
//! This module provides a clean abstraction for input routing. Instead of
//! checking `renaming_tab.is_some()` in every input handler, we have a single
//! Focus enum that determines where input goes.

use crate::ui::TextInput;

/// Represents what currently has keyboard focus
#[derive(Debug, Clone)]
pub enum Focus {
    /// Main text editor (the active tab's content)
    Editor,
    /// Tab title rename input
    TabRename {
        tab_index: usize,
        input: TextInput,
    },
}

impl Default for Focus {
    fn default() -> Self {
        Focus::Editor
    }
}

impl Focus {
    /// Check if we're currently renaming a tab
    pub fn is_renaming(&self) -> bool {
        matches!(self, Focus::TabRename { .. })
    }

    /// Get the tab index being renamed, if any
    pub fn renaming_tab_index(&self) -> Option<usize> {
        match self {
            Focus::TabRename { tab_index, .. } => Some(*tab_index),
            _ => None,
        }
    }

    /// Get mutable access to the rename input, if active
    pub fn rename_input_mut(&mut self) -> Option<&mut TextInput> {
        match self {
            Focus::TabRename { input, .. } => Some(input),
            _ => None,
        }
    }

    /// Get read access to the rename input, if active
    pub fn rename_input(&self) -> Option<&TextInput> {
        match self {
            Focus::TabRename { input, .. } => Some(input),
            _ => None,
        }
    }

    /// Start renaming a tab
    pub fn start_rename(tab_index: usize, current_title: &str) -> Self {
        let mut input = TextInput::new(current_title.to_string());
        input.select_all();
        Focus::TabRename { tab_index, input }
    }

    /// Confirm rename and return the new title, transitioning back to Editor focus
    pub fn confirm_rename(&mut self) -> Option<(usize, String)> {
        match std::mem::take(self) {
            Focus::TabRename { tab_index, input } => {
                let title = input.text().trim().to_string();
                *self = Focus::Editor;
                if title.is_empty() {
                    None
                } else {
                    Some((tab_index, title))
                }
            }
            other => {
                *self = other;
                None
            }
        }
    }

    /// Cancel rename and return to Editor focus
    pub fn cancel_rename(&mut self) -> bool {
        if self.is_renaming() {
            *self = Focus::Editor;
            true
        } else {
            false
        }
    }
}
