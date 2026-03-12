#![allow(dead_code)]
//! Text editing action handlers (char input, delete, undo/redo, select, line ops).

use crate::app::focus::Focus;
use crate::app::input_handler::InputHandler;
use crate::app::state::AppResult;
use crate::logic::AppLogic;

impl AppLogic {
    /// Ensure the picker input cursor is scrolled into view.
    /// Must be called after any keyboard edit that mutates the picker input.
    fn scroll_picker_cursor_visible(&mut self) {
        let list_len = self.focus.notes_picker_state()
            .map(|(_, list)| list.len()).unwrap_or(0);
        let layout = crate::ui::NotesPicker::new(self.width, self.height, self.scale, list_len);
        if let Some(input) = self.focus.notes_picker_input_mut() {
            layout.ensure_input_cursor_visible(input, self.char_width_hint);
        }
    }

    pub(crate) fn handle_char(&mut self, ch: char) -> AppResult {
        let result = self.focus.handle_char(ch);
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            if matches!(self.focus, Focus::NotesPicker { .. }) {
                self.scroll_picker_cursor_visible();
            }
            return result.into();
        }
        let line = self.tabs[self.active_tab].cursor_line();
        let col  = self.tabs[self.active_tab].cursor_col();
        self.tabs[self.active_tab].insert_char(ch);
        if !ch.is_control() {
            self.ui_state.typing_flame_positions.push((line, col, std::time::Instant::now()));
            let now = std::time::Instant::now();
            self.ui_state.typing_flame_positions.retain(|(_, _, ts)| {
                now.duration_since(*ts).as_secs_f32() < crate::config::flame::TYPING_FLAME_EXPIRY
            });
        }
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_backspace(&mut self) -> AppResult {
        let result = self.focus.handle_backspace();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            if matches!(self.focus, Focus::NotesPicker { .. }) {
                self.scroll_picker_cursor_visible();
            }
            return result.into();
        }
        self.tabs[self.active_tab].backspace();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_delete(&mut self) -> AppResult {
        let result = self.focus.handle_delete();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            if matches!(self.focus, Focus::NotesPicker { .. }) {
                self.scroll_picker_cursor_visible();
            }
            return result.into();
        }
        self.tabs[self.active_tab].delete();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_delete_word_left(&mut self) -> AppResult {
        let result = self.focus.handle_delete_word_left();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            if matches!(self.focus, Focus::NotesPicker { .. }) {
                self.scroll_picker_cursor_visible();
            }
            return result.into();
        }
        self.tabs[self.active_tab].delete_word_left();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_delete_word_right(&mut self) -> AppResult {
        let result = self.focus.handle_delete_word_right();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            if matches!(self.focus, Focus::NotesPicker { .. }) {
                self.scroll_picker_cursor_visible();
            }
            return result.into();
        }
        self.tabs[self.active_tab].delete_word_right();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_select_all(&mut self) -> AppResult {
        let result = self.focus.handle_select_all();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }
        self.tabs[self.active_tab].select_all();
        AppResult::Redraw
    }

    pub(crate) fn handle_undo(&mut self) -> AppResult {
        let result = self.focus.undo();
        if result.was_handled() { return result.into(); }
        if self.tabs[self.active_tab].undo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn handle_redo(&mut self) -> AppResult {
        let result = self.focus.redo();
        if result.was_handled() { return result.into(); }
        if self.tabs[self.active_tab].redo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn handle_move_lines_up(&mut self) -> AppResult {
        if !matches!(self.focus, Focus::Editor) { return AppResult::Ok; }
        if self.tabs[self.active_tab].move_lines_up() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn handle_move_lines_down(&mut self) -> AppResult {
        if !matches!(self.focus, Focus::Editor) { return AppResult::Ok; }
        if self.tabs[self.active_tab].move_lines_down() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn toggle_word_wrap(&mut self) -> AppResult {
        if !matches!(self.focus, Focus::Editor) { return AppResult::Ok; }
        self.tabs[self.active_tab].toggle_word_wrap();
        self.auto_scroll();
        AppResult::Redraw
    }
}
