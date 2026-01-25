//! File operations

use crate::tab::Tab;

use super::state::AppResult;
use super::App;

impl App {
    pub fn save_current(&mut self) -> AppResult {
        self.tabs[self.active_tab].save();
        AppResult::Redraw
    }

    pub fn open_file(&mut self) -> AppResult {
        if let Some(tab) = Tab::open() {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
            self.auto_scroll();
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }

    pub fn rename_current(&mut self) -> AppResult {
        self.start_rename(self.active_tab);
        AppResult::Redraw
    }
}
