//! Paste and cut clipboard tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str, line, total_lines};

#[test]
fn insert_paste_text_appends_to_buffer() {
    let mut logic = app();
    logic.insert_paste_text("pasted");
    assert_eq!(line(&logic, 0), "pasted");
}

#[test]
fn insert_paste_text_respects_cursor_position() {
    let mut logic = app();
    type_str(&mut logic, "ac");
    logic.execute(Action::CursorLeft { selecting: false });
    logic.insert_paste_text("b");
    assert_eq!(line(&logic, 0), "abc");
}

#[test]
fn paste_multiline_text_creates_multiple_lines() {
    let mut logic = app();
    logic.insert_paste_text("line1\nline2");
    assert!(total_lines(&logic) >= 2);
    assert_eq!(line(&logic, 0), "line1");
    assert_eq!(line(&logic, 1), "line2");
}

#[test]
fn cut_selection_returns_selected_text() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    logic.execute(Action::SelectAll);
    let text = logic.cut_selection();
    assert_eq!(text.as_deref(), Some("hello"));
}

#[test]
fn cut_selection_clears_buffer() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    logic.execute(Action::SelectAll);
    logic.cut_selection();
    assert_eq!(line(&logic, 0), "");
}

#[test]
fn cut_without_selection_returns_none() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    let text = logic.cut_selection();
    assert!(text.is_none());
}

#[test]
fn copy_selection_does_not_modify_buffer() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    logic.execute(Action::SelectAll);
    let text = logic.copy_selection();
    assert_eq!(text.as_deref(), Some("hello"));
    assert_eq!(line(&logic, 0), "hello");
}
