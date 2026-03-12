//! `InputHandler` for `Focus` — thin delegation layer.
//!
//! All text-field logic lives in `TextInput: InputHandler` (ui/text_input/).
//! Focus only adds the two things that are genuinely its own concern:
//!   - `move_up`/`move_down` in `NotesPicker` navigate the list, not the cursor.
//!   - Text-mutating ops in `NotesPicker` must re-run the filter after editing.
//! Everything else is a single `delegate!()` call.

use super::Focus;
use crate::app::input_handler::{InputHandler, InputResult};

/// Returns `InputResult::NotHandled` if `Editor`, delegates `$method` to the
/// active `TextInput` otherwise, then runs `$post` if the result was `Handled`.
macro_rules! delegate {
    // variant: no post-action
    ($self:expr, $method:ident $(, $arg:expr)*) => {{
        match $self.active_input_mut() {
            None    => InputResult::NotHandled,
            Some(i) => InputHandler::$method(i $(, $arg)*),
        }
    }};
    // variant: post-action run only when the result is Handled
    ($self:expr, $method:ident $(, $arg:expr)* => $post:expr) => {{
        match $self.active_input_mut() {
            None    => InputResult::NotHandled,
            Some(i) => {
                let r = InputHandler::$method(i $(, $arg)*);
                if r == InputResult::Handled { $post; }
                r
            }
        }
    }};
}

impl Focus {
    /// The `TextInput` that currently has keyboard focus, if any.
    /// `Editor` has no single-line input, so returns `None`.
    fn active_input_mut(&mut self) -> Option<&mut crate::ui::TextInput> {
        match self {
            Focus::Editor => None,
            Focus::TabRename { input, .. } => Some(input),
            Focus::NotesPicker { input, .. } => Some(input),
        }
    }

    fn active_input(&self) -> Option<&crate::ui::TextInput> {
        match self {
            Focus::Editor => None,
            Focus::TabRename { input, .. } => Some(input),
            Focus::NotesPicker { input, .. } => Some(input),
        }
    }
}

impl InputHandler for Focus {
    fn handle_char(&mut self, ch: char) -> InputResult {
        delegate!(self, handle_char, ch => self.update_notes_filter())
    }

    fn handle_backspace(&mut self) -> InputResult {
        delegate!(self, handle_backspace => self.update_notes_filter())
    }

    fn handle_delete(&mut self) -> InputResult {
        delegate!(self, handle_delete => self.update_notes_filter())
    }

    fn handle_delete_word_left(&mut self) -> InputResult {
        delegate!(self, handle_delete_word_left => self.update_notes_filter())
    }

    fn handle_delete_word_right(&mut self) -> InputResult {
        delegate!(self, handle_delete_word_right => self.update_notes_filter())
    }

    fn handle_select_all(&mut self) -> InputResult {
        delegate!(self, handle_select_all)
    }

    fn move_left(&mut self, selecting: bool) -> InputResult {
        delegate!(self, move_left, selecting)
    }

    fn move_right(&mut self, selecting: bool) -> InputResult {
        delegate!(self, move_right, selecting)
    }

    // NotesPicker: up/down navigate the list, not the cursor.
    // TabRename: up/down are meaningless in a single-line field — ignore.
    fn move_up(&mut self, _selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } => InputResult::Ignored,
            Focus::NotesPicker { .. } => { self.notes_picker_up(); InputResult::Handled }
        }
    }

    fn move_down(&mut self, _selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } => InputResult::Ignored,
            Focus::NotesPicker { .. } => { self.notes_picker_down(); InputResult::Handled }
        }
    }

    fn move_word_left(&mut self, selecting: bool) -> InputResult {
        delegate!(self, move_word_left, selecting)
    }

    fn move_word_right(&mut self, selecting: bool) -> InputResult {
        delegate!(self, move_word_right, selecting)
    }

    fn move_to_line_start(&mut self, selecting: bool) -> InputResult {
        delegate!(self, move_to_line_start, selecting)
    }

    fn move_to_line_end(&mut self, selecting: bool) -> InputResult {
        delegate!(self, move_to_line_end, selecting)
    }

    fn move_to_start(&mut self, selecting: bool) -> InputResult {
        delegate!(self, move_to_start, selecting)
    }

    fn move_to_end(&mut self, selecting: bool) -> InputResult {
        delegate!(self, move_to_end, selecting)
    }

    fn undo(&mut self) -> InputResult {
        delegate!(self, undo)
    }

    fn redo(&mut self) -> InputResult {
        delegate!(self, redo)
    }

    fn copy(&self) -> Option<String> {
        self.active_input().and_then(|i| i.copy())
    }

    fn cut(&mut self) -> Option<String> {
        // Need to split borrow: read result, then update filter.
        let is_picker = matches!(self, Focus::NotesPicker { .. });
        let result = self.active_input_mut().and_then(|i| i.cut());
        if result.is_some() && is_picker { self.update_notes_filter(); }
        result
    }

    fn paste(&mut self, text: &str) -> InputResult {
        delegate!(self, paste, text => self.update_notes_filter())
    }
}
