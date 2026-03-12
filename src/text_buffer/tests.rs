//! Unit tests for TextBuffer.
use super::*;

#[test]
fn test_new_buffer() {
    let buf = TextBuffer::new();
    assert!(buf.is_empty());
    assert_eq!(buf.cursor(), 0);
}

#[test]
fn test_insert_single() {
    let mut buf = TextBuffer::new();
    buf.insert('a');
    assert_eq!(buf.content(), "a");
    assert_eq!(buf.cursor(), 1);
}

#[test]
fn test_insert_multiple() {
    let mut buf = TextBuffer::new();
    for ch in "Hello".chars() { buf.insert(ch); }
    assert_eq!(buf.content(), "Hello");
    assert_eq!(buf.cursor(), 5);
}

#[test]
fn test_insert_str() {
    let mut buf = TextBuffer::new();
    buf.insert_str("Hello World");
    assert_eq!(buf.content(), "Hello World");
    assert_eq!(buf.cursor(), 11);
}

#[test]
fn test_backspace() {
    let mut buf = TextBuffer::from_str("Hello");
    buf.cursor = 5;
    buf.backspace();
    assert_eq!(buf.content(), "Hell");
    buf.backspace();
    assert_eq!(buf.content(), "Hel");
}

#[test]
fn test_backspace_at_start() {
    let mut buf = TextBuffer::from_str("Hello");
    buf.cursor = 0;
    buf.backspace();
    assert_eq!(buf.content(), "Hello");
    assert_eq!(buf.cursor(), 0);
}

#[test]
fn test_delete() {
    let mut buf = TextBuffer::from_str("Hello");
    buf.cursor = 0;
    buf.delete();
    assert_eq!(buf.content(), "ello");
}

#[test]
fn test_move_left_right() {
    let mut buf = TextBuffer::from_str("Hello");
    buf.cursor = 3;
    buf.move_left(false);
    assert_eq!(buf.cursor(), 2);
    buf.move_right(false);
    assert_eq!(buf.cursor(), 3);
}

#[test]
fn test_move_left_at_start() {
    let mut buf = TextBuffer::from_str("Hello");
    buf.cursor = 0;
    buf.move_left(false);
    assert_eq!(buf.cursor(), 0);
}

#[test]
fn test_move_right_at_end() {
    let mut buf = TextBuffer::from_str("Hello");
    buf.cursor = 5;
    buf.move_right(false);
    assert_eq!(buf.cursor(), 5);
}

#[test]
fn test_move_word_jump() {
    let mut buf = TextBuffer::from_str("word1 word2  word3");
    buf.cursor = 0;
    buf.move_word_right(false);
    assert_eq!(buf.cursor(), 5);
    buf.move_word_right(false);
    assert_eq!(buf.cursor(), 11);
    buf.move_word_right(false);
    assert_eq!(buf.cursor(), 18);
    buf.move_word_left(false);
    assert_eq!(buf.cursor(), 13);
    buf.move_word_left(false);
    assert_eq!(buf.cursor(), 6);
    buf.move_word_left(false);
    assert_eq!(buf.cursor(), 0);
}

#[test]
fn test_multiline_navigation() {
    let mut buf = TextBuffer::from_str("Line1\nLine2\nLine3");
    buf.cursor = 7;
    buf.move_up(false);
    assert!(buf.cursor() < 6);
    buf.move_down(false);
    buf.move_down(false);
    let line = buf.rope.char_to_line(buf.cursor());
    assert_eq!(line, 2);
}

#[test]
fn test_from_str() {
    let buf = TextBuffer::from_str("Initial content");
    assert_eq!(buf.content(), "Initial content");
    assert_eq!(buf.len(), 15);
}

#[test]
fn test_large_text() {
    let large_text = "a".repeat(100_000);
    let mut buf = TextBuffer::from_str(&large_text);
    assert_eq!(buf.len(), 100_000);
    buf.cursor = 50_000;
    buf.insert('X');
    assert_eq!(buf.len(), 100_001);
}

#[test]
fn test_selection_basic() {
    let mut buf = TextBuffer::from_str("Hello World");
    buf.cursor = 0;
    buf.move_right(true);
    buf.move_right(true);
    assert!(buf.has_selection());
    assert_eq!(buf.selected_text(), "He");
    buf.move_right(false);
    assert!(!buf.has_selection());
}

#[test]
fn test_move_line_up() {
    let mut buf = TextBuffer::from_str("Line 1\nLine 2\nLine 3");
    buf.set_cursor_by_line_col(1, 0, false);
    buf.move_lines_up();
    assert_eq!(buf.content(), "Line 2\nLine 1\nLine 3\n");
    let (line, _) = buf.char_to_line_col(buf.cursor());
    assert_eq!(line, 0);
}

#[test]
fn test_move_line_down() {
    let mut buf = TextBuffer::from_str("Line 1\nLine 2\nLine 3");
    buf.set_cursor_by_line_col(1, 0, false);
    buf.move_lines_down();
    assert_eq!(buf.content(), "Line 1\nLine 3\nLine 2\n");
    let (line, _) = buf.char_to_line_col(buf.cursor());
    assert_eq!(line, 2);
}

#[test]
fn test_move_line_down_eof_no_newline() {
    let mut buf = TextBuffer::from_str("Line 1\nLine 2");
    buf.cursor = 0;
    buf.move_lines_down();
    assert_eq!(buf.content(), "Line 2\nLine 1\n");
    let (line, _) = buf.char_to_line_col(buf.cursor());
    assert_eq!(line, 1);
}

#[test]
fn test_undo_redo_insert() {
    let mut buf = TextBuffer::new();
    buf.insert('a');
    buf.insert('b');
    assert_eq!(buf.content(), "ab");
    buf.undo();
    assert_eq!(buf.content(), "a");
    buf.undo();
    assert_eq!(buf.content(), "");
    buf.redo();
    assert_eq!(buf.content(), "a");
    buf.redo();
    assert_eq!(buf.content(), "ab");
}

#[test]
fn test_undo_redo_delete() {
    let mut buf = TextBuffer::from_str("abc");
    buf.cursor = 3;
    buf.backspace();
    assert_eq!(buf.content(), "ab");
    buf.undo();
    assert_eq!(buf.content(), "abc");
    buf.redo();
    assert_eq!(buf.content(), "ab");
}

#[test]
fn test_undo_selection_delete() {
    let mut buf = TextBuffer::from_str("hello world");
    buf.cursor = 0;
    for _ in 0..5 { buf.move_right(true); }
    buf.delete_selection();
    assert_eq!(buf.content(), " world");
    buf.undo();
    assert_eq!(buf.content(), "hello world");
    buf.redo();
    assert_eq!(buf.content(), " world");
}
