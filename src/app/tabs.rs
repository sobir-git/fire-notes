//! Tab management operations

use crate::tab::Tab;

use super::focus::Focus;
use super::state::AppResult;
use super::App;

impl App {
    pub fn new_tab(&mut self) -> AppResult {
        self.tabs.push(Tab::new_untitled());
        self.active_tab = self.tabs.len() - 1;
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn close_current_tab(&mut self) -> AppResult {
        if self.tabs.len() <= 1 {
            return AppResult::Ok;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn next_tab(&mut self) -> AppResult {
        if self.tabs.is_empty() {
            return AppResult::Ok;
        }
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn previous_tab(&mut self) -> AppResult {
        if self.tabs.is_empty() {
            return AppResult::Ok;
        }
        if self.active_tab == 0 {
            self.active_tab = self.tabs.len() - 1;
        } else {
            self.active_tab -= 1;
        }
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn go_to_tab(&mut self, index: usize) -> AppResult {
        if index >= self.tabs.len() {
            return AppResult::Ok;
        }
        self.active_tab = index;
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn start_rename(&mut self, tab_index: usize) {
        if let Some(tab) = self.tabs.get(tab_index) {
            self.focus = Focus::start_rename(tab_index, tab.title());
        }
    }

    pub fn confirm_rename(&mut self) -> AppResult {
        if let Some((tab_index, title)) = self.focus.confirm_rename() {
            if let Some(tab) = self.tabs.get_mut(tab_index) {
                tab.set_title(title);
            }
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn cancel_rename(&mut self) -> AppResult {
        if self.focus.cancel_rename() {
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}
