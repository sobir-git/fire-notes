#![allow(dead_code)]
//! Centralized action system
//!
//! All user-triggerable actions are defined here. This provides:
//! - Single source of truth for all app actions
//! - Easy to add new actions (one enum variant + one match arm)
//! - Foundation for command palette and keybinding customization
//!
//! Adding a new action:
//! 1. Add variant to Action enum
//! 2. Add handler in App::execute()
//! 3. Optionally add keybinding in keybindings.rs

use super::state::AppResult;
use super::App;

/// All actions that can be triggered by keyboard shortcuts or UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    // =========================================================================
    // Tab operations
    // =========================================================================
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    GoToTab(usize),

    // =========================================================================
    // File operations
    // =========================================================================
    Save,
    OpenFile,
    RenameTab,

    // =========================================================================
    // Notes picker
    // =========================================================================
    OpenNotesPicker,
    ConfirmNotesPicker,
    CancelNotesPicker,

    // =========================================================================
    // Edit operations
    // =========================================================================
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    SelectAll,
    DeleteWordLeft,
    DeleteWordRight,
    Delete,
    Backspace,

    // =========================================================================
    // Cursor movement
    // =========================================================================
    CursorLeft { selecting: bool },
    CursorRight { selecting: bool },
    CursorUp { selecting: bool },
    CursorDown { selecting: bool },
    CursorWordLeft { selecting: bool },
    CursorWordRight { selecting: bool },
    CursorLineStart { selecting: bool },
    CursorLineEnd { selecting: bool },
    CursorDocStart { selecting: bool },
    CursorDocEnd { selecting: bool },
    PageUp { selecting: bool },
    PageDown { selecting: bool },

    // =========================================================================
    // Line operations
    // =========================================================================
    MoveLinesUp,
    MoveLinesDown,

    // =========================================================================
    // View
    // =========================================================================
    ToggleWordWrap,

    // =========================================================================
    // Modal/Focus operations
    // =========================================================================
    Cancel,  // Escape - cancels current modal/rename
    Confirm, // Enter - confirms current modal/rename or inserts newline

    // =========================================================================
    // Character input
    // =========================================================================
    InsertChar(char),
}

impl App {
    /// Execute an action and return whether a redraw is needed.
    ///
    /// Most actions delegate directly to `AppLogic::execute`.
    /// The three clipboard actions (Copy/Cut/Paste) are handled here because
    /// they need the OS clipboard which `AppLogic` cannot access.
    /// `OpenNotesPicker` is handled here because it reads from persistence.
    pub fn execute(&mut self, action: Action) -> AppResult {
        match action {
            // Clipboard — App layer owns the OS clipboard.
            Action::Copy => {
                if let Some(text) = self.logic.copy_selection() {
                    if let Some(cb) = &mut self.clipboard { let _ = cb.set_text(text); }
                }
                AppResult::Ok
            }
            Action::Cut => {
                if let Some(text) = self.logic.cut_selection() {
                    if let Some(cb) = &mut self.clipboard { let _ = cb.set_text(text); }
                    self.logic.auto_scroll();
                    return AppResult::Redraw;
                }
                AppResult::Ok
            }
            Action::Paste => {
                if let Some(cb) = &mut self.clipboard {
                    if let Ok(text) = cb.get_text() {
                        if self.logic.insert_paste_text(&text).needs_redraw() {
                            return AppResult::Redraw;
                        }
                    }
                }
                AppResult::Ok
            }

            // OpenNotesPicker — needs persistence.
            Action::OpenNotesPicker => self.open_notes_picker(),

            // File ops — need filesystem/dialog access.
            Action::Save => {
                self.logic.tabs[self.logic.active_tab].save();
                AppResult::Redraw
            }
            Action::OpenFile => {
                use crate::tab::Tab;
                if let Some(tab) = Tab::open() {
                    self.logic.tabs.push(tab);
                    self.logic.activate_tab(self.logic.tabs.len() - 1);
                    AppResult::Redraw
                } else {
                    AppResult::Ok
                }
            }
            Action::RenameTab => {
                self.logic.start_rename(self.logic.active_tab);
                AppResult::Redraw
            }

            // Everything else — pure logic, delegate directly.
            other => self.logic.execute(other),
        }
    }
}
