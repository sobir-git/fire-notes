//! Text editing and cursor movement

use super::state::AppResult;
use super::App;

impl App {
    // =========================================================================
    // Text editing
    // =========================================================================

    pub fn handle_char(&mut self, ch: char) -> AppResult {
        if let Some(ref mut input) = self.state.rename_input {
            input.insert_char(ch);
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
        }
        
        // Get cursor position before inserting
        let line = self.tabs[self.active_tab].cursor_line();
        let col = self.tabs[self.active_tab].cursor_col();
        
        self.tabs[self.active_tab].insert_char(ch);
        
        // Record typed character position for flame emission
        if !ch.is_control() {
            self.state.typing_flame_positions.push((line, col, std::time::Instant::now()));
            
            // Keep only recent positions (last 1.0 second)
            let now = std::time::Instant::now();
            self.state.typing_flame_positions.retain(|(_, _, timestamp)| {
                now.duration_since(*timestamp).as_secs_f32() < 1.0
            });
        }
        
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_backspace(&mut self) -> AppResult {
        if let Some(ref mut input) = self.state.rename_input {
            input.backspace();
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].backspace();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete_word_left(&mut self) -> AppResult {
        if let Some(ref mut input) = self.state.rename_input {
            input.delete_word_left();
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].delete_word_left();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete(&mut self) -> AppResult {
        if let Some(ref mut input) = self.state.rename_input {
            input.delete();
            self.state.reset_cursor_blink();
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
        if let Some(ref mut input) = self.state.rename_input {
            input.move_left(selecting);
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].move_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_right(&mut self, selecting: bool) -> AppResult {
        if let Some(ref mut input) = self.state.rename_input {
            input.move_right(selecting);
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].move_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_left(&mut self, selecting: bool) -> AppResult {
        if let Some(ref mut input) = self.state.rename_input {
            input.move_word_left(selecting);
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].move_word_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_right(&mut self, selecting: bool) -> AppResult {
        if let Some(ref mut input) = self.state.rename_input {
            input.move_word_right(selecting);
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
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
        if let Some(ref mut input) = self.state.rename_input {
            input.move_to_start(selecting);
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].move_to_line_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_end(&mut self, selecting: bool) -> AppResult {
        if let Some(ref mut input) = self.state.rename_input {
            input.move_to_end(selecting);
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
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
        if let Some(ref mut input) = self.state.rename_input {
            input.select_all();
            self.state.reset_cursor_blink();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].select_all();
        AppResult::Redraw
    }
}
