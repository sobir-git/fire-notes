#![allow(dead_code)]
//! Tab management — App shims delegate to AppLogic.

use super::state::AppResult;
use super::App;

impl App {
    pub fn new_tab(&mut self) -> AppResult { self.logic.new_tab() }
    pub fn close_current_tab(&mut self) -> AppResult { self.logic.close_current_tab() }
    pub fn next_tab(&mut self) -> AppResult { self.logic.next_tab() }
    pub fn previous_tab(&mut self) -> AppResult { self.logic.previous_tab() }
    pub fn go_to_tab(&mut self, index: usize) -> AppResult { self.logic.go_to_tab(index) }
    pub fn start_rename(&mut self, tab_index: usize) { self.logic.start_rename(tab_index); }
    pub fn confirm_rename(&mut self) -> AppResult { self.logic.confirm_rename() }
    pub fn cancel_rename(&mut self) -> AppResult { self.logic.cancel_rename() }
}
