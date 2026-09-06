//! Unit tests for Tab.
use super::Tab;

#[test]
fn test_new_untitled() {
    let tab = Tab::new_untitled();
    assert!(tab.title().starts_with("Untitled-"));
    assert!(tab.content().is_empty());
}

#[test]
fn test_insert_and_content() {
    let mut tab = Tab::new_untitled();
    tab.insert_char('H');
    tab.insert_char('i');
    assert_eq!(tab.content(), "Hi");
}

#[test]
fn test_backspace() {
    let mut tab = Tab::new_untitled();
    tab.insert_char('A');
    tab.insert_char('B');
    tab.backspace();
    assert_eq!(tab.content(), "A");
}

#[test]
fn test_scroll_up_down() {
    let mut tab = Tab::new_untitled();
    for _ in 0..20 { tab.insert_char('\n'); }
    tab.scroll_down(5, 10);
    assert_eq!(tab.scroll_offset(), 5);
    tab.scroll_up(2);
    assert_eq!(tab.scroll_offset(), 3);
}

#[test]
fn test_select_all_then_copy() {
    let mut tab = Tab::new_untitled();
    tab.insert_char('h');
    tab.insert_char('i');
    tab.select_all();
    assert_eq!(tab.copy_selection(), Some("hi".to_string()));
}

#[test]
fn test_cut_selection_clears() {
    let mut tab = Tab::new_untitled();
    tab.insert_char('x');
    tab.insert_char('y');
    tab.select_all();
    let cut = tab.cut_selection();
    assert_eq!(cut, Some("xy".to_string()));
    assert_eq!(tab.content(), "");
}

#[test]
fn test_paste_text() {
    let mut tab = Tab::new_untitled();
    assert!(tab.paste_text("hello"));
    assert_eq!(tab.content(), "hello");
}

#[test]
fn test_word_wrap_toggle() {
    let mut tab = Tab::new_untitled();
    assert!(!tab.word_wrap());
    tab.toggle_word_wrap();
    assert!(tab.word_wrap());
}

#[test]
fn test_undo_redo() {
    let mut tab = Tab::new_untitled();
    tab.insert_char('a');
    tab.insert_char('b');
    tab.undo();
    assert_eq!(tab.content(), "a");
    tab.redo();
    assert_eq!(tab.content(), "ab");
}
