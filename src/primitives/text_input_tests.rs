//! QA-level tests for `primitives::TextInput`.
//!
//! Coverage:
//! - Insert / backspace / delete
//! - Cursor movement (left, right, word, home, end)
//! - Selection: extend, collapse, delete-over-selection
//! - Scroll: cursor visibility after movement
//! - Unicode: multi-byte chars navigate by grapheme, not byte
//! - Edge cases: empty input, cursor at boundaries, insert-replaces-selection

#[cfg(test)]
mod tests {
    use crate::primitives::TextInput;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn ti(text: &str) -> TextInput {
        let mut t = TextInput::new(text.to_string());
        // place cursor at start for predictable baselines
        t.state.move_to_start(false);
        t
    }

    fn ti_at_end(text: &str) -> TextInput {
        TextInput::new(text.to_string())
        // TextInput::new places cursor at end by default
    }

    // ── insert ────────────────────────────────────────────────────────────────

    #[test]
    fn insert_into_empty() {
        let mut t = ti("");
        t.insert('a');
        assert_eq!(t.text(), "a");
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn insert_at_start_pushes_text_right() {
        let mut t = ti("bc");
        t.insert('a');
        assert_eq!(t.text(), "abc");
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn insert_at_end_appends() {
        let mut t = ti_at_end("ab");
        t.insert('c');
        assert_eq!(t.text(), "abc");
        assert_eq!(t.cursor(), 3);
    }

    #[test]
    fn insert_replaces_selection() {
        let mut t = ti("hello");
        t.state.select_all();                 // anchor=0, cursor=5
        t.insert('X');
        assert_eq!(t.text(), "X");
        assert_eq!(t.cursor(), 1);
        assert!(t.state.selection_anchor.is_none());
    }

    // ── backspace ─────────────────────────────────────────────────────────────

    #[test]
    fn backspace_removes_previous_char() {
        let mut t = ti_at_end("abc");
        t.backspace();
        assert_eq!(t.text(), "ab");
        assert_eq!(t.cursor(), 2);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut t = ti("abc");
        t.backspace();
        assert_eq!(t.text(), "abc");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn backspace_deletes_selection() {
        let mut t = ti("hello world");
        t.state.move_to_end(false);
        t.state.move_word_left(true); // select "world"
        t.backspace();
        assert_eq!(t.text(), "hello ");
    }

    // ── move_left / move_right ────────────────────────────────────────────────

    #[test]
    fn move_left_decrements_cursor() {
        let mut t = ti_at_end("abc");
        t.move_left(false);
        assert_eq!(t.cursor(), 2);
    }

    #[test]
    fn move_right_increments_cursor() {
        let mut t = ti("abc");
        t.move_right(false);
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn move_left_at_start_is_noop() {
        let mut t = ti("abc");
        t.move_left(false);
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn move_right_at_end_is_noop() {
        let mut t = ti_at_end("abc");
        t.move_right(false);
        assert_eq!(t.cursor(), 3);
    }

    #[test]
    fn move_left_collapses_selection_to_start() {
        let mut t = ti("hello");
        t.state.select_all(); // anchor=0, cursor=5
        t.move_left(false);
        assert_eq!(t.cursor(), 0);
        assert!(t.state.selection_anchor.is_none());
    }

    #[test]
    fn move_right_collapses_selection_to_end() {
        let mut t = ti("hello");
        t.state.select_all(); // anchor=0, cursor=5
        t.move_right(false);
        assert_eq!(t.cursor(), 5);
        assert!(t.state.selection_anchor.is_none());
    }

    // ── home / end ────────────────────────────────────────────────────────────

    #[test]
    fn move_to_start_goes_to_zero() {
        let mut t = ti_at_end("abc");
        t.move_to_start(false);
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn move_to_end_goes_to_text_len() {
        let mut t = ti("abc");
        t.move_to_end(false);
        assert_eq!(t.cursor(), 3);
    }

    #[test]
    fn move_to_start_on_empty_is_noop() {
        let mut t = ti("");
        t.move_to_start(false);
        assert_eq!(t.cursor(), 0);
    }

    // ── word movement ─────────────────────────────────────────────────────────

    #[test]
    fn move_word_left_skips_over_word() {
        let mut t = ti_at_end("hello world");
        t.move_word_left(false);
        assert_eq!(&t.text()[t.cursor()..], "world");
    }

    #[test]
    fn move_word_right_stops_at_end_of_word() {
        let mut t = ti("hello world");
        t.move_word_right(false);
        // Should stop at end of "hello", before the space
        assert_eq!(&t.text()[..t.cursor()], "hello");
    }

    #[test]
    fn move_word_right_from_whitespace_stops_at_end_of_next_word() {
        let mut t = ti("hello world");
        // advance past "hello" to the space
        t.move_word_right(false); // -> end of "hello"
        t.move_word_right(false); // -> skip space, end of "world"
        assert_eq!(t.cursor(), t.text().len());
    }

    #[test]
    fn move_word_left_at_start_is_noop() {
        let mut t = ti("hello");
        t.move_word_left(false);
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn move_word_right_at_end_is_noop() {
        let mut t = ti_at_end("hello");
        t.move_word_right(false);
        assert_eq!(t.cursor(), 5);
    }

    // ── selection extend ─────────────────────────────────────────────────────

    #[test]
    fn move_right_with_shift_extends_selection() {
        let mut t = ti("abc");
        t.move_right(true); // anchor=0, cursor=1
        assert_eq!(t.state.selection_anchor, Some(0));
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn move_left_with_shift_extends_selection_backward() {
        let mut t = ti_at_end("abc");
        t.move_left(true); // anchor=3, cursor=2
        assert_eq!(t.state.selection_anchor, Some(3));
        assert_eq!(t.cursor(), 2);
    }

    #[test]
    fn select_all_then_insert_replaces_entire_text() {
        let mut t = ti("old content");
        t.state.select_all();
        t.insert('X');
        assert_eq!(t.text(), "X");
    }

    // ── unicode ───────────────────────────────────────────────────────────────

    #[test]
    fn insert_multibyte_char() {
        let mut t = ti("");
        t.insert('é'); // 2 bytes in UTF-8
        assert_eq!(t.text(), "é");
        assert_eq!(t.cursor(), 'é'.len_utf8());
    }

    #[test]
    fn move_left_over_multibyte_char() {
        let mut t = ti_at_end("aé");
        t.move_left(false);
        // cursor should be after 'a', before 'é' (byte offset 1)
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn move_right_over_multibyte_char() {
        let mut t = ti("é!");
        // cursor starts at 0
        t.move_right(false);
        assert_eq!(t.cursor(), 'é'.len_utf8());
    }

    #[test]
    fn backspace_over_multibyte_char() {
        let mut t = ti_at_end("aé");
        t.backspace();
        assert_eq!(t.text(), "a");
        assert_eq!(t.cursor(), 1);
    }

    // ── scroll / ensure_cursor_visible ───────────────────────────────────────

    #[test]
    fn insert_updates_scroll_so_cursor_is_visible() {
        let mut t = TextInput::new(String::new());
        t.relayout(crate::ui::Rect { x: 0.0, y: 0.0, width: 50.0, height: 20.0 }, 1.0);
        t.set_char_width(8.0);
        // type 10 chars — cursor at x=80, visible_width ≈ 34px → scroll must kick in
        for ch in "abcdefghij".chars() { t.insert(ch); }
        // cursor should be visible: cursor_x >= scroll_offset
        let cursor_x = t.cursor() as f32 * 8.0;
        let scroll = t.scroll_offset();
        assert!(cursor_x >= scroll, "cursor_x={cursor_x} < scroll={scroll}");
        let visible = 50.0 - 8.0 * 2.0; // rect.width - padding*2
        assert!(cursor_x - scroll <= visible + 8.0,
            "cursor out of view: cursor_x={cursor_x} scroll={scroll} visible={visible}");
    }

    #[test]
    fn move_to_start_scrolls_back_to_zero() {
        let mut t = TextInput::new(String::new());
        t.relayout(crate::ui::Rect { x: 0.0, y: 0.0, width: 50.0, height: 20.0 }, 1.0);
        t.set_char_width(8.0);
        for ch in "abcdefghijklmnop".chars() { t.insert(ch); }
        t.move_to_start(false);
        assert_eq!(t.scroll_offset(), 0.0);
    }

    // ── empty input edge cases ────────────────────────────────────────────────

    #[test]
    fn empty_input_is_empty() {
        let t = ti("");
        assert!(t.is_empty());
        assert_eq!(t.text(), "");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn all_operations_on_empty_are_noop() {
        let mut t = ti("");
        t.backspace();
        t.move_left(false);
        t.move_right(false);
        t.move_word_left(false);
        t.move_word_right(false);
        assert_eq!(t.text(), "");
        assert_eq!(t.cursor(), 0);
    }

    // ── render ────────────────────────────────────────────────────────────────

    #[test]
    fn render_produces_node_with_text() {
        use crate::layout::Node;
        let mut t = ti("hello");
        t.relayout(crate::ui::Rect { x: 0.0, y: 0.0, width: 200.0, height: 30.0 }, 1.0);
        // render() should not panic and return a Layer node
        let node = t.render(true);
        assert!(matches!(node, Node::Layer(_)));
    }

    #[test]
    fn render_at_produces_node() {
        use crate::layout::Node;
        let t = ti("hi");
        let rect = crate::ui::Rect { x: 0.0, y: 0.0, width: 200.0, height: 30.0 };
        let node = t.render_at(rect, 1.0, "placeholder", false);
        assert!(matches!(node, Node::Layer(_)));
    }

    #[test]
    fn render_at_uses_placeholder_when_empty() {
        // Smoke test: no panic on empty text with placeholder
        use crate::layout::Node;
        let t = ti("");
        let rect = crate::ui::Rect { x: 0.0, y: 0.0, width: 200.0, height: 30.0 };
        let node = t.render_at(rect, 1.0, "Type here...", false);
        assert!(matches!(node, Node::Layer(_)));
    }
}
