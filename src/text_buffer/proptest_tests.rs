//! Property-based fuzz tests for TextBuffer.
use super::*;
use proptest::prelude::*;

fn printable() -> impl Strategy<Value = char> {
    (b' '..=b'~').prop_map(|b| b as char)
}

proptest! {
    #[test]
    fn prop_undo_all_inserts_empties_buffer(
        chars in prop::collection::vec(printable(), 1..50)
    ) {
        let mut buf = TextBuffer::new();
        let n = chars.len();
        for ch in chars { buf.insert(ch); }
        for _ in 0..n { buf.undo(); }
        prop_assert_eq!(buf.content(), "");
    }

    #[test]
    fn prop_len_matches_content(
        chars in prop::collection::vec(printable(), 0..80)
    ) {
        let mut buf = TextBuffer::new();
        for ch in &chars { buf.insert(*ch); }
        prop_assert_eq!(buf.len(), buf.content().chars().count());
    }

    #[test]
    fn prop_cursor_never_exceeds_len(
        chars in prop::collection::vec(printable(), 1..50),
        moves_left in 0usize..30,
        moves_right in 0usize..30,
    ) {
        let mut buf = TextBuffer::new();
        for ch in chars { buf.insert(ch); }
        for _ in 0..moves_left  { buf.move_left(false); }
        for _ in 0..moves_right { buf.move_right(false); }
        prop_assert!(buf.cursor() <= buf.len());
    }

    #[test]
    fn prop_backspace_erases_inserts(s in "[a-zA-Z0-9 ]{1,40}") {
        let n = s.chars().count();
        let mut buf = TextBuffer::new();
        for ch in s.chars() { buf.insert(ch); }
        for _ in 0..n { buf.backspace(); }
        prop_assert_eq!(buf.content(), "");
        prop_assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn prop_undo_redo_roundtrip(
        chars in prop::collection::vec(printable(), 2..30)
    ) {
        let mut buf = TextBuffer::new();
        for ch in &chars { buf.insert(*ch); }
        let full = buf.content().to_string();
        let n = chars.len();
        for _ in 0..n { buf.undo(); }
        for _ in 0..n { buf.redo(); }
        prop_assert_eq!(buf.content(), full.as_str());
    }

    #[test]
    fn prop_select_all_delete_empties(s in "[a-zA-Z0-9 \n]{1,60}") {
        let mut buf = TextBuffer::from_str(&s);
        buf.select_all();
        buf.delete_selection();
        prop_assert_eq!(buf.content(), "");
    }
}
