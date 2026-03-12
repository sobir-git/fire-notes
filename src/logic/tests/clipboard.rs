//! Paste and cut clipboard tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str};

#[test]
fn insert_paste_text_appends_to_buffer() {
    let mut logic = app();
    logic.insert_paste_text("pasted");
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "pasted");
}

#[test]
fn insert_paste_text_respects_cursor_position() {
    let mut logic = app();
    type_str(&mut logic, "ac");
    logic.execute(Action::CursorLeft { selecting: false });
    logic.insert_paste_text("b");
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "abc");
}

#[test]
fn paste_multiline_text_creates_multiple_lines() {
    let mut logic = app();
    logic.insert_paste_text("line1\nline2");
    let frame = logic.render_frame();
    assert!(frame.visible_lines.len() >= 2);
    assert_eq!(frame.visible_lines[0].text, "line1");
    assert_eq!(frame.visible_lines[1].text, "line2");
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
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines.first().map(|l| l.text.as_str()).unwrap_or(""), "");
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
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "hello");
}
