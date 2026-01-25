//! Tab management operations

use crate::tab::Tab;

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
        println!("start_rename: tab_index={}", tab_index);
        if let Some(tab) = self.tabs.get(tab_index) {
            self.state.renaming_tab = Some(tab_index);
            self.state.rename_buffer = tab.title().to_string();
            println!(
                "renaming_tab set to {:?}, buffer='{}'",
                self.state.renaming_tab, self.state.rename_buffer
            );
        }
    }

    pub fn confirm_rename(&mut self) -> AppResult {
        if let Some(tab_index) = self.state.renaming_tab.take() {
            let title = self.state.rename_buffer.trim();
            if !title.is_empty() {
                if let Some(tab) = self.tabs.get_mut(tab_index) {
                    tab.set_title(title.to_string());
                }
            }
            self.state.rename_buffer.clear();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn cancel_rename(&mut self) -> AppResult {
        if self.state.renaming_tab.take().is_some() {
            self.state.rename_buffer.clear();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}
