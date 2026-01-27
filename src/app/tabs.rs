//! Tab management operations

use crate::tab::Tab;
use crate::ui::TextInput;

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
            let mut input = TextInput::new(tab.title().to_string());
            input.select_all();
            self.state.rename_input = Some(input);
            println!(
                "renaming_tab set to {:?}",
                self.state.renaming_tab
            );
        }
    }

    pub fn confirm_rename(&mut self) -> AppResult {
        if let Some(tab_index) = self.state.renaming_tab.take() {
            if let Some(input) = self.state.rename_input.take() {
                let title = input.text().trim();
                if !title.is_empty() {
                    if let Some(tab) = self.tabs.get_mut(tab_index) {
                        tab.set_title(title.to_string());
                    }
                }
            }
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn cancel_rename(&mut self) -> AppResult {
        if self.state.renaming_tab.take().is_some() {
            self.state.rename_input = None;
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}
