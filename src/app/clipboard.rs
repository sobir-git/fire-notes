//! Clipboard operations

use super::state::AppResult;
use super::App;

impl App {
    pub fn handle_copy(&mut self) -> AppResult {
        if let Some(text) = self.tabs[self.active_tab].copy_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
        AppResult::Ok
    }

    pub fn handle_cut(&mut self) -> AppResult {
        if let Some(text) = self.tabs[self.active_tab].cut_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
            self.tabs[self.active_tab].auto_save();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_paste(&mut self) -> AppResult {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                self.tabs[self.active_tab].paste_text(&text);
                self.tabs[self.active_tab].auto_save();
                self.auto_scroll();
                return AppResult::Redraw;
            }
        }
        AppResult::Ok
    }
}
