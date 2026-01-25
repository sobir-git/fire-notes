//! Text editing and cursor movement

use super::state::AppResult;
use super::App;

impl App {
    // =========================================================================
    // Text editing
    // =========================================================================

    pub fn handle_char(&mut self, ch: char) -> AppResult {
        if self.state.renaming_tab.is_some() {
            if !ch.is_control() {
                self.state.rename_buffer.push(ch);
                return AppResult::Redraw;
            }
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].insert_char(ch);
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_backspace(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            self.state.rename_buffer.pop();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].backspace();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete_word_left(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            self.state.rename_buffer.clear();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].delete_word_left();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            self.state.rename_buffer.clear();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].delete();
        self.auto_scroll();
        AppResult::Redraw
    }

    // =========================================================================
    // Cursor movement
    // =========================================================================

    pub fn move_cursor_left(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_right(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_left(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_word_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_right(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_word_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_up(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_up(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_down(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_down(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_start(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_to_line_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_end(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_to_line_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_start(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_to_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_end(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_to_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    // =========================================================================
    // Line operations
    // =========================================================================

    pub fn handle_move_lines_up(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        if self.tabs[self.active_tab].move_lines_up() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_move_lines_down(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        if self.tabs[self.active_tab].move_lines_down() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    // =========================================================================
    // Undo/Redo
    // =========================================================================

    pub fn handle_undo(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        if self.tabs[self.active_tab].undo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_redo(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        if self.tabs[self.active_tab].redo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    // =========================================================================
    // View settings
    // =========================================================================

    pub fn toggle_word_wrap(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].toggle_word_wrap();
        self.auto_scroll();
        AppResult::Redraw
    }

    // =========================================================================
    // Selection
    // =========================================================================

    pub fn handle_select_all(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].select_all();
        AppResult::Redraw
    }
}
