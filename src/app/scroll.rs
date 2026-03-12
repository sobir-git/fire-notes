//! Scrolling operations — App shims delegate to AppLogic.

use super::state::AppResult;
use super::App;

impl App {
    pub fn page_up(&mut self, selecting: bool) -> AppResult { self.logic.page_up(selecting) }
    pub fn page_down(&mut self, selecting: bool) -> AppResult { self.logic.page_down(selecting) }
}
