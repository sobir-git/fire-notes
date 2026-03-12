//! Edit input handlers — char, delete, select, clipboard, undo/redo, word-wrap.

use super::super::input_handler::InputHandler;
use super::super::state::AppResult;
use super::super::App;

impl App {
    pub fn handle_char(&mut self, ch: char) -> AppResult {
        let result = self.logic.focus.handle_char(ch);
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            return result.into();
        }

        let line = self.logic.tabs[self.logic.active_tab].cursor_line();
        let col = self.logic.tabs[self.logic.active_tab].cursor_col();
        self.logic.tabs[self.logic.active_tab].insert_char(ch);

        if !ch.is_control() {
            self.logic.ui_state
                .typing_flame_positions
                .push((line, col, std::time::Instant::now()));
            let now = std::time::Instant::now();
            self.logic.ui_state.typing_flame_positions.retain(|(_, _, ts)| {
                now.duration_since(*ts).as_secs_f32() < crate::config::flame::TYPING_FLAME_EXPIRY
            });
        }

        self.logic.tabs[self.logic.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_backspace(&mut self) -> AppResult {
        let result = self.logic.focus.handle_backspace();
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].backspace();
        self.logic.tabs[self.logic.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete_word_left(&mut self) -> AppResult {
        let result = self.logic.focus.handle_delete_word_left();
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].delete_word_left();
        self.logic.tabs[self.logic.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete(&mut self) -> AppResult {
        let result = self.logic.focus.handle_delete();
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].delete();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete_word_right(&mut self) -> AppResult {
        let result = self.logic.focus.handle_delete_word_right();
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].delete_word_right();
        self.logic.tabs[self.logic.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_select_all(&mut self) -> AppResult {
        let result = self.logic.focus.handle_select_all();
        if result.was_handled() {
            self.logic.ui_state.reset_cursor_blink();
            return result.into();
        }
        self.logic.tabs[self.logic.active_tab].select_all();
        AppResult::Redraw
    }

    pub fn handle_move_lines_up(&mut self) -> AppResult {
        if !matches!(self.logic.focus, super::super::focus::Focus::Editor) {
            return AppResult::Ok;
        }
        if self.logic.tabs[self.logic.active_tab].move_lines_up() {
            self.logic.tabs[self.logic.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_move_lines_down(&mut self) -> AppResult {
        if !matches!(self.logic.focus, super::super::focus::Focus::Editor) {
            return AppResult::Ok;
        }
        if self.logic.tabs[self.logic.active_tab].move_lines_down() {
            self.logic.tabs[self.logic.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_undo(&mut self) -> AppResult {
        let result = self.logic.focus.undo();
        if result.was_handled() {
            return result.into();
        }
        if self.logic.tabs[self.logic.active_tab].undo() {
            self.logic.tabs[self.logic.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_redo(&mut self) -> AppResult {
        let result = self.logic.focus.redo();
        if result.was_handled() {
            return result.into();
        }
        if self.logic.tabs[self.logic.active_tab].redo() {
            self.logic.tabs[self.logic.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn toggle_word_wrap(&mut self) -> AppResult {
        if !matches!(self.logic.focus, super::super::focus::Focus::Editor) {
            return AppResult::Ok;
        }
        self.logic.tabs[self.logic.active_tab].toggle_word_wrap();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_copy(&mut self) -> AppResult {
        if let Some(text) = self.logic.focus.copy() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
            return AppResult::Ok;
        }
        if !matches!(self.logic.focus, super::super::focus::Focus::Editor) {
            return AppResult::Ok;
        }
        if let Some(text) = self.logic.tabs[self.logic.active_tab].copy_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
        AppResult::Ok
    }

    pub fn handle_cut(&mut self) -> AppResult {
        if let Some(text) = self.logic.focus.cut() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
            self.logic.ui_state.reset_cursor_blink();
            return AppResult::Redraw;
        }
        if !matches!(self.logic.focus, super::super::focus::Focus::Editor) {
            return AppResult::Ok;
        }
        if let Some(text) = self.logic.tabs[self.logic.active_tab].cut_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
            self.logic.tabs[self.logic.active_tab].auto_save();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_paste(&mut self) -> AppResult {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                let result = self.logic.focus.paste(&text);
                if result.was_handled() {
                    self.logic.ui_state.reset_cursor_blink();
                    return result.into();
                }
                if matches!(self.logic.focus, super::super::focus::Focus::Editor) {
                    self.logic.tabs[self.logic.active_tab].paste_text(&text);
                    self.logic.tabs[self.logic.active_tab].auto_save();
                    self.auto_scroll();
                    return AppResult::Redraw;
                }
            }
        }
        AppResult::Ok
    }
}
