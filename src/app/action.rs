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
    /// Execute an action and return whether a redraw is needed
    pub fn execute(&mut self, action: Action) -> AppResult {
        match action {
            // Tab operations
            Action::NewTab => self.new_tab(),
            Action::CloseTab => self.close_current_tab(),
            Action::NextTab => self.next_tab(),
            Action::PreviousTab => self.previous_tab(),
            Action::GoToTab(index) => self.go_to_tab(index),

            // File operations
            Action::Save => self.save_current(),
            Action::OpenFile => self.open_file(),
            Action::RenameTab => self.rename_current(),

            // Notes picker
            Action::OpenNotesPicker => self.open_notes_picker(),
            Action::ConfirmNotesPicker => self.confirm_notes_picker(),
            Action::CancelNotesPicker => self.cancel_notes_picker(),

            // Edit operations
            Action::Undo => self.handle_undo(),
            Action::Redo => self.handle_redo(),
            Action::Copy => self.handle_copy(),
            Action::Cut => self.handle_cut(),
            Action::Paste => self.handle_paste(),
            Action::SelectAll => self.handle_select_all(),
            Action::DeleteWordLeft => self.handle_delete_word_left(),
            Action::DeleteWordRight => self.handle_delete_word_right(),
            Action::Delete => self.handle_delete(),
            Action::Backspace => self.handle_backspace(),

            // Cursor movement
            Action::CursorLeft { selecting } => self.move_cursor_left(selecting),
            Action::CursorRight { selecting } => self.move_cursor_right(selecting),
            Action::CursorUp { selecting } => self.move_cursor_up(selecting),
            Action::CursorDown { selecting } => self.move_cursor_down(selecting),
            Action::CursorWordLeft { selecting } => self.move_cursor_word_left(selecting),
            Action::CursorWordRight { selecting } => self.move_cursor_word_right(selecting),
            Action::CursorLineStart { selecting } => self.move_cursor_to_line_start(selecting),
            Action::CursorLineEnd { selecting } => self.move_cursor_to_line_end(selecting),
            Action::CursorDocStart { selecting } => self.move_cursor_to_start(selecting),
            Action::CursorDocEnd { selecting } => self.move_cursor_to_end(selecting),
            Action::PageUp { selecting } => self.page_up(selecting),
            Action::PageDown { selecting } => self.page_down(selecting),

            // Line operations
            Action::MoveLinesUp => self.handle_move_lines_up(),
            Action::MoveLinesDown => self.handle_move_lines_down(),

            // View
            Action::ToggleWordWrap => self.toggle_word_wrap(),

            // Modal operations
            Action::Cancel => {
                // Try canceling in order: notes picker, then rename
                let result = self.cancel_notes_picker();
                if result.needs_redraw() {
                    return result;
                }
                self.cancel_rename()
            }
            Action::Confirm => {
                // Try confirming in order: notes picker, rename, then insert newline
                let result = self.confirm_notes_picker();
                if result.needs_redraw() {
                    return result;
                }
                let result = self.confirm_rename();
                if result.needs_redraw() {
                    return result;
                }
                self.handle_char('\n')
            }

            // Character input
            Action::InsertChar(ch) => self.handle_char(ch),
        }
    }
}
