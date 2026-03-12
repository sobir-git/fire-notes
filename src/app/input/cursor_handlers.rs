//! Cursor movement input handlers.

use super::super::input_handler::InputHandler;
use super::super::state::AppResult;
use super::super::App;

impl App {
    pub fn move_cursor_left(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_left(selecting);
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            if self.logic.focus.is_notes_picker() { self.ensure_picker_cursor_visible(); }
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].move_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_right(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_right(selecting);
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            if self.logic.focus.is_notes_picker() { self.ensure_picker_cursor_visible(); }
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].move_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_left(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_word_left(selecting);
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            if self.logic.focus.is_notes_picker() { self.ensure_picker_cursor_visible(); }
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].move_word_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_right(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_word_right(selecting);
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            if self.logic.focus.is_notes_picker() { self.ensure_picker_cursor_visible(); }
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].move_word_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_up(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_up(selecting);
        if result.was_handled() { return result.into(); }
        self.logic.tabs[self.logic.active_tab].move_up(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_down(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_down(selecting);
        if result.was_handled() { return result.into(); }
        self.logic.tabs[self.logic.active_tab].move_down(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_start(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_to_line_start(selecting);
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            if self.logic.focus.is_notes_picker() { self.ensure_picker_cursor_visible(); }
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].move_to_line_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_end(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_to_line_end(selecting);
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            if self.logic.focus.is_notes_picker() { self.ensure_picker_cursor_visible(); }
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].move_to_line_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_start(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_to_start(selecting);
        if result.was_handled() { return result.into(); }
        self.logic.tabs[self.logic.active_tab].move_to_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_end(&mut self, selecting: bool) -> AppResult {
        let result = self.logic.focus.move_to_end(selecting);
        if result.was_handled() { return result.into(); }
        self.logic.tabs[self.logic.active_tab].move_to_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }
}
