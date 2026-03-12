//! InputHandler implementation for Focus — dispatches to the focused widget.

use super::Focus;
use crate::app::input_handler::{InputHandler, InputResult};

impl InputHandler for Focus {
    fn handle_char(&mut self, ch: char) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.insert_char(ch); InputResult::Handled }
            Focus::NotesPicker { input, .. } => {
                input.insert_char(ch);
                self.update_notes_filter();
                InputResult::Handled
            }
        }
    }

    fn handle_backspace(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.backspace(); InputResult::Handled }
            Focus::NotesPicker { input, .. } => {
                input.backspace();
                self.update_notes_filter();
                InputResult::Handled
            }
        }
    }

    fn handle_delete(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.delete(); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn handle_delete_word_left(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.delete_word_left(); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn handle_delete_word_right(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.delete_word_right(); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn handle_select_all(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.select_all(); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_left(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.move_left(selecting); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_right(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.move_right(selecting); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

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
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.move_word_left(selecting); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_word_right(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.move_word_right(selecting); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_to_line_start(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.move_to_start(selecting); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_to_line_end(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.move_to_end(selecting); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_to_start(&mut self, _selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } | Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_to_end(&mut self, _selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } | Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn copy(&self) -> Option<String> {
        match self {
            Focus::Editor => None,
            Focus::TabRename { input, .. } => input.copy(),
            Focus::NotesPicker { .. } => None,
        }
    }

    fn cut(&mut self) -> Option<String> {
        match self {
            Focus::Editor => None,
            Focus::TabRename { input, .. } => input.cut(),
            Focus::NotesPicker { .. } => None,
        }
    }

    fn paste(&mut self, text: &str) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => { input.paste(text); InputResult::Handled }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn undo(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } | Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn redo(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } | Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }
}
