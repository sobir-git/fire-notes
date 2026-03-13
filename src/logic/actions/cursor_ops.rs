//! Cursor movement action handlers.

use crate::app::focus::Focus;
use crate::app::state::AppResult;
use crate::logic::AppLogic;

/// Helper: while TabRename is active, silently consume cursor movement
/// (left/right are dispatched explicitly below; up/down/etc. are consumed).
macro_rules! rename_delegate {
    ($self:expr) => {{
        if matches!($self.focus, Focus::TabRename) {
            return AppResult::Ok;
        }
    }};
}

impl AppLogic {
    pub(crate) fn move_cursor_left(&mut self, selecting: bool) -> AppResult {
        if matches!(self.focus, Focus::TabRename) {
            return self.dispatch_inline(crate::layout::FrameworkEvent::ArrowLeft);
        }
        if self.focus.is_overlay() {
            return self.dispatch_overlay(crate::layout::FrameworkEvent::ArrowLeft);
        }
        self.tabs[self.active_tab].move_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_right(&mut self, selecting: bool) -> AppResult {
        if matches!(self.focus, Focus::TabRename) {
            return self.dispatch_inline(crate::layout::FrameworkEvent::ArrowRight);
        }
        if self.focus.is_overlay() {
            return self.dispatch_overlay(crate::layout::FrameworkEvent::ArrowRight);
        }
        self.tabs[self.active_tab].move_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_up(&mut self, selecting: bool) -> AppResult {
        if matches!(self.focus, Focus::TabRename) { return AppResult::Ok; }
        if self.focus.is_overlay() {
            return self.dispatch_overlay(crate::layout::FrameworkEvent::ArrowUp);
        }
        self.tabs[self.active_tab].move_up(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_down(&mut self, selecting: bool) -> AppResult {
        if matches!(self.focus, Focus::TabRename) { return AppResult::Ok; }
        if self.focus.is_overlay() {
            return self.dispatch_overlay(crate::layout::FrameworkEvent::ArrowDown);
        }
        self.tabs[self.active_tab].move_down(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_word_left(&mut self, selecting: bool) -> AppResult {
        rename_delegate!(self);
        self.tabs[self.active_tab].move_word_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_word_right(&mut self, selecting: bool) -> AppResult {
        rename_delegate!(self);
        self.tabs[self.active_tab].move_word_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_to_line_start(&mut self, selecting: bool) -> AppResult {
        if matches!(self.focus, Focus::TabRename) {
            return self.dispatch_inline(crate::layout::FrameworkEvent::Key(crate::app::Key::Home));
        }
        self.tabs[self.active_tab].move_to_line_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_to_line_end(&mut self, selecting: bool) -> AppResult {
        if matches!(self.focus, Focus::TabRename) {
            return self.dispatch_inline(crate::layout::FrameworkEvent::Key(crate::app::Key::End));
        }
        self.tabs[self.active_tab].move_to_line_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_to_start(&mut self, selecting: bool) -> AppResult {
        rename_delegate!(self);
        self.tabs[self.active_tab].move_to_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn move_cursor_to_end(&mut self, selecting: bool) -> AppResult {
        rename_delegate!(self);
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

