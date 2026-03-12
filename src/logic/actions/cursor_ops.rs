#![allow(dead_code)]
//! Cursor movement action handlers.

use crate::app::input_handler::InputHandler;
use crate::app::state::AppResult;
use crate::logic::AppLogic;

impl AppLogic {
    pub(crate) fn move_cursor_left(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_left(selecting);
        if result.was_handled() { self.ui_state.reset_cursor_blink(); return result.into(); }
        self.tabs[self.active_tab].move_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_right(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_right(selecting);
        if result.was_handled() { self.ui_state.reset_cursor_blink(); return result.into(); }
        self.tabs[self.active_tab].move_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_up(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_up(selecting);
        if result.was_handled() { return result.into(); }
        self.tabs[self.active_tab].move_up(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_down(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_down(selecting);
        if result.was_handled() { return result.into(); }
        self.tabs[self.active_tab].move_down(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_word_left(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_word_left(selecting);
        if result.was_handled() { self.ui_state.reset_cursor_blink(); return result.into(); }
        self.tabs[self.active_tab].move_word_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_word_right(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_word_right(selecting);
        if result.was_handled() { self.ui_state.reset_cursor_blink(); return result.into(); }
        self.tabs[self.active_tab].move_word_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_to_line_start(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_to_line_start(selecting);
        if result.was_handled() { self.ui_state.reset_cursor_blink(); return result.into(); }
        self.tabs[self.active_tab].move_to_line_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_to_line_end(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_to_line_end(selecting);
        if result.was_handled() { self.ui_state.reset_cursor_blink(); return result.into(); }
        self.tabs[self.active_tab].move_to_line_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_to_start(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_to_start(selecting);
        if result.was_handled() { return result.into(); }
        self.tabs[self.active_tab].move_to_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_to_end(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_to_end(selecting);
        if result.was_handled() { return result.into(); }
        self.tabs[self.active_tab].move_to_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn page_up(&mut self, selecting: bool) -> AppResult {
        let visible = self.visible_line_count();
        for _ in 0..visible { self.tabs[self.active_tab].move_up(selecting); }
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn page_down(&mut self, selecting: bool) -> AppResult {
        let visible = self.visible_line_count();
        for _ in 0..visible { self.tabs[self.active_tab].move_down(selecting); }
        self.auto_scroll();
        AppResult::Redraw
    }
}
