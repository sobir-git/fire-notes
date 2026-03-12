#![allow(dead_code)]
//! Tab management and rename/notes-picker actions.

use crate::app::focus::Focus;
use crate::app::state::AppResult;
use crate::tab::Tab;
use crate::logic::AppLogic;

impl AppLogic {
    pub(crate) fn new_tab(&mut self) -> AppResult {
        self.tabs.push(Tab::new_untitled());
        self.active_tab = self.tabs.len() - 1;
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn close_current_tab(&mut self) -> AppResult {
        if self.tabs.len() <= 1 { return AppResult::Ok; }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn next_tab(&mut self) -> AppResult {
        if self.tabs.is_empty() { return AppResult::Ok; }
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn previous_tab(&mut self) -> AppResult {
        if self.tabs.is_empty() { return AppResult::Ok; }
        self.active_tab = if self.active_tab == 0 { self.tabs.len() - 1 } else { self.active_tab - 1 };
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn go_to_tab(&mut self, index: usize) -> AppResult {
        if index >= self.tabs.len() { return AppResult::Ok; }
        self.active_tab = index;
        self.auto_scroll();
        AppResult::Redraw
    }

    pub(crate) fn rename_current(&mut self) -> AppResult {
        self.start_rename(self.active_tab);
        AppResult::Redraw
    }

    pub(crate) fn start_rename(&mut self, tab_index: usize) {
        if let Some(tab) = self.tabs.get(tab_index) {
            self.focus = Focus::start_rename(tab_index, tab.title());
        }
    }

    pub(crate) fn confirm_rename(&mut self) -> AppResult {
        if let Some((tab_index, title)) = self.focus.confirm_rename() {
            if let Some(tab) = self.tabs.get_mut(tab_index) {
                tab.set_title(title);
            }
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub(crate) fn cancel_rename(&mut self) -> AppResult {
        if self.focus.cancel_rename() { return AppResult::Redraw; }
        AppResult::Ok
    }

    pub(crate) fn confirm_notes_picker(&mut self) -> AppResult {
        if let Some(path) = self.focus.confirm_notes_picker() {
            for (i, tab) in self.tabs.iter().enumerate() {
                if tab.path() == Some(&path) {
                    self.active_tab = i;
                    self.auto_scroll();
                    return AppResult::Redraw;
                }
            }
            if let Some(tab) = Tab::from_file(path) {
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
                self.auto_scroll();
                return AppResult::Redraw;
            }
        }
        AppResult::Ok
    }

    pub(crate) fn cancel_notes_picker(&mut self) -> AppResult {
        if self.focus.cancel_notes_picker() { return AppResult::Redraw; }
        AppResult::Ok
    }
}
