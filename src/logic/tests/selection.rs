//! Selection geometry tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str};

#[test]
fn selection_absent_without_select() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    let frame = logic.render_frame();
    assert!(frame.selection.is_none());
}

#[test]
fn selection_present_after_shift_move() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    logic.execute(Action::CursorLeft { selecting: true });
    logic.execute(Action::CursorLeft { selecting: true });
    let frame = logic.render_frame();
    let sel = frame.selection.expect("selection should be Some");
    assert_eq!(sel.start, (0, 3));
    assert_eq!(sel.end, (0, 5));
}

#[test]
fn selection_cleared_after_non_selecting_move() {
    let mut logic = app();
    type_str(&mut logic, "hello");
    logic.execute(Action::CursorLeft { selecting: true });
    logic.execute(Action::CursorLeft { selecting: false });
    let frame = logic.render_frame();
    assert!(frame.selection.is_none());
}

#[test]
fn select_all_covers_entire_content() {
    let mut logic = app();
    type_str(&mut logic, "abc");
    logic.execute(Action::SelectAll);
    let frame = logic.render_frame();
    let sel = frame.selection.expect("select-all should produce selection");
    assert_eq!(sel.start, (0, 0));
    assert_eq!(sel.end, (0, 3));
}

#[test]
fn multiline_selection_spans_lines() {
    let mut logic = app();
    type_str(&mut logic, "line1");
    logic.execute(Action::InsertChar('\n'));
    type_str(&mut logic, "line2");
    logic.execute(Action::CursorDocStart { selecting: false });
    logic.execute(Action::CursorDocEnd { selecting: true });
    let frame = logic.render_frame();
    let sel = frame.selection.expect("should have selection spanning lines");
    assert_eq!(sel.start.0, 0);
    assert_eq!(sel.end.0, 1);
}
