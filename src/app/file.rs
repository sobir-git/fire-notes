//! File operations

use crate::tab::Tab;

use super::state::AppResult;
use super::App;

impl App {
    pub fn save_current(&mut self) -> AppResult {
        self.logic.tabs[self.logic.active_tab].save();
        AppResult::Redraw
    }

    pub fn open_file(&mut self) -> AppResult {
        if let Some(tab) = Tab::open() {
            self.logic.tabs.push(tab);
            self.logic.active_tab = self.logic.tabs.len() - 1;
            self.auto_scroll();
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }

    pub fn rename_current(&mut self) -> AppResult {
        self.logic.start_rename(self.logic.active_tab);
        AppResult::Redraw
    }
}
