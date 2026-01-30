//! Unified input handling through the Focus system
//!
//! This module uses trait-based dispatch: all input first goes to Focus's
//! InputHandler implementation. If Focus returns NotHandled (meaning the
//! editor should handle it), we fall through to the editor. This eliminates
//! manual focus checks - adding a new widget only requires updating Focus.

use super::input_handler::InputHandler;
use super::state::AppResult;
use super::App;

impl App {
    /// Handle character input - routes to focused component
    pub fn handle_char(&mut self, ch: char) -> AppResult {
        let result = self.focus.handle_char(ch);
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        // Editor mode - delegate to active tab
        let line = self.tabs[self.active_tab].cursor_line();
        let col = self.tabs[self.active_tab].cursor_col();

        self.tabs[self.active_tab].insert_char(ch);

        // Record typed character position for flame emission
        if !ch.is_control() {
            self.ui_state
                .typing_flame_positions
                .push((line, col, std::time::Instant::now()));

            // Keep only recent positions
            let now = std::time::Instant::now();
            self.ui_state.typing_flame_positions.retain(|(_, _, ts)| {
                now.duration_since(*ts).as_secs_f32() < crate::config::flame::TYPING_FLAME_EXPIRY
            });
        }

        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    /// Handle backspace - routes to focused component
    pub fn handle_backspace(&mut self) -> AppResult {
        let result = self.focus.handle_backspace();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].backspace();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    /// Handle delete word left (Ctrl+Backspace)
    pub fn handle_delete_word_left(&mut self) -> AppResult {
        let result = self.focus.handle_delete_word_left();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].delete_word_left();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    /// Handle delete key
    pub fn handle_delete(&mut self) -> AppResult {
        let result = self.focus.handle_delete();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].delete();
        self.auto_scroll();
        AppResult::Redraw
    }

    /// Handle delete word right (Ctrl+Delete)
    pub fn handle_delete_word_right(&mut self) -> AppResult {
        let result = self.focus.handle_delete_word_right();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].delete_word_right();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    /// Handle select all (Ctrl+A)
    pub fn handle_select_all(&mut self) -> AppResult {
        let result = self.focus.handle_select_all();
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].select_all();
        AppResult::Redraw
    }

    // =========================================================================
    // Cursor movement
    // =========================================================================

    pub fn move_cursor_left(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_left(selecting);
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].move_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_right(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_right(selecting);
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].move_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_left(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_word_left(selecting);
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].move_word_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_right(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_word_right(selecting);
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].move_word_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_up(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_up(selecting);
        if result.was_handled() {
            return result.into();
        }

        self.tabs[self.active_tab].move_up(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_down(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_down(selecting);
        if result.was_handled() {
            return result.into();
        }

        self.tabs[self.active_tab].move_down(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_start(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_to_line_start(selecting);
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].move_to_line_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_end(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_to_line_end(selecting);
        if result.was_handled() {
            self.ui_state.reset_cursor_blink();
            return result.into();
        }

        self.tabs[self.active_tab].move_to_line_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_start(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_to_start(selecting);
        if result.was_handled() {
            return result.into();
        }

        self.tabs[self.active_tab].move_to_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_end(&mut self, selecting: bool) -> AppResult {
        let result = self.focus.move_to_end(selecting);
        if result.was_handled() {
            return result.into();
        }

        self.tabs[self.active_tab].move_to_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    // =========================================================================
    // Line operations (editor only - no widget handles these)
    // =========================================================================

    pub fn handle_move_lines_up(&mut self) -> AppResult {
        if !matches!(self.focus, super::focus::Focus::Editor) {
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
        if !matches!(self.focus, super::focus::Focus::Editor) {
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
        let result = self.focus.undo();
        if result.was_handled() {
            return result.into();
        }

        if self.tabs[self.active_tab].undo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_redo(&mut self) -> AppResult {
        let result = self.focus.redo();
        if result.was_handled() {
            return result.into();
        }

        if self.tabs[self.active_tab].redo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    // =========================================================================
    // View settings (editor only)
    // =========================================================================

    pub fn toggle_word_wrap(&mut self) -> AppResult {
        if !matches!(self.focus, super::focus::Focus::Editor) {
            return AppResult::Ok;
        }

        self.tabs[self.active_tab].toggle_word_wrap();
        self.auto_scroll();
        AppResult::Redraw
    }

    // =========================================================================
    // Clipboard operations
    // =========================================================================

    pub fn handle_copy(&mut self) -> AppResult {
        if let Some(text) = self.focus.copy() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
            return AppResult::Ok;
        }

        // Only allow editor copy when in editor focus
        if !matches!(self.focus, super::focus::Focus::Editor) {
            return AppResult::Ok;
        }

        if let Some(text) = self.tabs[self.active_tab].copy_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
        AppResult::Ok
    }

    pub fn handle_cut(&mut self) -> AppResult {
        if let Some(text) = self.focus.cut() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
            self.ui_state.reset_cursor_blink();
            return AppResult::Redraw;
        }

        // Only allow editor cut when in editor focus
        if !matches!(self.focus, super::focus::Focus::Editor) {
            return AppResult::Ok;
        }

        if let Some(text) = self.tabs[self.active_tab].cut_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
            self.tabs[self.active_tab].auto_save();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_paste(&mut self) -> AppResult {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                let result = self.focus.paste(&text);
                if result.was_handled() {
                    self.ui_state.reset_cursor_blink();
                    return result.into();
                }

                // Only allow editor paste when in editor focus
                if matches!(self.focus, super::focus::Focus::Editor) {
                    self.tabs[self.active_tab].paste_text(&text);
                    self.tabs[self.active_tab].auto_save();
                    self.auto_scroll();
                    return AppResult::Redraw;
                }
            }
        }
        AppResult::Ok
    }
}
