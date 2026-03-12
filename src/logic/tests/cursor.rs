//! Cursor movement tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str};

#[test]
fn cursor_moves_left_and_right() {
    let mut logic = app();
    type_str(&mut logic, "abc");
    logic.execute(Action::CursorLeft { selecting: false });
    let frame = logic.render_frame();
    assert_eq!(frame.cursor.col, 2);
    logic.execute(Action::CursorRight { selecting: false });
    let frame2 = logic.render_frame();
    assert_eq!(frame2.cursor.col, 3);
}

#[test]
fn cursor_moves_to_line_start_end() {
    let mut logic = app();
    type_str(&mut logic, "hello world");
    logic.execute(Action::CursorLineStart { selecting: false });
    let frame = logic.render_frame();
    assert_eq!(frame.cursor.col, 0);
    logic.execute(Action::CursorLineEnd { selecting: false });
    let frame2 = logic.render_frame();
    assert_eq!(frame2.cursor.col, 11);
}

#[test]
fn cursor_word_jump() {
    let mut logic = app();
    type_str(&mut logic, "hello world");
    logic.execute(Action::CursorDocStart { selecting: false });
    logic.execute(Action::CursorWordRight { selecting: false });
    let frame = logic.render_frame();
    assert_eq!(frame.cursor.col, 5);
}

#[test]
fn cursor_up_down_across_lines() {
    let mut logic = app();
    type_str(&mut logic, "line1");
    logic.execute(Action::InsertChar('\n'));
    type_str(&mut logic, "line2");
    logic.execute(Action::CursorUp { selecting: false });
    let frame = logic.render_frame();
    assert_eq!(frame.cursor.line, 0);
    logic.execute(Action::CursorDown { selecting: false });
    let frame2 = logic.render_frame();
    assert_eq!(frame2.cursor.line, 1);
}
