//! Scrolling operations

use crate::config::layout;

use super::state::AppResult;
use super::App;

impl App {
    pub fn scroll_up(&mut self) -> AppResult {
        if self.state.last_mouse_y < layout::TAB_HEIGHT * self.scale {
            self.state.tab_scroll_x =
                (self.state.tab_scroll_x - layout::TAB_PADDING * 2.0 * self.scale).max(0.0);
            self.renderer.set_tab_scroll_x(self.state.tab_scroll_x);
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].scroll_up(crate::config::scroll::LINES_PER_WHEEL_TICK);
        AppResult::Redraw
    }

    pub fn scroll_down(&mut self) -> AppResult {
        if self.state.last_mouse_y < layout::TAB_HEIGHT * self.scale {
            let max_scroll = 1000.0; // TODO: Calculate based on tabs width
            self.state.tab_scroll_x =
                (self.state.tab_scroll_x + layout::TAB_PADDING * 2.0 * self.scale).min(max_scroll);
            self.renderer.set_tab_scroll_x(self.state.tab_scroll_x);
            return AppResult::Redraw;
        }
        let visible = self.visible_lines();
        self.tabs[self.active_tab]
            .scroll_down(crate::config::scroll::LINES_PER_WHEEL_TICK, visible);
        AppResult::Redraw
    }

    /// Page Up: scroll and move cursor up by a full page
    pub fn page_up(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        let page_size = self.visible_lines().saturating_sub(1).max(1);
        
        // Move cursor up by page size
        for _ in 0..page_size {
            self.tabs[self.active_tab].move_up(selecting);
        }
        
        self.auto_scroll();
        AppResult::Redraw
    }

    /// Page Down: scroll and move cursor down by a full page
    pub fn page_down(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        let page_size = self.visible_lines().saturating_sub(1).max(1);
        
        // Move cursor down by page size
        for _ in 0..page_size {
            self.tabs[self.active_tab].move_down(selecting);
        }
        
        self.auto_scroll();
        AppResult::Redraw
    }
}
