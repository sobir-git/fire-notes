//! Trait-based input handling for focusable widgets
//!
//! Each focusable component implements InputHandler, and the Focus system
//! routes all input to the currently active widget. This eliminates the need
//! for manual focus checks in every input handler.

use super::state::AppResult;

/// Result of handling an input event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputResult {
    /// Input was handled, needs redraw
    Handled,
    /// Input was handled, no redraw needed
    Ignored,
    /// Input was not handled, should propagate to next handler
    NotHandled,
}

impl InputResult {
    pub fn needs_redraw(&self) -> bool {
        matches!(self, InputResult::Handled)
    }

    pub fn was_handled(&self) -> bool {
        !matches!(self, InputResult::NotHandled)
    }
}

impl From<InputResult> for AppResult {
    fn from(result: InputResult) -> Self {
        match result {
            InputResult::Handled => AppResult::Redraw,
            InputResult::Ignored | InputResult::NotHandled => AppResult::Ok,
        }
    }
}

/// Trait for components that can receive keyboard input when focused
pub trait InputHandler {
    /// Handle character input
    fn handle_char(&mut self, ch: char) -> InputResult;

    /// Handle backspace
    fn handle_backspace(&mut self) -> InputResult;

    /// Handle delete key
    fn handle_delete(&mut self) -> InputResult;

    /// Handle delete word left (Ctrl+Backspace)
    fn handle_delete_word_left(&mut self) -> InputResult {
        InputResult::Ignored
    }

    /// Handle delete word right (Ctrl+Delete)
    fn handle_delete_word_right(&mut self) -> InputResult {
        InputResult::Ignored
    }

    /// Handle select all (Ctrl+A)
    fn handle_select_all(&mut self) -> InputResult {
        InputResult::Ignored
    }

    /// Move cursor left
    fn move_left(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Move cursor right
    fn move_right(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Move cursor up
    fn move_up(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Move cursor down
    fn move_down(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Move cursor word left
    fn move_word_left(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Move cursor word right
    fn move_word_right(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Move to line start (Home)
    fn move_to_line_start(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Move to line end (End)
    fn move_to_line_end(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Move to document start (Ctrl+Home)
    fn move_to_start(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Move to document end (Ctrl+End)
    fn move_to_end(&mut self, _selecting: bool) -> InputResult {
        InputResult::Ignored
    }

    /// Handle Enter key - returns true if handled
    fn handle_enter(&mut self) -> InputResult {
        InputResult::NotHandled
    }

    /// Handle Escape key - returns true if handled
    fn handle_escape(&mut self) -> InputResult {
        InputResult::NotHandled
    }

    /// Copy selection to clipboard, returns the text if any
    fn copy(&self) -> Option<String> {
        None
    }

    /// Cut selection to clipboard, returns the text if any
    fn cut(&mut self) -> Option<String> {
        None
    }

    /// Paste text from clipboard
    fn paste(&mut self, _text: &str) -> InputResult {
        InputResult::Ignored
    }

    /// Handle undo
    fn undo(&mut self) -> InputResult {
        InputResult::Ignored
    }

    /// Handle redo
    fn redo(&mut self) -> InputResult {
        InputResult::Ignored
    }
}
