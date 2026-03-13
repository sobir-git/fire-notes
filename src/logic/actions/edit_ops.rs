#![allow(dead_code)]
//! Text editing action handlers (char input, delete, undo/redo, select, line ops).

use crate::app::focus::Focus;
use crate::app::input_handler::InputHandler;
use crate::app::state::AppResult;
use crate::logic::AppLogic;
use crate::ui::Rect;

/// Delegate an edit method to rename_input if TabRename is active.
/// Redraw + reset cursor blink on success; returns early either way.
macro_rules! rename_edit {
    ($self:expr, $method:ident $(, $arg:expr)*) => {{
        if matches!($self.focus, Focus::TabRename) {
            if let Some((_, fw_input)) = &mut $self.rename_input {
                let r = InputHandler::$method(&mut fw_input.state $(, $arg)*);
                if r.was_handled() { $self.reset_cursor_blink(); }
                return AppResult::Redraw;
            }
            return AppResult::Ok;
        }
    }};
}

impl AppLogic {
    fn window_rect(&self) -> Rect {
        Rect { x: 0.0, y: 0.0, width: self.width, height: self.height }
    }

    /// Re-run the search filter and re-layout the picker after an edit.
    fn update_picker_filter(&mut self) {
        let window = self.window_rect();
        let scale  = self.scale;
        if let Some(picker) = &mut self.notes_picker {
            picker.update_filter(window, scale);
        }
    }

    /// Ensure the picker input cursor is scrolled into view.
    fn scroll_picker_cursor_visible(&mut self) {
        let char_width = self.char_width_hint;
        if let Some(picker) = &mut self.notes_picker {
            picker.search.ensure_cursor_visible(char_width);
        }
    }

    pub(crate) fn handle_char(&mut self, ch: char) -> AppResult {
        if matches!(self.focus, Focus::NotesPicker) {
            if let Some(picker) = &mut self.notes_picker {
                let _ = picker.search.state.handle_char(ch);
            }
            self.update_picker_filter();
            self.reset_cursor_blink();
            self.scroll_picker_cursor_visible();
            return AppResult::Redraw;
        }
        rename_edit!(self, handle_char, ch);
        let line = self.tabs[self.active_tab].cursor_line();
        let col  = self.tabs[self.active_tab].cursor_col();
        self.tabs[self.active_tab].insert_char(ch);
        if !ch.is_control() {
            self.typing_flame_positions.push((line, col, std::time::Instant::now()));
            let now = std::time::Instant::now();
            self.typing_flame_positions.retain(|(_, _, ts)| {
                now.duration_since(*ts).as_secs_f32() < crate::config::flame::TYPING_FLAME_EXPIRY
            });
        }
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_backspace(&mut self) -> AppResult {
        if matches!(self.focus, Focus::NotesPicker) {
            if let Some(picker) = &mut self.notes_picker {
                let _ = picker.search.state.handle_backspace();
            }
            self.update_picker_filter();
            self.reset_cursor_blink();
            self.scroll_picker_cursor_visible();
            return AppResult::Redraw;
        }
        rename_edit!(self, handle_backspace);
        self.tabs[self.active_tab].backspace();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_delete(&mut self) -> AppResult {
        if matches!(self.focus, Focus::NotesPicker) {
            if let Some(picker) = &mut self.notes_picker {
                let _ = picker.search.state.handle_delete();
            }
            self.update_picker_filter();
            self.reset_cursor_blink();
            self.scroll_picker_cursor_visible();
            return AppResult::Redraw;
        }
        rename_edit!(self, handle_delete);
        self.tabs[self.active_tab].delete();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_delete_word_left(&mut self) -> AppResult {
        if matches!(self.focus, Focus::NotesPicker) {
            if let Some(picker) = &mut self.notes_picker {
                let _ = picker.search.state.handle_delete_word_left();
            }
            self.update_picker_filter();
            self.reset_cursor_blink();
            self.scroll_picker_cursor_visible();
            return AppResult::Redraw;
        }
        rename_edit!(self, handle_delete_word_left);
        self.tabs[self.active_tab].delete_word_left();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_delete_word_right(&mut self) -> AppResult {
        if matches!(self.focus, Focus::NotesPicker) {
            if let Some(picker) = &mut self.notes_picker {
                let _ = picker.search.state.handle_delete_word_right();
            }
            self.update_picker_filter();
            self.reset_cursor_blink();
            self.scroll_picker_cursor_visible();
            return AppResult::Redraw;
        }
        rename_edit!(self, handle_delete_word_right);
        self.tabs[self.active_tab].delete_word_right();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_select_all(&mut self) -> AppResult {
        rename_edit!(self, handle_select_all);
        self.tabs[self.active_tab].select_all();
        AppResult::Redraw
    }

    pub(crate) fn handle_undo(&mut self) -> AppResult {
        if matches!(self.focus, Focus::TabRename) { return AppResult::Ok; }
        if self.tabs[self.active_tab].undo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn handle_redo(&mut self) -> AppResult {
        if matches!(self.focus, Focus::TabRename) { return AppResult::Ok; }
        if self.tabs[self.active_tab].redo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn handle_move_lines_up(&mut self) -> AppResult {
        if matches!(self.focus, Focus::NotesPicker | Focus::TabRename) { return AppResult::Ok; }
        if self.tabs[self.active_tab].move_lines_up() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn handle_move_lines_down(&mut self) -> AppResult {
        if matches!(self.focus, Focus::NotesPicker | Focus::TabRename) { return AppResult::Ok; }
        if self.tabs[self.active_tab].move_lines_down() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn toggle_word_wrap(&mut self) -> AppResult {
        if matches!(self.focus, Focus::NotesPicker | Focus::TabRename) { return AppResult::Ok; }
        self.tabs[self.active_tab].toggle_word_wrap();
        self.auto_scroll();
        AppResult::Redraw
    }
}
