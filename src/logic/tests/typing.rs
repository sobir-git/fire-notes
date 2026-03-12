//! Basic typing, backspace, delete, undo, redo tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str};

#[test]
fn type_chars_appear_in_frame() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "hello");
}

#[test]
fn backspace_removes_char() {
    let mut logic = app();
    type_str(&mut logic, "abc");
    logic.execute(Action::Backspace);
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "ab");
}

#[test]
fn delete_removes_char_forward() {
    let mut logic = app();
    type_str(&mut logic, "abc");
    logic.execute(Action::CursorLeft { selecting: false });
    logic.execute(Action::Delete);
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "ab");
}

#[test]
fn undo_restores_previous_state() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    logic.execute(Action::Undo);
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "hell");
}

#[test]
fn redo_reapplies_undone_action() {
    let mut logic = app();
    type_str(&mut logic, "hi");
    logic.execute(Action::Undo);
    logic.execute(Action::Redo);
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "hi");
}

#[test]
fn select_all_then_delete_clears_buffer() {
    let mut logic = app();
    type_str(&mut logic, "clear me");
    logic.execute(Action::SelectAll);
    logic.execute(Action::Backspace);
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines.first().map(|l| l.text.as_str()).unwrap_or(""), "");
}

#[test]
fn newline_creates_second_visible_line() {
    let mut logic = app();
    type_str(&mut logic, "line1");
    logic.execute(Action::InsertChar('\n'));
    type_str(&mut logic, "line2");
    let frame = logic.render_frame();
    assert!(frame.visible_lines.len() >= 2);
    assert_eq!(frame.visible_lines[0].text, "line1");
    assert_eq!(frame.visible_lines[1].text, "line2");
}

#[test]
fn word_wrap_toggles() {
    let mut logic = app();
    let before = logic.render_frame().word_wrap;
    logic.execute(Action::ToggleWordWrap);
    let after = logic.render_frame().word_wrap;
    assert_ne!(before, after);
}
