//! Cursor movement tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str, cursor_pos};

#[test]
fn cursor_moves_left_and_right() {
    let mut logic = app();
    type_str(&mut logic, "abc");
    logic.execute(Action::CursorLeft { selecting: false });
    assert_eq!(cursor_pos(&logic).1, 2);
    logic.execute(Action::CursorRight { selecting: false });
    assert_eq!(cursor_pos(&logic).1, 3);
}

#[test]
fn cursor_moves_to_line_start_end() {
    let mut logic = app();
    type_str(&mut logic, "hello world");
    logic.execute(Action::CursorLineStart { selecting: false });
    assert_eq!(cursor_pos(&logic).1, 0);
    logic.execute(Action::CursorLineEnd { selecting: false });
    assert_eq!(cursor_pos(&logic).1, 11);
}

#[test]
fn cursor_word_jump() {
    let mut logic = app();
    type_str(&mut logic, "hello world");
    logic.execute(Action::CursorDocStart { selecting: false });
    logic.execute(Action::CursorWordRight { selecting: false });
    assert_eq!(cursor_pos(&logic).1, 5);
}

#[test]
fn cursor_up_down_across_lines() {
    let mut logic = app();
    type_str(&mut logic, "line1");
    logic.execute(Action::InsertChar('\n'));
    type_str(&mut logic, "line2");
    logic.execute(Action::CursorUp { selecting: false });
    assert_eq!(cursor_pos(&logic).0, 0);
    logic.execute(Action::CursorDown { selecting: false });
    assert_eq!(cursor_pos(&logic).0, 1);
}
