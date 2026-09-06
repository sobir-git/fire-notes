//! Headless AppLogic integration tests — no GPU or platform deps.

mod typing;
mod tabs;
mod cursor;
mod selection;
mod scroll;
mod clipboard;
mod frame;

use crate::logic::AppLogic;
use crate::app::action::Action;

pub(super) fn app() -> AppLogic {
    AppLogic::new_headless(800.0, 600.0, 1.0)
}

pub(super) fn type_str(logic: &mut AppLogic, s: &str) {
    for ch in s.chars() {
        let _ = logic.execute(Action::InsertChar(ch));
    }
}

/// Get the text of line `n` in the active tab.
pub(super) fn line(logic: &AppLogic, n: usize) -> String {
    logic.tabs[logic.active_tab].content().lines()
        .nth(n).unwrap_or("").to_string()
}

/// Get (line, col) of the cursor in the active tab.
pub(super) fn cursor_pos(logic: &AppLogic) -> (usize, usize) {
    let t = &logic.tabs[logic.active_tab];
    (t.cursor_line(), t.cursor_col())
}

/// Get the selection as ((start_line, start_col), (end_line, end_col)) if any.
pub(super) fn selection(logic: &AppLogic) -> Option<((usize, usize), (usize, usize))> {
    logic.tabs[logic.active_tab].selection_range_line_col()
}

/// Total line count in active tab.
pub(super) fn total_lines(logic: &AppLogic) -> usize {
    logic.tabs[logic.active_tab].total_lines()
}

/// Scroll offset of active tab.
pub(super) fn scroll_offset(logic: &AppLogic) -> usize {
    logic.tabs[logic.active_tab].scroll_offset()
}
