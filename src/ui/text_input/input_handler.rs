//! `InputHandler` implementation for `TextInput`.
//!
//! All text-field keyboard logic lives here — local to the widget.
//! `Focus` in `app/` delegates to this; it never repeats these operations.

use crate::app::input_handler::{InputHandler, InputResult};

use super::TextInput;

impl InputHandler for TextInput {
    fn handle_char(&mut self, ch: char) -> InputResult {
        self.insert_char(ch);
        InputResult::Handled
    }

    fn handle_backspace(&mut self) -> InputResult {
        self.backspace();
        InputResult::Handled
    }

    fn handle_delete(&mut self) -> InputResult {
        self.delete();
        InputResult::Handled
    }

    fn handle_delete_word_left(&mut self) -> InputResult {
        self.delete_word_left();
        InputResult::Handled
    }

    fn handle_delete_word_right(&mut self) -> InputResult {
        self.delete_word_right();
        InputResult::Handled
    }

    fn handle_select_all(&mut self) -> InputResult {
        self.select_all();
        InputResult::Handled
    }

    fn move_left(&mut self, selecting: bool) -> InputResult {
        self.move_left(selecting);
        InputResult::Handled
    }

    fn move_right(&mut self, selecting: bool) -> InputResult {
        self.move_right(selecting);
        InputResult::Handled
    }

    fn move_word_left(&mut self, selecting: bool) -> InputResult {
        self.move_word_left(selecting);
        InputResult::Handled
    }

    fn move_word_right(&mut self, selecting: bool) -> InputResult {
        self.move_word_right(selecting);
        InputResult::Handled
    }

    fn move_to_line_start(&mut self, selecting: bool) -> InputResult {
        self.move_to_start(selecting);
        InputResult::Handled
    }

    fn move_to_line_end(&mut self, selecting: bool) -> InputResult {
        self.move_to_end(selecting);
        InputResult::Handled
    }

    fn move_to_start(&mut self, selecting: bool) -> InputResult {
        self.move_to_start(selecting);
        InputResult::Handled
    }

    fn move_to_end(&mut self, selecting: bool) -> InputResult {
        self.move_to_end(selecting);
        InputResult::Handled
    }

    fn undo(&mut self) -> InputResult {
        InputResult::NotHandled
    }

    fn redo(&mut self) -> InputResult {
        InputResult::NotHandled
    }

    fn copy(&self) -> Option<String> {
        self.copy()
    }

    fn cut(&mut self) -> Option<String> {
        self.cut()
    }

    fn paste(&mut self, text: &str) -> InputResult {
        self.paste(text);
        InputResult::Handled
    }
}
