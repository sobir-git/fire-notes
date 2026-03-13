//! Basic typing, backspace, delete, undo, redo tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str, line};

#[test]
fn type_chars_appear_in_frame() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    assert_eq!(line(&logic, 0), "hello");
}

#[test]
fn backspace_removes_char() {
    let mut logic = app();
    type_str(&mut logic, "abc");
    logic.execute(Action::Backspace);
    assert_eq!(line(&logic, 0), "ab");
}

#[test]
fn delete_removes_char_forward() {
    let mut logic = app();
    type_str(&mut logic, "abc");
    logic.execute(Action::CursorLeft { selecting: false });
    logic.execute(Action::Delete);
    assert_eq!(line(&logic, 0), "ab");
}

#[test]
fn undo_restores_previous_state() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    logic.execute(Action::Undo);
    assert_eq!(line(&logic, 0), "hell");
}

#[test]
fn redo_reapplies_undone_action() {
    let mut logic = app();
    type_str(&mut logic, "hi");
    logic.execute(Action::Undo);
    logic.execute(Action::Redo);
    assert_eq!(line(&logic, 0), "hi");
}

#[test]
fn select_all_then_delete_clears_buffer() {
    let mut logic = app();
    type_str(&mut logic, "clear me");
    logic.execute(Action::SelectAll);
    logic.execute(Action::Backspace);
    assert_eq!(line(&logic, 0), "");
}

#[test]
fn newline_creates_second_visible_line() {
    let mut logic = app();
    type_str(&mut logic, "line1");
    logic.execute(Action::InsertChar('\n'));
    type_str(&mut logic, "line2");
    assert_eq!(logic.tabs[logic.active_tab].total_lines(), 2);
    assert_eq!(line(&logic, 0), "line1");
    assert_eq!(line(&logic, 1), "line2");
}

#[test]
fn word_wrap_toggles() {
    let mut logic = app();
    let before = logic.tabs[logic.active_tab].word_wrap();
    logic.execute(Action::ToggleWordWrap);
    let after = logic.tabs[logic.active_tab].word_wrap();
    assert_ne!(before, after);
}
