//! Tab management and rename/notes-picker actions.

use crate::app::active_overlay::ActiveInline;
use crate::app::focus::Focus;
use crate::app::state::AppResult;
use crate::tab::Tab;
use crate::logic::AppLogic;

impl AppLogic {
    /// Single point of truth for switching the active tab.
    /// Sets `active_tab` and scrolls the tab bar so the tab is fully visible.
    pub(crate) fn activate_tab(&mut self, index: usize) {
        self.active_tab = index;
        self.auto_scroll();
        let tab_titles: Vec<(String, bool)> = self.tabs.iter().enumerate()
            .map(|(i, t)| (t.title().to_string(), i == self.active_tab))
            .collect();
        let owned: Vec<(&str, bool)> = tab_titles.iter().map(|(s, b)| (s.as_str(), *b)).collect();
        self.ui_tree.relayout_tabs(self.tab_scroll_x, &owned);
        self.tab_scroll_x =
            self.ui_tree.tab_bar.scroll_x_to_reveal(index, self.tab_scroll_x);
    }

    pub(crate) fn new_tab(&mut self) -> AppResult {
        self.tabs.push(Tab::new_untitled());
        self.activate_tab(self.tabs.len() - 1);
        AppResult::Redraw
    }

    pub(crate) fn close_current_tab(&mut self) -> AppResult {
        if self.tabs.len() <= 1 { return AppResult::Ok; }
        self.tabs.remove(self.active_tab);
        let new_index = self.active_tab.min(self.tabs.len() - 1);
        self.activate_tab(new_index);
        AppResult::Redraw
    }

    pub(crate) fn next_tab(&mut self) -> AppResult {
        if self.tabs.is_empty() { return AppResult::Ok; }
        let next = (self.active_tab + 1) % self.tabs.len();
        self.activate_tab(next);
        AppResult::Redraw
    }

    pub(crate) fn previous_tab(&mut self) -> AppResult {
        if self.tabs.is_empty() { return AppResult::Ok; }
        let prev = if self.active_tab == 0 { self.tabs.len() - 1 } else { self.active_tab - 1 };
        self.activate_tab(prev);
        AppResult::Redraw
    }

    pub(crate) fn go_to_tab(&mut self, index: usize) -> AppResult {
        if index >= self.tabs.len() { return AppResult::Ok; }
        self.activate_tab(index);
        AppResult::Redraw
    }

    pub(crate) fn rename_current(&mut self) -> AppResult {
        self.start_rename(self.active_tab);
        AppResult::Redraw
    }

    pub(crate) fn start_rename(&mut self, tab_index: usize) {
        if let Some(tab) = self.tabs.get(tab_index) {
            use crate::components::tab_rename::TabRename;
            use crate::ui::Rect;
            let rect = Rect::ZERO; // renderer will position it; geometry not needed at logic layer
            let widget = TabRename::new(tab_index, tab.title(), rect, self.scale);
            self.inline = Some(ActiveInline::TabRename(widget));
            self.focus = Focus::TabRename;
        }
    }

    pub(crate) fn confirm_rename(&mut self) -> AppResult {
        if !matches!(self.focus, Focus::TabRename) { return AppResult::Ok; }
        self.focus = Focus::Editor;
        self.inline = None;
        AppResult::Redraw
    }

    pub(crate) fn cancel_rename(&mut self) -> AppResult {
        if self.focus.cancel_rename() { return AppResult::Redraw; }
        AppResult::Ok
    }

    /// Switch to an already-open tab for `path`, or open it as a new tab.
    pub(crate) fn open_or_switch_to(&mut self, path: std::path::PathBuf) -> AppResult {
        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.path() == Some(&path) {
                self.activate_tab(i);
                return AppResult::Redraw;
            }
        }
        if let Some(tab) = Tab::from_file(path) {
            self.tabs.push(tab);
            self.activate_tab(self.tabs.len() - 1);
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}
