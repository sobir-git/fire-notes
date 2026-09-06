//! Scroll tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, scroll_offset, total_lines};

fn overflowing_app() -> crate::logic::AppLogic {
    let mut logic = app();
    for _ in 0..100 {
        logic.execute(Action::InsertChar('\n'));
    }
    logic
}

#[test]
fn scrollbar_absent_when_content_fits() {
    let logic = app();
    let visible = logic.visible_line_count();
    assert!(total_lines(&logic) <= visible, "fresh app should fit in viewport");
}

#[test]
fn scrollbar_present_when_content_overflows() {
    let logic = overflowing_app();
    let visible = logic.visible_line_count();
    assert!(total_lines(&logic) > visible, "100-line doc should overflow viewport");
}

#[test]
fn scroll_offset_zero_at_doc_start() {
    let mut logic = overflowing_app();
    logic.execute(Action::CursorDocStart { selecting: false });
    assert_eq!(scroll_offset(&logic), 0);
}

#[test]
fn scroll_offset_increases_after_page_down() {
    let mut logic = overflowing_app();
    logic.execute(Action::PageDown { selecting: false });
    assert!(scroll_offset(&logic) > 0, "scroll_offset should move down after page-down");
}
