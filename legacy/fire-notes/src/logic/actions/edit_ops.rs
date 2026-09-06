//! Text editing action handlers (char input, delete, undo/redo, select, line ops).

use crate::app::focus::Focus;
use crate::app::overlay_event::OverlayEvent;
use crate::app::state::AppResult;
use crate::primitives::text_input::TextInputEvent;
use crate::logic::AppLogic;

macro_rules! rename_edit {
    ($self:expr, backspace) => {{
        if matches!($self.focus, Focus::TabRename) {
            return $self.dispatch_inline(OverlayEvent::Text(TextInputEvent::Backspace { word: false }));
        }
    }};
    ($self:expr, handle_delete) => {{
        if matches!($self.focus, Focus::TabRename) {
            return $self.dispatch_inline(OverlayEvent::Text(TextInputEvent::Delete { word: false }));
        }
    }};
    ($self:expr, handle_char, $ch:expr) => {{
        if matches!($self.focus, Focus::TabRename) {
            return $self.dispatch_inline(OverlayEvent::Text(TextInputEvent::Char($ch)));
        }
    }};
    ($self:expr, $method:ident $(, $arg:expr)*) => {{
        if matches!($self.focus, Focus::TabRename) {
            return AppResult::Redraw;
        }
    }};
}

impl AppLogic {
    pub(crate) fn handle_char(&mut self, ch: char) -> AppResult {
        if self.focus.is_overlay() {
            return self.dispatch_overlay(OverlayEvent::Text(TextInputEvent::Char(ch)));
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
        if self.focus.is_overlay() {
            return self.dispatch_overlay(OverlayEvent::Text(TextInputEvent::Backspace { word: false }));
        }
        rename_edit!(self, handle_backspace);
        self.tabs[self.active_tab].backspace();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_delete(&mut self) -> AppResult {
        if self.focus.is_overlay() {
            return self.dispatch_overlay(OverlayEvent::Text(TextInputEvent::Delete { word: false }));
        }
        rename_edit!(self, handle_delete);
        self.tabs[self.active_tab].delete();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_delete_word_left(&mut self) -> AppResult {
        if self.focus.is_overlay() { return AppResult::Ok; }
        rename_edit!(self, handle_delete_word_left);
        self.tabs[self.active_tab].delete_word_left();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn handle_delete_word_right(&mut self) -> AppResult {
        if self.focus.is_overlay() { return AppResult::Ok; }
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
        if matches!(self.focus, Focus::Overlay | Focus::TabRename) { return AppResult::Ok; }
        if self.tabs[self.active_tab].move_lines_up() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn handle_move_lines_down(&mut self) -> AppResult {
        if matches!(self.focus, Focus::Overlay | Focus::TabRename) { return AppResult::Ok; }
        if self.tabs[self.active_tab].move_lines_down() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn toggle_word_wrap(&mut self) -> AppResult {
        if matches!(self.focus, Focus::Overlay | Focus::TabRename) { return AppResult::Ok; }
        self.tabs[self.active_tab].toggle_word_wrap();
        self.auto_scroll();
        AppResult::Redraw
    }
}
