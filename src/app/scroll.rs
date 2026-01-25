//! Scrolling operations

use super::state::AppResult;
use super::App;

impl App {

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
